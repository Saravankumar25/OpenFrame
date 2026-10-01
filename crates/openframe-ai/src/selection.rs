//! Device check for the single Offline AI profile (Local AI Runtime spec §5).
//!
//! There is no model picker: this module only decides HOW the one profile runs on this computer
//! (graphics card or processor, how many layers to offload), how much free disk it needs, and
//! whether answers are likely to be slow. Quantization details never reach the UI.

use serde::Serialize;

use crate::hardware::HardwareInfo;
use crate::manifest::{Backend, ModelEntry};

const GIB: u64 = 1024 * 1024 * 1024;
/// Head-room kept free on the drive after installation.
pub const DISK_MARGIN_BYTES: u64 = 512 * 1024 * 1024;
/// Extracted runtime is roughly this many times the archive size.
const RUNTIME_EXTRACT_FACTOR: u64 = 3;
/// Graphics memory kept free for the desktop / other applications before offloading.
const VRAM_HEADROOM: u64 = 512 * 1024 * 1024;

/// How well this computer suits Offline AI.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fitness {
    /// Enough memory to run at all.
    pub suitable: bool,
    /// Works, but answers may be slow.
    pub likely_slow: bool,
    /// Plain-language warnings (no model or file names).
    pub warnings: Vec<String>,
}

fn gb(bytes: u64) -> String {
    let v = bytes as f64 / GIB as f64;
    if v >= 10.0 {
        format!("{v:.0} GB")
    } else {
        format!("{v:.1} GB")
    }
}

/// Use the graphics card only when a real (non-software) adapter has enough DEDICATED memory
/// for the whole chat model plus head-room and a Vulkan driver is installed. Integrated graphics
/// report little dedicated memory and share system RAM; on those the processor build is used
/// (see the benchmark in the Local AI Runtime spec §4.2). A graphics-card start failure falls
/// back to the processor build at install time.
pub fn choose_backend(hw: &HardwareInfo, chat: &ModelEntry) -> Backend {
    if hw.vulkan_available && hw.best_vram_bytes() >= chat.gpu_vram_bytes + VRAM_HEADROOM {
        Backend::Vulkan
    } else {
        Backend::Cpu
    }
}

/// Number of layers to offload (llama.cpp `-ngl`): all when the model fits in
/// graphics memory, a proportional share otherwise, none on CPU.
pub fn gpu_layers(hw: &HardwareInfo, model: &ModelEntry, backend: Backend) -> i32 {
    if backend == Backend::Cpu {
        return 0;
    }
    let vram = hw.best_vram_bytes();
    if vram >= model.gpu_vram_bytes {
        999
    } else {
        let layers = if model.layers > 0 { model.layers } else { 32 } as f64;
        ((vram as f64 / model.gpu_vram_bytes.max(1) as f64) * layers)
            .floor()
            .max(0.0) as i32
    }
}

/// Memory/processor assessment for the profile's chat model.
pub fn assess(hw: &HardwareInfo, chat: &ModelEntry, backend: Backend) -> Fitness {
    let gpu_full = backend == Backend::Vulkan && hw.best_vram_bytes() >= chat.gpu_vram_bytes;
    let suitable = hw.total_ram_bytes >= chat.min_ram_bytes || gpu_full;
    let likely_slow =
        !gpu_full && (hw.total_ram_bytes < chat.recommended_ram_bytes || hw.logical_cpus < 4);
    let mut warnings = Vec::new();
    if !suitable {
        warnings.push(format!(
            "This computer has less memory than Offline AI normally needs ({} of {}). It may be very slow or fail to start.",
            gb(hw.total_ram_bytes),
            gb(chat.min_ram_bytes)
        ));
    } else if likely_slow {
        warnings.push(
            "Offline AI will work on this computer, but answers may take a while.".to_string(),
        );
    }
    Fitness {
        suitable,
        likely_slow: likely_slow || !suitable,
        warnings,
    }
}

/// Bytes that must be free before downloading: remaining download + extracted
/// runtime + margin. Bytes of partial downloads already on disk are not needed again.
pub fn required_free_bytes(
    model_bytes_remaining: u64,
    runtime_archive_bytes_remaining: u64,
) -> u64 {
    model_bytes_remaining
        .saturating_add(runtime_archive_bytes_remaining.saturating_mul(RUNTIME_EXTRACT_FACTOR))
        .saturating_add(DISK_MARGIN_BYTES)
}

pub fn human_size(bytes: u64) -> String {
    if bytes >= GIB {
        gb(bytes)
    } else {
        format!("{} MB", (bytes as f64 / (1024.0 * 1024.0)).ceil() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::GpuInfo;

    fn hw(ram_gb: u64, vram_gb: u64, cpus: usize) -> HardwareInfo {
        HardwareInfo {
            os: "windows".into(),
            arch: "x86_64".into(),
            total_ram_bytes: ram_gb * GIB,
            available_ram_bytes: ram_gb * GIB / 2,
            logical_cpus: cpus,
            physical_cpus: Some(cpus / 2),
            cpu_brand: "Test CPU".into(),
            gpus: vec![GpuInfo {
                name: if vram_gb > 0 {
                    "Test GPU".into()
                } else {
                    "Integrated graphics".into()
                },
                vendor_id: 0x10de,
                dedicated_vram_bytes: if vram_gb > 0 {
                    vram_gb * GIB
                } else {
                    128 << 20
                },
                shared_memory_bytes: ram_gb * GIB / 2,
                software: false,
            }],
            vulkan_available: true,
            free_disk_bytes: Some(100 * GIB),
        }
    }

    fn chat() -> ModelEntry {
        crate::manifest::embedded()
            .unwrap()
            .chat_model()
            .unwrap()
            .clone()
    }

    #[test]
    fn integrated_graphics_laptop_runs_on_the_processor() {
        let h = hw(16, 0, 8);
        assert_eq!(choose_backend(&h, &chat()), Backend::Cpu);
        let f = assess(&h, &chat(), Backend::Cpu);
        assert!(f.suitable && !f.likely_slow && f.warnings.is_empty());
    }

    #[test]
    fn discrete_graphics_card_is_used_with_all_layers() {
        let h = hw(16, 6, 12);
        assert_eq!(choose_backend(&h, &chat()), Backend::Vulkan);
        assert_eq!(gpu_layers(&h, &chat(), Backend::Vulkan), 999);
        assert_eq!(gpu_layers(&h, &chat(), Backend::Cpu), 0);
    }

    #[test]
    fn no_vulkan_driver_means_processor() {
        let mut h = hw(16, 6, 12);
        h.vulkan_available = false;
        assert_eq!(choose_backend(&h, &chat()), Backend::Cpu);
    }

    #[test]
    fn partial_offload_is_proportional_to_graphics_memory() {
        let mut m = chat();
        m.gpu_vram_bytes = 4 * GIB;
        m.layers = 26;
        assert_eq!(gpu_layers(&hw(16, 2, 8), &m, Backend::Vulkan), 13);
    }

    #[test]
    fn tiny_machine_is_warned_but_not_refused() {
        let f = assess(&hw(2, 0, 2), &chat(), Backend::Cpu);
        assert!(!f.suitable && f.likely_slow);
        assert!(f.warnings[0].contains("less memory"));
        for w in &f.warnings {
            assert!(!w.to_lowercase().contains("gemma") && !w.contains("GGUF"));
        }
    }

    #[test]
    fn disk_requirement_includes_margin() {
        assert_eq!(required_free_bytes(0, 0), DISK_MARGIN_BYTES);
        assert!(required_free_bytes(2 * GIB, 20_000_000) > 2 * GIB + DISK_MARGIN_BYTES);
    }
}
