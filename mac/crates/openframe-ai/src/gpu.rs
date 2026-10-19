//! Which processor runs the chat engine (ADR-0029 decisions 2 to 5; Windows phase W7).
//!
//! A pure decision: it reads facts about the computer (graphics adapters, the Vulkan loader and
//! runtime, free memory), the person's setting and what earlier probes on this computer found, and
//! returns a launch plan. It never touches the system, so a table of imagined computers tests it
//! completely. Detection lives in the platform layer and the probe in the supervisor.

/// A graphics adapter as the system graphics interface reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adapter {
    pub name: String,
    pub vendor_id: u32,
    /// Memory on the card itself (zero or small for integrated graphics).
    pub dedicated_mb: u64,
    /// System memory the adapter may borrow.
    pub shared_mb: u64,
    /// A software renderer (Microsoft Basic Render Driver, WARP). Never used.
    pub software: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Dedicated,
    Integrated,
    Software,
}

/// A card counts as dedicated from this much memory of its own.
pub const DEDICATED_MIN_MB: u64 = 512;

impl Adapter {
    pub fn kind(&self) -> AdapterKind {
        if self.software {
            AdapterKind::Software
        } else if self.dedicated_mb >= DEDICATED_MIN_MB {
            AdapterKind::Dedicated
        } else {
            AdapterKind::Integrated
        }
    }
}

/// Settings, Offline AI, Graphics card (ADR-0029 decision 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Automatic,
    Off,
}

/// What an earlier probe found for one adapter, driver version and runtime version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Remembered {
    Passed,
    Failed,
}

pub struct Inputs<'a> {
    pub adapters: &'a [Adapter],
    /// `remembered[i]` is the probe result for `adapters[i]` with today's driver and runtime.
    pub remembered: &'a [Option<Remembered>],
    pub vulkan_loader: bool,
    pub vulkan_runtime: bool,
    /// Memory the chat model needs: weights plus context cache.
    pub model_mb: u64,
    /// Free system memory now.
    pub free_ram_mb: u64,
    pub setting: Setting,
    /// The hidden developer flag: probe integrated graphics even when the memory rule says no.
    pub force_integrated_probe: bool,
}

/// Room kept on top of the model for the engine's own buffers.
pub const MARGIN_MB: u64 = 1024;
/// System memory left for Windows and OpenFrame when integrated graphics borrow memory.
pub const SYSTEM_RESERVE_MB: u64 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuReason {
    SettingOff,
    NoVulkanLoader,
    NoVulkanRuntime,
    NoAdapter,
    OnlySoftware,
    NotEnoughMemory,
    ProbeFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Run on the processor.
    Cpu(CpuReason),
    /// First time for this adapter, driver and runtime: start on the card and run the probe.
    Probe { adapter: usize },
    /// A probe on this adapter passed before: use it.
    Gpu { adapter: usize },
}

