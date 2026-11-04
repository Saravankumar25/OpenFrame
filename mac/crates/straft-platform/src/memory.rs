//! Available memory as the kernel reports it.
//!
//! macOS adapter: the kernel's memory statistics (`host_statistics64`), because the sysinfo reading
//! subtracts the pages the compressor occupies from pages that never included them. Other systems
//! return `None` and the caller keeps its own reading.

/// Memory a new process can take now without pressure, in bytes. `None` when the kernel does not
/// answer, and always `None` off macOS.
pub fn available_ram_bytes() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        mac::available_bytes()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

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

    #[test]
    fn the_kernel_answers_on_the_mac() {
        assert!(available_ram_bytes().expect("the kernel reports its memory statistics") > 0);
    }
}
