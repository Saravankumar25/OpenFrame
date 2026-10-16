//! Graphics adapters and the Vulkan loader (ADR-0029 decision 3; Windows phase W7).
//!
//! Windows reads the adapters through DXGI: name, vendor and device id, memory on the card, the
//! system memory it may borrow, the software flag and the user-mode driver version (the probe result
//! is remembered per adapter, driver and runtime). The Vulkan loader counts only when `vulkan-1.dll`
//! is in the System32 folder, which the graphics driver installs; no other folder is searched, so a
//! file planted next to OpenFrame or in the current folder never counts. Nothing here starts an engine.

/// One graphics adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsAdapter {
    pub name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub dedicated_mb: u64,
    pub shared_mb: u64,
    pub software: bool,
    /// The user-mode driver version as `a.b.c.d`, when the system reports it.
    pub driver_version: Option<String>,
}

/// The adapters of this computer, in the system's order. Empty when they cannot be read.
pub fn adapters() -> Vec<GraphicsAdapter> {
    #[cfg(windows)]
    {
        windows_impl::adapters()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Whether the system's Vulkan loader is installed.
pub fn vulkan_loader_present() -> bool {
    #[cfg(windows)]
    {
        windows_impl::system32().is_some_and(|d| d.join("vulkan-1.dll").is_file())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// `a.b.c.d` from the 64-bit driver version DXGI reports (four 16-bit parts, most significant first).
pub fn driver_version_text(v: u64) -> String {
    format!(
        "{}.{}.{}.{}",
        v >> 48,
        (v >> 32) & 0xFFFF,
        (v >> 16) & 0xFFFF,
        v & 0xFFFF
    )
}

#[cfg(windows)]
mod windows_impl {
    use super::{GraphicsAdapter, driver_version_text};
    use std::path::PathBuf;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIDevice, IDXGIFactory1,
    };
    use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
    use windows::core::Interface;

    pub fn adapters() -> Vec<GraphicsAdapter> {
        // SAFETY: plain COM calls on a factory this function owns; every returned interface is
        // released when it drops, and the description is a fixed-size array copied out at once.
        let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for i in 0..32u32 {
            let Ok(adapter) = (unsafe { factory.EnumAdapters1(i) }) else {
                break;
            };
            let Ok(d) = (unsafe { adapter.GetDesc1() }) else {
                continue;
            };
            let len = d
                .Description
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(d.Description.len());
            let driver = unsafe { adapter.CheckInterfaceSupport(&IDXGIDevice::IID) }
                .ok()
                .map(|v| driver_version_text(v as u64));
            out.push(GraphicsAdapter {
                name: String::from_utf16_lossy(&d.Description[..len]),
                vendor_id: d.VendorId,
                device_id: d.DeviceId,
                dedicated_mb: d.DedicatedVideoMemory as u64 / 1_048_576,
                shared_mb: d.SharedSystemMemory as u64 / 1_048_576,
                software: d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0,
                driver_version: driver,
            });
        }
        out
    }

    pub fn system32() -> Option<PathBuf> {
        let mut buf = [0u16; 260];
        // SAFETY: the buffer outlives the call and its length is passed with it.
        let n = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
        (n > 0 && n < buf.len()).then(|| PathBuf::from(String::from_utf16_lossy(&buf[..n])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_versions_read_as_four_parts() {
        let v: u64 = (31 << 48) | (0 << 32) | (101 << 16) | 5333;
        assert_eq!(driver_version_text(v), "31.0.101.5333");
    }

    #[test]
    fn this_computer_reports_its_adapters_without_failing() {
        let list = adapters();
        if cfg!(windows) {
            // Every Windows computer has at least the Basic Render Driver or a real adapter.
            assert!(!list.is_empty(), "no adapter was reported");
            assert!(list.iter().all(|a| !a.name.is_empty()));
        }
        let _ = vulkan_loader_present();
    }
}