pub fn decide(i: &Inputs<'_>) -> Plan {
    if i.setting == Setting::Off {
        return Plan::Cpu(CpuReason::SettingOff);
    }
    // The plainest true reason first: no card at all before a missing driver piece.
    if i.adapters.is_empty() {
        return Plan::Cpu(CpuReason::NoAdapter);
    }
    if i.adapters.iter().all(|a| a.kind() == AdapterKind::Software) {
        return Plan::Cpu(CpuReason::OnlySoftware);
    }
    if !i.vulkan_loader {
        return Plan::Cpu(CpuReason::NoVulkanLoader);
    }
    if !i.vulkan_runtime {
        return Plan::Cpu(CpuReason::NoVulkanRuntime);
    }
    let need = i.model_mb + MARGIN_MB;
    // Dedicated cards whose own memory holds the model, largest first; then integrated graphics
    // when the borrowed memory and the free system memory both hold it (or the developer flag).
    let mut dedicated: Vec<usize> = (0..i.adapters.len())
        .filter(|&k| {
            i.adapters[k].kind() == AdapterKind::Dedicated && i.adapters[k].dedicated_mb >= need
        })
        .collect();
    dedicated.sort_by_key(|&k| std::cmp::Reverse(i.adapters[k].dedicated_mb));
    let integrated = (0..i.adapters.len()).filter(|&k| {
        let a = &i.adapters[k];
        a.kind() == AdapterKind::Integrated
            && (i.force_integrated_probe
                || (a.shared_mb >= need && i.free_ram_mb >= need + SYSTEM_RESERVE_MB))
    });
    let candidates: Vec<usize> = dedicated.into_iter().chain(integrated).collect();
    if candidates.is_empty() {
        return Plan::Cpu(CpuReason::NotEnoughMemory);
    }
    for k in candidates {
        match i.remembered.get(k).copied().flatten() {
            Some(Remembered::Passed) => return Plan::Gpu { adapter: k },
            Some(Remembered::Failed) => continue,
            None => return Plan::Probe { adapter: k },
        }
    }
    Plan::Cpu(CpuReason::ProbeFailed)
}

impl CpuReason {
    /// One plain sentence for Settings and the health card.
    pub fn text(self) -> &'static str {
        match self {
            CpuReason::SettingOff => "The graphics card is switched off in Settings.",
            CpuReason::NoVulkanLoader => {
                "This computer's graphics driver does not offer what OpenFrame AI needs."
            }
            CpuReason::NoVulkanRuntime => {
                "This copy of OpenFrame Studio has no graphics card engine."
            }
            CpuReason::NoAdapter | CpuReason::OnlySoftware => "No graphics card was found.",
            CpuReason::NotEnoughMemory => {
                "The graphics card does not have enough memory for OpenFrame AI."
            }
            CpuReason::ProbeFailed => {
                "The graphics card did not pass its check, so the processor is used."
            }
        }
    }
}

/// Settings, Offline AI, Graphics card, and what the probes on this computer found
/// (`<app data>/ai/graphics.json`).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsRecord {
    /// `false` means Off (ADR-0029 decision 6). Automatic is the default.
    #[serde(default)]
    pub off: bool,
    /// The hidden developer flag (ADR-0029 decision 3): probe integrated graphics even when the
    /// memory rule says no. Set by hand in the file; never shown.
    #[serde(default)]
    pub force_integrated_probe: bool,
    #[serde(default)]
    pub probes: Vec<ProbeRecord>,
}

/// One probe: the adapter, driver and engine it ran with, and what it found.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecord {
    pub adapter: String,
    pub vendor_id: u32,
    pub driver: String,
    pub runtime: String,
    pub result: Remembered,
    pub gpu_tokens_per_second: Option<f64>,
    pub cpu_tokens_per_second: Option<f64>,
    pub at: i64,
}

impl GraphicsRecord {
    /// What an earlier probe found for this adapter with today's driver and engine.
    pub fn remembered(
        &self,
        adapter: &str,
        vendor: u32,
        driver: &str,
        runtime: &str,
    ) -> Option<Remembered> {
        self.probes
            .iter()
            .rev()
            .find(|p| {
                p.adapter == adapter
                    && p.vendor_id == vendor
                    && p.driver == driver
                    && p.runtime == runtime
            })
            .map(|p| p.result)
    }

    /// Keep one record per adapter, driver and engine (the newest), at most 16.
    pub fn remember(&mut self, r: ProbeRecord) {
        self.probes.retain(|p| {
            !(p.adapter == r.adapter
                && p.vendor_id == r.vendor_id
                && p.driver == r.driver
                && p.runtime == r.runtime)
        });
        self.probes.push(r);
        let extra = self.probes.len().saturating_sub(16);
        self.probes.drain(..extra);
    }
}

