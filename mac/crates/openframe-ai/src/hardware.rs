//! Local hardware inspection (Local AI Runtime spec §5): memory and processor, shown in
//! Settings > Offline AI > Technical details and used for the "may be slow" hint. Collected
//! locally and never transmitted. Graphics adapters are read by the platform layer
//! (`openframe_platform::graphics`) and judged by `gpu::decide` (ADR-0029).

use serde::Serialize;

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
}

/// Inspect this computer.
pub fn inspect() -> HardwareInfo {
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
    let total = sys.total_memory();
    HardwareInfo {
        os: openframe_platform::current_os().name().to_string(),
        arch: std::env::consts::ARCH.to_string(),
        total_ram_bytes: total,
        available_ram_bytes: available_ram(sys.available_memory(), total),
        logical_cpus: logical,
        physical_cpus: System::physical_core_count(),
        cpu_brand,
    }
}

/// Memory a new process can take now. On the Mac, sysinfo subtracts the pages the compressor
/// occupies from free, inactive and purgeable pages, although those compressed pages are not part
/// of that sum; on a machine that has been in use it reads far less than macOS would hand back
/// without pressure. There the count comes from the kernel (`openframe_platform::memory`), never below
/// the sysinfo reading and never above the installed memory. Elsewhere the sysinfo reading stands.
fn available_ram(sysinfo_available: u64, total: u64) -> u64 {
    if let Some(bytes) = openframe_platform::memory::available_ram_bytes() {
        return bytes.max(sysinfo_available).min(total);
    }
    sysinfo_available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_reports_plausible_values() {
        let hw = inspect();
        assert!(hw.total_ram_bytes > 512 * 1024 * 1024, "RAM detected");
        assert!(hw.logical_cpus >= 1);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_mac_reading_is_at_least_the_sysinfo_one_and_at_most_the_installed_memory() {
        use sysinfo::{MemoryRefreshKind, RefreshKind, System};
        let kernel = openframe_platform::memory::available_ram_bytes()
            .expect("the kernel reports its memory statistics");
        let sys = System::new_with_specifics(
            RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let hw = inspect();
        eprintln!(
            "available memory: kernel {} MB, sysinfo {} MB, reported {} MB, installed {} MB",
            kernel >> 20,
            sys.available_memory() >> 20,
            hw.available_ram_bytes >> 20,
            hw.total_ram_bytes >> 20
        );
        assert!(kernel > 0);
        assert!(hw.available_ram_bytes <= hw.total_ram_bytes);
        assert!(
            available_ram(sys.available_memory(), hw.total_ram_bytes) >= sys.available_memory()
        );
        assert!(available_ram(u64::MAX, hw.total_ram_bytes) == hw.total_ram_bytes);
    }
}
