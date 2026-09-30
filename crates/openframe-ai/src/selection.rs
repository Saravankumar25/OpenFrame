//! Hardware-aware profile recommendation (Local AI Runtime spec §5).
//!
//! Produces: recommended profile, runtime backend, expected disk usage,
//! minimum free disk and a "likely slow" warning. Users may override with a
//! simple profile name; quantization details never reach the UI.

use serde::Serialize;

use crate::hardware::HardwareInfo;
use crate::manifest::{Backend, Manifest, ModelEntry, Tier};

const GIB: u64 = 1024 * 1024 * 1024;
/// Head-room kept free on the drive after installation.
pub const DISK_MARGIN_BYTES: u64 = 512 * 1024 * 1024;
/// Extracted runtime is roughly this many times the archive size.
const RUNTIME_EXTRACT_FACTOR: u64 = 3;
/// Minimum graphics memory for the Vulkan build to be worth using.
const MIN_USEFUL_VRAM: u64 = 3 * GIB;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFit {
    pub profile_id: String,
    pub tier: Tier,
    pub suitable: bool,
    pub likely_slow: bool,
    /// Plain-language explanation, e.g. "Needs at least 16 GB of memory."
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub profile_id: String,
    pub tier: Tier,
    pub backend: Backend,
    pub runtime_id: String,
    pub likely_slow: bool,
    pub warnings: Vec<String>,
    pub fits: Vec<ProfileFit>,
}

fn gb(bytes: u64) -> String {
    let v = bytes as f64 / GIB as f64;
    if v >= 10.0 {
        format!("{v:.0} GB")
    } else {
        format!("{v:.1} GB")
    }
}

/// Use the graphics card when a real adapter with enough memory and a Vulkan driver exist.
pub fn choose_backend(hw: &HardwareInfo) -> Backend {
    if hw.vulkan_available && hw.best_vram_bytes() >= MIN_USEFUL_VRAM {
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
        // Qwen-class models have ~28-36 layers; offload the share that fits.
        ((vram as f64 / model.gpu_vram_bytes as f64) * 32.0)
            .floor()
            .max(0.0) as i32
    }
}

pub fn assess(hw: &HardwareInfo, model: &ModelEntry, backend: Backend) -> ProfileFit {
    let gpu_full = backend == Backend::Vulkan && hw.best_vram_bytes() >= model.gpu_vram_bytes;
    let suitable = hw.total_ram_bytes >= model.min_ram_bytes || gpu_full;
    let likely_slow =
        !gpu_full && (hw.total_ram_bytes < model.recommended_ram_bytes || hw.logical_cpus < 4);
    let note = if !suitable {
        Some(format!(
            "Needs at least {} of memory. This computer has {}.",
            gb(model.min_ram_bytes),
            gb(hw.total_ram_bytes)
        ))
    } else if likely_slow {
        Some("Works on this computer, but answers may be slow.".to_string())
    } else {
        None
    };
    ProfileFit {
        profile_id: model.profile_id.clone(),
        tier: model.tier,
        suitable,
        likely_slow,
        note,
    }
}

/// Pick the best profile this machine runs comfortably; fall back to the
/// smallest profile (with a warning) on low-memory machines.
pub fn recommend(hw: &HardwareInfo, manifest: &Manifest) -> Option<Recommendation> {
    let backend_pref = choose_backend(hw);
    let (backend, runtime) = match manifest.runtime_for(backend_pref) {
        Some(r) => (backend_pref, r),
        None => (Backend::Cpu, manifest.runtime_for(Backend::Cpu)?),
    };
    let mut models: Vec<&ModelEntry> = manifest.models.iter().collect();
    models.sort_by_key(|m| m.tier.rank());
    let fits: Vec<ProfileFit> = models.iter().map(|m| assess(hw, m, backend)).collect();
    let comfortable = models
        .iter()
        .zip(&fits)
        .rev()
        .find(|(_, f)| f.suitable && !f.likely_slow)
        .map(|(m, _)| *m);
    let usable = models
        .iter()
        .zip(&fits)
        .rev()
        .find(|(_, f)| f.suitable)
        .map(|(m, _)| *m);
    let chosen = comfortable.or(usable).or_else(|| models.first().copied())?;
    let fit = fits
        .iter()
        .find(|f| f.profile_id == chosen.profile_id)
        .cloned()?;
    let mut warnings = Vec::new();
    if !fit.suitable {
        warnings.push(format!(
            "This computer has less memory than Offline AI normally needs ({}). It may be very slow or fail to start.",
            gb(hw.total_ram_bytes)
        ));
    } else if fit.likely_slow {
        warnings.push(
            "Offline AI will work on this computer, but answers may take a while.".to_string(),
        );
    }
    Some(Recommendation {
        profile_id: chosen.profile_id.clone(),
        tier: chosen.tier,
        backend,
        runtime_id: runtime.runtime_id.clone(),
        likely_slow: fit.likely_slow || !fit.suitable,
        warnings,
        fits,
    })
}

/// Bytes that must be free before downloading: remaining download + extracted
/// runtime + margin. `partial_bytes` already on disk are not needed again.
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
            gpus: if vram_gb > 0 {
                vec![GpuInfo {
                    name: "Test GPU".into(),
                    vendor_id: 0x10de,
                    dedicated_vram_bytes: vram_gb * GIB,
                    shared_memory_bytes: 0,
                    software: false,
                }]
            } else {
                vec![GpuInfo {
                    name: "Microsoft Basic Render Driver".into(),
                    vendor_id: 0x1414,
                    dedicated_vram_bytes: 0,
                    shared_memory_bytes: 0,
                    software: true,
                }]
            },
            vulkan_available: vram_gb > 0,
            free_disk_bytes: Some(100 * GIB),
        }
    }

    fn manifest() -> Manifest {
        crate::manifest::embedded().unwrap()
    }

    #[test]
    fn low_memory_laptop_gets_lightweight_on_cpu() {
        let r = recommend(&hw(6, 0, 4), &manifest()).unwrap();
        assert_eq!(r.tier, Tier::Lightweight);
        assert_eq!(r.backend, Backend::Cpu);
    }

    #[test]
    fn mainstream_laptop_gets_recommended() {
        let r = recommend(&hw(16, 0, 8), &manifest()).unwrap();
        assert_eq!(r.tier, Tier::Recommended);
        assert!(!r.likely_slow);
    }

    #[test]
    fn workstation_with_gpu_gets_high_quality_on_graphics_card() {
        let r = recommend(&hw(32, 8, 16), &manifest()).unwrap();
        assert_eq!(r.tier, Tier::HighQuality);
        assert_eq!(r.backend, Backend::Vulkan);
        let m = manifest();
        assert_eq!(
            gpu_layers(
                &hw(32, 8, 16),
                m.model_for_tier(Tier::HighQuality).unwrap(),
                Backend::Vulkan
            ),
            999
        );
    }

    #[test]
    fn tiny_machine_still_gets_a_profile_with_a_warning() {
        let r = recommend(&hw(3, 0, 2), &manifest()).unwrap();
        assert_eq!(r.tier, Tier::Lightweight);
        assert!(r.likely_slow);
        assert!(!r.warnings.is_empty());
    }

    #[test]
    fn disk_requirement_includes_margin() {
        assert_eq!(required_free_bytes(0, 0), DISK_MARGIN_BYTES);
        assert!(required_free_bytes(2 * GIB, 20_000_000) > 2 * GIB + DISK_MARGIN_BYTES);
    }
}