/// The engine's device ids by name, from `llama-server --list-devices`
/// (`  Vulkan0: Intel(R) Arc(TM) Graphics (9009 MiB, 8302 MiB free)`).
pub fn parse_devices(listing: &str) -> Vec<(String, String)> {
    listing
        .lines()
        .filter_map(|l| {
            let (id, rest) = l.trim().split_once(": ")?;
            let id_ok = id.starts_with("Vulkan")
                && id[6..].chars().all(|c| c.is_ascii_digit())
                && id.len() > 6;
            let name = rest.rsplit_once(" (").map_or(rest, |(n, _)| n).trim();
            (id_ok && !name.is_empty()).then(|| (id.to_string(), name.to_string()))
        })
        .collect()
}

/// The engine device for a Windows adapter: the one with the same name (ignoring case).
pub fn device_for(adapter: &str, devices: &[(String, String)]) -> Option<String> {
    let a = adapter.trim().to_ascii_lowercase();
    devices
        .iter()
        .find(|(_, n)| n.to_ascii_lowercase() == a)
        .map(|(id, _)| id.clone())
}

/// The Metal device of the Mac engine, from `llama-server --list-devices`
/// (`  MTL0: Apple M1 (12124 MiB, 12123 MiB free)`): its id and name. The Mac has one graphics
/// device that shares the memory of the processor (ADR-0033).
pub fn metal_device(listing: &str) -> Option<(String, String)> {
    listing.lines().find_map(|l| {
        let (id, rest) = l.trim().split_once(": ")?;
        let ok =
            id.starts_with("MTL") && id.len() > 3 && id[3..].chars().all(|c| c.is_ascii_digit());
        let name = rest.rsplit_once(" (").map_or(rest, |(n, _)| n).trim();
        (ok && !name.is_empty()).then(|| (id.to_string(), name.to_string()))
    })
}

