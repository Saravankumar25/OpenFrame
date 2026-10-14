//! Local hardware inspection (Local AI Runtime spec §5): memory and processor, shown in
//! Settings > Offline AI > Technical details and used for the "may be slow" hint. Collected
//! locally and never transmitted. The bundled engine is the processor build (ADR-0025), so the
//! graphics card does not change how Offline AI runs.

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
    HardwareInfo {
        os: openframe_platform::current_os().name().to_string(),
        arch: std::env::consts::ARCH.to_string(),
        total_ram_bytes: sys.total_memory(),
        available_ram_bytes: sys.available_memory(),
        logical_cpus: logical,
        physical_cpus: System::physical_core_count(),
        cpu_brand,
    }
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
}
