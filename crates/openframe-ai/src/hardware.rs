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
/// without pressure. There the count comes from the kernel (`mac::available_bytes`), never below
/// the sysinfo reading and never above the installed memory. Elsewhere the sysinfo reading stands.
fn available_ram(sysinfo_available: u64, total: u64) -> u64 {
    #[cfg(target_os = "macos")]
    if let Some(bytes) = mac::available_bytes() {
        return bytes.max(sysinfo_available).min(total);
    }
    let _ = total;
    sysinfo_available
}

#[cfg(target_os = "macos")]
mod mac {
    use std::sync::OnceLock;

    /// Page counts of the kernel's memory statistics (`host_statistics64`, `HOST_VM_INFO64`).
    #[derive(Debug, Clone, Copy)]
    pub(super) struct Pages {
        /// Free pages, the speculative ones included (`vm_stat` shows them apart).
        pub free: u64,
        pub inactive: u64,
        pub purgeable: u64,
    }

    /// Memory macOS hands back without pressure: free pages (speculative ones included), inactive
    /// pages and purgeable pages. Nothing is taken off for the compressor: its pages are counted
    /// apart and are not in this sum.
    pub(super) fn available(pages: Pages, page_size: u64) -> u64 {
        pages
            .free
            .saturating_add(pages.inactive)
            .saturating_add(pages.purgeable)
            .saturating_mul(page_size)
    }

    /// The host port, asked for once and kept for the life of the process (as sysinfo does).
    /// libc marks the call deprecated in favour of the `mach2` crate; sysinfo makes the same call,
    /// and one more crate for one function is not worth it.
    #[allow(deprecated)]
    fn host() -> libc::mach_port_t {
        static HOST: OnceLock<libc::mach_port_t> = OnceLock::new();
        // SAFETY: `mach_host_self` takes no arguments and returns a send right to the host port.
        *HOST.get_or_init(|| unsafe { libc::mach_host_self() })
    }

    pub(super) fn pages() -> Option<Pages> {
        let mut count = libc::HOST_VM_INFO64_COUNT;
        // SAFETY: an all-zero `vm_statistics64` is a valid value (plain integers), and `count`
        // tells the kernel how many 32-bit words it may write into it.
        let mut stat: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        // SAFETY: the pointer and count describe `stat`, which lives for the whole call.
        let status = unsafe {
            libc::host_statistics64(
                host(),
                libc::HOST_VM_INFO64,
                (&mut stat as *mut libc::vm_statistics64).cast(),
                &mut count,
            )
        };
        (status == libc::KERN_SUCCESS).then_some(Pages {
            free: u64::from(stat.free_count),
            inactive: u64::from(stat.inactive_count),
            purgeable: u64::from(stat.purgeable_count),
        })
    }

    fn page_size() -> Option<u64> {
        // SAFETY: `sysconf` only reads a system value.
        let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        u64::try_from(size).ok().filter(|s| *s > 0)
    }

    /// None when the kernel does not answer; the caller then keeps the sysinfo reading.
    pub(super) fn available_bytes() -> Option<u64> {
        Some(available(pages()?, page_size()?))
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

    #[cfg(target_os = "macos")]
    #[test]
    fn the_mac_counts_free_inactive_and_purgeable_pages_and_nothing_compressed() {
        // The page counts of `vm_stat` on the M1 used for the 2026-10-27 measurement: free 20,730
        // plus speculative 1,479 (the kernel's free count holds both), inactive 331,135,
        // purgeable 4,148, 189,215 pages in the compressor. sysinfo read 2.7 GB; macOS hands back
        // 5.9 GB.
        let pages = mac::Pages {
            free: 20_730 + 1_479,
            inactive: 331_135,
            purgeable: 4_148,
        };
        assert_eq!(mac::available(pages, 16_384), 357_492 * 16_384);
        let sysinfo_reading = (20_730 + 1_479 + 331_135 + 4_148 - 189_215) * 16_384;
        assert!(mac::available(pages, 16_384) > sysinfo_reading);
        assert_eq!(mac::available(pages, 0), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_mac_reading_is_at_least_the_sysinfo_one_and_at_most_the_installed_memory() {
        use sysinfo::{MemoryRefreshKind, RefreshKind, System};
        let kernel = mac::available_bytes().expect("the kernel reports its memory statistics");
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
