//! How well this computer suits the bundled Offline AI (Local AI Runtime spec §5). There is no
//! model picker and, since ADR-0025, no choice of engine build: this module only says whether
//! answers are likely to be slow, in plain language without model or file names.

use serde::Serialize;

use crate::bundle::ModelEntry;
use crate::hardware::HardwareInfo;

const GIB: u64 = 1024 * 1024 * 1024;

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

/// Memory and processor assessment for the chat model.
pub fn assess(hw: &HardwareInfo, chat: &ModelEntry) -> Fitness {
    let suitable = hw.total_ram_bytes >= chat.min_ram_bytes;
    let likely_slow = hw.total_ram_bytes < chat.recommended_ram_bytes || hw.logical_cpus < 4;
    let mut warnings = Vec::new();
    if !suitable {
        warnings.push(format!(
            "This computer has less memory than Offline AI normally needs ({} of {}). It may be very slow or fail to start.",
            gb(hw.total_ram_bytes),
            gb(chat.min_ram_bytes)
        ));
    } else if likely_slow {
        warnings
            .push("Offline AI works on this computer, but answers may take a while.".to_string());
    }
    Fitness {
        suitable,
        likely_slow: likely_slow || !suitable,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(ram_gb: u64, cpus: usize) -> HardwareInfo {
        HardwareInfo {
            os: "windows".into(),
            arch: "x86_64".into(),
            total_ram_bytes: ram_gb * GIB,
            available_ram_bytes: ram_gb * GIB / 2,
            logical_cpus: cpus,
            physical_cpus: Some(cpus / 2),
            cpu_brand: "Test CPU".into(),
        }
    }

    fn chat() -> ModelEntry {
        crate::bundle::AssetLock::embedded()
            .unwrap()
            .chat_model()
            .clone()
    }

    #[test]
    fn a_normal_laptop_is_fine() {
        let f = assess(&hw(16, 8), &chat());
        assert!(f.suitable && !f.likely_slow && f.warnings.is_empty());
    }

    #[test]
    fn tiny_machine_is_warned_but_not_refused() {
        let f = assess(&hw(2, 2), &chat());
        assert!(!f.suitable && f.likely_slow);
        assert!(f.warnings[0].contains("less memory"));
        for w in &f.warnings {
            assert!(!w.to_lowercase().contains("gemma") && !w.contains("GGUF"));
        }
    }
}