/// A probe passes when the card gave valid constrained output and generated faster than the
/// processor (ADR-0029 decision 4). Without a processor figure the card is not trusted.
pub fn probe_passed(valid: bool, gpu_tps: Option<f64>, cpu_tps: Option<f64>) -> bool {
    match (valid, gpu_tps, cpu_tps) {
        (true, Some(g), Some(c)) => g.is_finite() && g > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mac_engine_lists_one_metal_device() {
        let listing = "Available devices:
  MTL0: Apple M1 (12124 MiB, 12123 MiB free)
  BLAS: Accelerate (0 MiB, 0 MiB free)
";
        assert_eq!(
            metal_device(listing),
            Some(("MTL0".to_string(), "Apple M1".to_string()))
        );
        assert_eq!(
            metal_device("Available devices:\n  BLAS: Accelerate (0 MiB)"),
            None
        );
        assert_eq!(
            metal_device("  Vulkan0: NVIDIA GeForce (8188 MiB, 7900 MiB free)"),
            None
        );
    }

    #[test]
    fn engine_devices_are_matched_to_windows_adapters_by_name() {
        let listing = "ggml_vulkan: Found 2 Vulkan devices:
Available devices:
  Vulkan0: NVIDIA GeForce RTX 4060 Laptop GPU (8188 MiB, 7900 MiB free)
  Vulkan1: Intel(R) Arc(TM) Graphics (9009 MiB, 8302 MiB free)
";
        let d = parse_devices(listing);
        assert_eq!(d.len(), 2);
        assert_eq!(
            device_for("Intel(R) Arc(TM) Graphics", &d).as_deref(),
            Some("Vulkan1")
        );
        assert_eq!(
            device_for("nvidia geforce rtx 4060 laptop gpu", &d).as_deref(),
            Some("Vulkan0")
        );
        assert_eq!(device_for("AMD Radeon 780M", &d), None);
        assert!(
            parse_devices(
                "Vulkan: x
VulkanX: y (1 MiB)
"
            )
            .is_empty()
        );
    }

    #[test]
    fn a_probe_passes_only_with_valid_output_faster_than_the_processor() {
        assert!(probe_passed(true, Some(30.0), Some(12.0)));
        assert!(
            !probe_passed(true, Some(10.0), Some(12.0)),
            "slower than the processor"
        );
        assert!(
            !probe_passed(false, Some(30.0), Some(12.0)),
            "invalid output"
        );
        assert!(!probe_passed(true, None, Some(12.0)));
        assert!(!probe_passed(true, Some(30.0), None), "no processor figure");
        assert!(!probe_passed(true, Some(f64::NAN), Some(1.0)));
    }

    #[test]
    fn probe_results_are_kept_per_adapter_driver_and_engine() {
        let rec = |driver: &str, result| ProbeRecord {
            adapter: "Intel(R) Arc(TM) Graphics".into(),
            vendor_id: INTEL,
            driver: driver.into(),
            runtime: "b11259".into(),
            result,
            gpu_tokens_per_second: None,
            cpu_tokens_per_second: None,
            at: 0,
        };
        let mut g = GraphicsRecord::default();
        g.remember(rec("32.0.101.8132", Remembered::Failed));
        g.remember(rec("32.0.101.8132", Remembered::Passed));
        assert_eq!(g.probes.len(), 1, "the newest result replaces the old one");
        let arc = "Intel(R) Arc(TM) Graphics";
        assert_eq!(
            g.remembered(arc, INTEL, "32.0.101.8132", "b11259"),
            Some(Remembered::Passed)
        );
        // A new driver or engine version means a new probe.
        assert_eq!(g.remembered(arc, INTEL, "32.0.101.9000", "b11259"), None);
        assert_eq!(g.remembered(arc, INTEL, "32.0.101.8132", "b12000"), None);
        for k in 0..40 {
            g.remember(rec(&format!("d{k}"), Remembered::Failed));
        }
        assert_eq!(g.probes.len(), 16, "bounded");
    }

    const INTEL: u32 = 0x8086;
    const NVIDIA: u32 = 0x10DE;
    const AMD: u32 = 0x1002;
    const MS: u32 = 0x1414;

    fn card(name: &str, vendor: u32, dedicated: u64, shared: u64) -> Adapter {
        Adapter {
            name: name.into(),
            vendor_id: vendor,
            dedicated_mb: dedicated,
            shared_mb: shared,
            software: false,
        }
    }
    fn soft() -> Adapter {
        Adapter {
            name: "Microsoft Basic Render Driver".into(),
            vendor_id: MS,
            dedicated_mb: 0,
            shared_mb: 4096,
            software: true,
        }
    }
    fn igpu(shared: u64) -> Adapter {
        card("Intel Arc integrated", INTEL, 128, shared)
    }

    struct Computer {
        name: &'static str,
        adapters: Vec<Adapter>,
        remembered: Vec<Option<Remembered>>,
        loader: bool,
        runtime: bool,
        model_mb: u64,
        free_ram_mb: u64,
        setting: Setting,
        force: bool,
        want: Plan,
    }

    fn pc(name: &'static str, adapters: Vec<Adapter>, want: Plan) -> Computer {
        let n = adapters.len();
        Computer {
            name,
            adapters,
            remembered: vec![None; n],
            loader: true,
            runtime: true,
            model_mb: 1400,
            free_ram_mb: 8000,
            setting: Setting::Automatic,
            force: false,
            want,
        }
    }

    #[test]
    fn the_table_of_imagined_computers() {
        use CpuReason::*;
        let table = vec![
            pc("no graphics adapter at all", vec![], Plan::Cpu(NoAdapter)),
            pc(
                "software renderer only (virtual machine)",
                vec![soft()],
                Plan::Cpu(OnlySoftware),
            ),
            pc(
                "two software renderers",
                vec![soft(), soft()],
                Plan::Cpu(OnlySoftware),
            ),
            Computer {
                setting: Setting::Off,
                ..pc(
                    "setting Off with a big card",
                    vec![card("RTX 4070", NVIDIA, 12288, 8000)],
                    Plan::Cpu(SettingOff),
                )
            },
            Computer {
                loader: false,
                ..pc(
                    "no Vulkan loader",
                    vec![card("RTX 3060", NVIDIA, 12288, 8000)],
                    Plan::Cpu(NoVulkanLoader),
                )
            },
            Computer {
                runtime: false,
                ..pc(
                    "Vulkan runtime missing from the package",
                    vec![card("RTX 3060", NVIDIA, 12288, 8000)],
                    Plan::Cpu(NoVulkanRuntime),
                )
            },
            Computer {
                loader: false,
                setting: Setting::Off,
                ..pc(
                    "setting Off wins over a missing loader",
                    vec![],
                    Plan::Cpu(SettingOff),
                )
            },
            pc(
                "big dedicated card, first run: probe it",
                vec![card("RTX 4070", NVIDIA, 12288, 8000)],
                Plan::Probe { adapter: 0 },
            ),
            Computer {
                remembered: vec![Some(Remembered::Passed)],
                ..pc(
                    "big dedicated card, probe passed before",
                    vec![card("RTX 4070", NVIDIA, 12288, 8000)],
                    Plan::Gpu { adapter: 0 },
                )
            },
            Computer {
                remembered: vec![Some(Remembered::Failed)],
                ..pc(
                    "big dedicated card, probe failed before",
                    vec![card("RTX 4070", NVIDIA, 12288, 8000)],
                    Plan::Cpu(ProbeFailed),
                )
            },
            pc(
                "small dedicated card below the model's need",
                vec![card("GT 1030", NVIDIA, 2048, 4000)],
                Plan::Cpu(NotEnoughMemory),
            ),
            pc(
                "dedicated card exactly at the need",
                vec![card("RX 6400", AMD, 2424, 4000)],
                Plan::Probe { adapter: 0 },
            ),
            pc(
                "dedicated card one MB short",
                vec![card("RX 6400", AMD, 2423, 4000)],
                Plan::Cpu(NotEnoughMemory),
            ),
            Computer {
                model_mb: 5000,
                ..pc(
                    "larger model tier on a 6 GB card",
                    vec![card("RTX 3050 6 GB", NVIDIA, 6144, 8000)],
                    Plan::Probe { adapter: 0 },
                )
            },
            Computer {
                model_mb: 5500,
                ..pc(
                    "larger model tier too big for a 6 GB card",
                    vec![card("RTX 3050 6 GB", NVIDIA, 6144, 8000)],
                    Plan::Cpu(NotEnoughMemory),
                )
            },
            pc(
                "integrated graphics with room, first run",
                vec![igpu(8000)],
                Plan::Probe { adapter: 0 },
            ),
            Computer {
                remembered: vec![Some(Remembered::Passed)],
                ..pc(
                    "integrated graphics, probe passed",
                    vec![igpu(8000)],
                    Plan::Gpu { adapter: 0 },
                )
            },
            Computer {
                remembered: vec![Some(Remembered::Failed)],
                ..pc(
                    "integrated graphics, probe failed",
                    vec![igpu(8000)],
                    Plan::Cpu(ProbeFailed),
                )
            },
            pc(
                "integrated graphics with too little shared memory",
                vec![igpu(1500)],
                Plan::Cpu(NotEnoughMemory),
            ),
            Computer {
                free_ram_mb: 3000,
                ..pc(
                    "integrated graphics on an 8 GB laptop with little free memory",
                    vec![igpu(4000)],
                    Plan::Cpu(NotEnoughMemory),
                )
            },
            Computer {
                free_ram_mb: 3000,
                force: true,
                ..pc(
                    "developer flag forces the probe on integrated graphics",
                    vec![igpu(4000)],
                    Plan::Probe { adapter: 0 },
                )
            },
            Computer {
                force: true,
                ..pc(
                    "developer flag never forces a software renderer",
                    vec![soft()],
                    Plan::Cpu(OnlySoftware),
                )
            },
            pc(
                "laptop with integrated and a dedicated card: the dedicated card first",
                vec![igpu(8000), card("RTX 4060 Laptop", NVIDIA, 8192, 8000)],
                Plan::Probe { adapter: 1 },
            ),
            Computer {
                remembered: vec![None, Some(Remembered::Failed)],
                ..pc(
                    "dedicated card failed, integrated still worth a probe",
                    vec![igpu(8000), card("RTX 4060 Laptop", NVIDIA, 8192, 8000)],
                    Plan::Probe { adapter: 0 },
                )
            },
            Computer {
                remembered: vec![Some(Remembered::Failed), Some(Remembered::Failed)],
                ..pc(
                    "both failed: processor",
                    vec![igpu(8000), card("RTX 4060 Laptop", NVIDIA, 8192, 8000)],
                    Plan::Cpu(ProbeFailed),
                )
            },
            Computer {
                remembered: vec![Some(Remembered::Passed), None],
                ..pc(
                    "integrated passed but the dedicated card is new: probe the card",
                    vec![igpu(8000), card("RTX 4060 Laptop", NVIDIA, 8192, 8000)],
                    Plan::Probe { adapter: 1 },
                )
            },
            pc(
                "two dedicated cards: the larger first",
                vec![
                    card("RTX 3060", NVIDIA, 12288, 8000),
                    card("RTX 4090", NVIDIA, 24576, 8000),
                ],
                Plan::Probe { adapter: 1 },
            ),
            Computer {
                remembered: vec![Some(Remembered::Passed), Some(Remembered::Failed)],
                ..pc(
                    "larger card failed, the smaller passed",
                    vec![
                        card("RTX 3060", NVIDIA, 12288, 8000),
                        card("RTX 4090", NVIDIA, 24576, 8000),
                    ],
                    Plan::Gpu { adapter: 0 },
                )
            },
            pc(
                "software renderer listed before a real card",
                vec![soft(), card("RX 7600", AMD, 8192, 8000)],
                Plan::Probe { adapter: 1 },
            ),
            pc(
                "AMD integrated graphics with room",
                vec![card("Radeon 780M", AMD, 512 - 1, 8000)],
                Plan::Probe { adapter: 0 },
            ),
            pc(
                "an adapter with 512 MB of its own counts as dedicated and is too small",
                vec![card("Radeon 512 MB", AMD, 512, 8000)],
                Plan::Cpu(NotEnoughMemory),
            ),
            pc(
                "small dedicated card and roomy integrated graphics: integrated",
                vec![card("GT 1030", NVIDIA, 2048, 4000), igpu(8000)],
                Plan::Probe { adapter: 1 },
            ),
            Computer {
                model_mb: 0,
                ..pc(
                    "no model needs memory (zero): still probes first",
                    vec![igpu(2000)],
                    Plan::Probe { adapter: 0 },
                )
            },
            Computer {
                remembered: vec![],
                ..pc(
                    "remembered list shorter than the adapter list",
                    vec![card("RTX 4070", NVIDIA, 12288, 8000)],
                    Plan::Probe { adapter: 0 },
                )
            },
        ];
        assert!(
            table.len() >= 30,
            "ADR-0029 asks for at least 30 computers, the table has {}",
            table.len()
        );
        for c in &table {
            let got = decide(&Inputs {
                adapters: &c.adapters,
                remembered: &c.remembered,
                vulkan_loader: c.loader,
                vulkan_runtime: c.runtime,
                model_mb: c.model_mb,
                free_ram_mb: c.free_ram_mb,
                setting: c.setting,
                force_integrated_probe: c.force,
            });
            assert_eq!(got, c.want, "{}", c.name);
        }
    }
}
