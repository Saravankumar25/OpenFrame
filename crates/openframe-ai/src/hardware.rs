//! Local hardware inspection (Local AI Runtime spec §5).
//!
//! Everything is collected locally and never transmitted. GPU adapters and
//! their dedicated memory come from DXGI (reliable above 4 GB, unlike WMI).

use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor_id: u32,
    pub dedicated_vram_bytes: u64,
    pub shared_memory_bytes: u64,
    /// Microsoft Basic Render Driver / WARP — not useful for acceleration.
    pub software: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub os: String,
    pub arch: String,
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub logical_cpus: usize,
    pub physical_cpus: Option<usize>,
    pub cpu_brand: String,
    pub gpus: Vec<GpuInfo>,
    /// A Vulkan loader is installed (the graphics driver ships one).
    pub vulkan_available: bool,
    /// Free space on the drive holding the model store.
    pub free_disk_bytes: Option<u64>,
}

impl HardwareInfo {
    /// The hardware (non-software) adapter with the most dedicated memory.
    pub fn best_gpu(&self) -> Option<&GpuInfo> {
        self.gpus
            .iter()
            .filter(|g| !g.software)
            .max_by_key(|g| g.dedicated_vram_bytes)
    }
    pub fn best_vram_bytes(&self) -> u64 {
        self.best_gpu().map(|g| g.dedicated_vram_bytes).unwrap_or(0)
    }
}

/// Inspect this computer. `store_dir` is the model store (its drive's free space is reported).
pub fn inspect(store_dir: &Path) -> HardwareInfo {
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::nothing().with_ram())
            .with_cpu(CpuRefreshKind::nothing()),
    );
    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(sys.cpus().len().max(1));
    HardwareInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        total_ram_bytes: sys.total_memory(),
        available_ram_bytes: sys.available_memory(),
        logical_cpus: logical,
        physical_cpus: System::physical_core_count(),
        cpu_brand,
        gpus: gpus(),
        vulkan_available: vulkan_available(),
        free_disk_bytes: free_space(store_dir),
    }
}

/// Free space for `dir` (or its nearest existing ancestor).
pub fn free_space(dir: &Path) -> Option<u64> {
    let mut p = Some(dir);
    while let Some(cur) = p {
        if cur.exists() {
            return fs4::available_space(cur).ok();
        }
        p = cur.parent();
    }
    None
}

#[cfg(windows)]
fn vulkan_available() -> bool {
    let sys_root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    Path::new(&sys_root)
        .join("System32")
        .join("vulkan-1.dll")
        .is_file()
}

#[cfg(not(windows))]
fn vulkan_available() -> bool {
    false
}

#[cfg(windows)]
fn gpus() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
    };
    let mut out = Vec::new();
    // SAFETY: plain COM calls on a factory we own; failures are handled by returning what we have.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return out;
        };
        let mut i = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(desc) = adapter.GetDesc1() else {
                continue;
            };
            let len = desc
                .Description
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(desc.Description.len());
            let name = String::from_utf16_lossy(&desc.Description[..len])
                .trim()
                .to_string();
            let software =
                (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 || desc.VendorId == 0x1414;
            let info = GpuInfo {
                name,
                vendor_id: desc.VendorId,
                dedicated_vram_bytes: desc.DedicatedVideoMemory as u64,
                shared_memory_bytes: desc.SharedSystemMemory as u64,
                software,
            };
            // The same physical adapter can be listed more than once.
            if !out.iter().any(|g: &GpuInfo| {
                g.name == info.name && g.dedicated_vram_bytes == info.dedicated_vram_bytes
            }) {
                out.push(info);
            }
            if i > 16 {
                break;
            }
        }
    }
    out
}

#[cfg(not(windows))]
fn gpus() -> Vec<GpuInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_reports_plausible_values() {
        let dir = tempfile::tempdir().unwrap();
        let hw = inspect(dir.path());
        assert!(hw.total_ram_bytes > 512 * 1024 * 1024, "RAM detected");
        assert!(hw.logical_cpus >= 1);
        assert!(hw.free_disk_bytes.is_some());
        // Nonexistent subfolder resolves to the nearest existing ancestor.
        assert!(free_space(&dir.path().join("a").join("b")).is_some());
    }
}
