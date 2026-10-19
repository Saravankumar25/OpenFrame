//! Where the chat engine runs (ADR-0029 decisions 3 to 6; Windows phase W7).
//!
//! The facts about this computer's graphics come from the platform layer, the choice from the pure
//! decision in `gpu.rs`, and the probe from the manager: a fixed short request with constrained
//! output, first on the processor (the figure to beat) and then on the card. What a probe found is
//! remembered per adapter, driver and engine version in `<app data>/ai/graphics.json`, together
//! with the person's setting (Automatic or Off) and the hidden developer flag.

use serde_json::json;

use crate::client::{ChatMessage, ChatRequest, Usage};
use crate::gpu::{self, Adapter, CpuReason};

/// What the launch plan needs to know about this computer's graphics.
#[derive(Debug, Clone, Default)]
pub struct GraphicsFacts {
    /// Each adapter with its driver version.
    pub adapters: Vec<(Adapter, String)>,
    pub vulkan_loader: bool,
}

/// The adapters and the Vulkan loader of this computer (platform layer).
pub fn detect() -> GraphicsFacts {
    GraphicsFacts {
        adapters: openframe_platform::graphics::adapters()
            .into_iter()
            .map(|a| {
                (
                    Adapter {
                        name: a.name,
                        vendor_id: a.vendor_id,
                        dedicated_mb: a.dedicated_mb,
                        shared_mb: a.shared_mb,
                        software: a.software,
                    },
                    a.driver_version.unwrap_or_default(),
                )
            })
            .collect(),
        vulkan_loader: openframe_platform::graphics::vulkan_loader_present(),
    }
}

/// Where the chat engine runs this session. Decided once, before its first start.
#[derive(Debug, Clone, PartialEq)]
pub enum Placement {
    Undecided,
    Processor(CpuReason),
    Card {
        adapter: String,
        vendor_id: u32,
        driver: String,
        /// The engine's device id (`Vulkan0`, ...).
        device: String,
        tokens_per_second: Option<f64>,
    },
}

/// The probe's version, part of each remembered result: a new way of measuring checks every card
/// again. Version 2 (2026-10-16): a realistic prompt and the whole request's time. Version 1 timed
/// only writing on a 20-token prompt and turned down this laptop's Intel Arc graphics, which read
/// the assistant's real prompts 4.4 times faster than the processor (evidence in
/// docs/release/evidence/w7/).
pub const PROBE_VERSION: &str = "2";

/// The engine part of a remembered result: the engine version and the probe version, so a new
/// engine or a new way of measuring checks the card again. Every record (pass, fail, failure in
/// use) is written and read with this key.
pub fn probe_key(runtime_version: &str) -> String {
    format!("{runtime_version} probe {PROBE_VERSION}")
}

/// The fixed probe request: about 2,000 tokens of context like the assistant's real prompts (most
/// of the time of a real request is spent reading the prompt), then a short answer under a JSON
/// schema, sampled without randomness.
pub fn probe_request() -> ChatRequest {
    let context: String = (1..=110)
        .map(|i| format!("Scene {i}. INT. LOCATION {i} - DAY. Two characters discuss plan {i} and the weather.
"))
        .collect();
    let mut r = ChatRequest::new(vec![
        ChatMessage::system(format!(
            "You reply with JSON only.
{context}"
        )),
        ChatMessage::user("Write the numbers from one to twenty in words, separated by spaces."),
    ])
    .with_schema(json!({
        "type": "object",
        "properties": { "text": { "type": "string" } },
        "required": ["text"],
        "additionalProperties": false
    }))
    .max_tokens(64);
    r.temperature = 0.0;
    r.timeout = std::time::Duration::from_secs(120);
    r
}

/// The probe's answer is valid when it is the schema's object with some text.
pub fn probe_valid(answer: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(answer)
        .ok()
        .and_then(|v| {
            v.get("text")
                .and_then(|t| t.as_str())
                .map(|t| !t.trim().is_empty())
        })
        .unwrap_or(false)
}

/// Tokens handled per second over the whole call (reading the prompt and writing the answer),
/// from the engine's own numbers.
pub fn tokens_per_second(u: &Usage) -> Option<f64> {
    let ms = u.prompt_ms + u.generation_ms;
    let tokens = u.prompt_tokens + u.completion_tokens;
    (ms > 0 && u.completion_tokens > 0).then(|| f64::from(tokens) * 1000.0 / f64::from(ms))
}

/// Memory the chat model needs on a card: its file plus room for the context cache.
pub fn model_mb(model_bytes: u64, context_tokens: u32) -> u64 {
    // About 32 KB of cache per token of context for the small models this app runs.
    model_bytes / (1024 * 1024) + u64::from(context_tokens) * 32 / 1024
}

/// The plain sentence for Settings and the health card.
pub fn describe(p: &Placement) -> String {
    match p {
        Placement::Undecided => "Decided before the first answer.".into(),
        Placement::Processor(r) => format!("Runs on the processor. {}", r.text()),
        Placement::Card {
            adapter,
            tokens_per_second,
            ..
        } => match tokens_per_second {
            // About three words for every four tokens.
            Some(t) => format!(
                "Runs on the graphics card ({adapter}), about {:.0} words a second.",
                t * 0.75
            ),
            None => format!("Runs on the graphics card ({adapter})."),
        },
    }
}

/// `gpu::decide` over the facts, the record and the model.
pub fn plan(
    facts: &GraphicsFacts,
    record: &gpu::GraphicsRecord,
    gpu_runtime: Option<&str>,
    model_mb: u64,
    free_ram_mb: u64,
) -> gpu::Plan {
    let adapters: Vec<Adapter> = facts.adapters.iter().map(|(a, _)| a.clone()).collect();
    let remembered: Vec<Option<gpu::Remembered>> = facts
        .adapters
        .iter()
        .map(|(a, driver)| {
            gpu_runtime.and_then(|rt| record.remembered(&a.name, a.vendor_id, driver, rt))
        })
        .collect();
    gpu::decide(&gpu::Inputs {
        adapters: &adapters,
        remembered: &remembered,
        vulkan_loader: facts.vulkan_loader,
        vulkan_runtime: gpu_runtime.is_some(),
        model_mb,
        free_ram_mb,
        setting: if record.off {
            gpu::Setting::Off
        } else {
            gpu::Setting::Automatic
        },
        force_integrated_probe: record.force_integrated_probe,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_probe_answer_must_be_the_schema_object_with_text() {
        assert!(probe_valid(r#"{"text":"one two three"}"#));
        assert!(!probe_valid(r#"{"text":"  "}"#));
        assert!(!probe_valid("one two three"));
        assert!(!probe_valid(r#"{"other":1}"#));
        let r = probe_request();
        assert_eq!(r.temperature, 0.0);
        assert!(r.json_schema.is_some());
    }

    #[test]
    fn speed_comes_from_the_engine_numbers() {
        let u = Usage {
            completion_tokens: 40,
            generation_ms: 2000,
            ..Usage::default()
        };
        assert_eq!(tokens_per_second(&u), Some(20.0));
        assert_eq!(tokens_per_second(&Usage::default()), None);
        assert_eq!(model_mb(1_069_306_368, 8192), 1019 + 256);
    }
}
