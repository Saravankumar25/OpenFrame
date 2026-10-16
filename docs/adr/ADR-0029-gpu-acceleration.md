# ADR-0029: GPU acceleration for the offline AI

- **Status:** Accepted 2026-10-15 (owner, in chat).
- **Date:** 2026-10-15
- **Amends:** [ADR-0025](ADR-0025-offline-ai-bundled-in-every-package.md) and the owner decision "AI engine build: CPU
  build only for the first release" (both already historical). The processor build stays as the fallback.
- **Related:** [ADR-0026](ADR-0026-offline-ai-download-after-install.md) (the runtime ships in the package),
  [ADR-0028](ADR-0028-ai-quality-program-and-harness.md), `docs/specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md`
  section 4.3 and 5, [Windows phase 7](../roadmap/chapter-1a-windows-phases.md).

## Context

The engine today is the `llama.cpp` processor build, started with no layers on the graphics card. On a processor a
multi-step answer takes 20 to 30 seconds on the development machine. The owner wants laptops with graphics cards to use
them, and wants the feature built so that it can be trusted before it has been run on every kind of card. The owner will
test on several laptops with different graphics cards.

## Decision

1. **Vulkan first.** The runtime in the package gains a Vulkan build of `llama.cpp` next to the processor build, both
   pinned by hash like today's runtime. One Vulkan build covers NVIDIA, AMD and Intel cards. A CUDA build is considered only
   if measurements show a clear gain and its redistribution terms are checked.
2. **A pure decision function** chooses the backend from the detected adapters, the Vulkan loader, the model's memory need
   and the user's setting. It is tested with a table of at least 30 imagined computers (no card, integrated only,
   small and large dedicated cards, mixed laptops, no driver, software adapter, low free memory, setting off).
3. **Detection.** Windows adapters are read through the system graphics interface (name, vendor, dedicated memory, software
   flag). Software adapters are never used. Dedicated cards are used when their memory covers the model plus a margin.
   Integrated graphics are used when the first-run probe passes (owner's choice; a hidden developer flag lets the owner
   force the probe on this laptop's integrated graphics for testing).
4. **Probe, then commit.** The first time a combination of adapter, driver version and runtime version is seen, the engine
   is started on the card and runs one fixed short request. It must return valid constrained output within a time limit and
   faster than the stored processor benchmark. A pass is remembered and the card is used. A failure, crash or timeout is
   remembered too, the processor is used, and the person sees one plain sentence. The probe runs again when the driver
   changes.
5. **Safety net while running.** If the engine on the card stops, it restarts on the processor.
6. **Setting.** Settings, Offline AI: Graphics card, Automatic or Off. The AI health card shows which is in use and the
   speed.
7. **Embeddings stay on the processor.** The embedding engine keeps all layers on the processor so the card's memory is
   left to the chat model.
8. **Claims rule.** The app and the website say a card is detected, checked and used, and that the processor is the fallback.
   A speed-up figure is published only for a named card that was measured. Until a card is measured it counts as
   "designed and unit tested", not "verified".
9. **Verification plan.** The decision table runs in the unit tests. The probe and fallback run on the owner's integrated
   graphics with the developer flag. Each borrowed laptop with another card gets the benchmark (processor against card:
   first-token time, prompt speed, generation speed, memory) and the result is stored as evidence with the card name and
   driver version.

## Consequences

- The package grows by the Vulkan runtime (estimate: tens of megabytes; measured in phase 7).
- The supervisor stops hard-coding no layers on the card for the chat engine; a launch plan comes from the decision
  function and is the only place the layer count is set.
- The Mac uses Metal through the same `llama.cpp` runtime and its own decision; Apple's model, when chosen, needs none.

## Implementation notes (2026-10-16, Windows phase W7)

1. **Where it lives.** `gpu.rs` (the pure decision, the probe record, the pass rule, matching an adapter to the engine
   device by name from `llama-server --list-devices`), `placement.rs` (facts from the platform layer, the probe request,
   plain sentences), `manager.rs` (decides once per session before the chat engine first starts, runs the probe, falls
   back), `supervisor.rs` (`-ngl 999 --device VulkanN` only when the plan names a card; the embedding engine stays at
   `-ngl 0`). Probe results, the setting and the developer flag are in `<app data>/ai/graphics.json`.
2. **The probe.** A fixed request with a JSON schema (temperature 0, at most 64 tokens) runs first on the processor and
   then on the card. The card passes when its answer is the schema's object and its generation speed, from the engine's
   own timings, is higher than the processor's. Without a processor figure the card is not trusted. The record keeps
   one result per adapter, driver and engine version, at most 16; a new driver or engine probes again. The first AI
   request after a new driver takes longer once (two model loads).
3. **Fallback in use.** When a request on the card fails because the engine stopped or timed out, the card is recorded as
   failed for that driver and engine, and the same request runs again on the processor.
4. **Order of reasons.** No card (or only a software renderer) is reported before a missing Vulkan loader, so a computer
   without a card is never told about its driver.
5. **Developer flag.** `"forceIntegratedProbe": true` in `graphics.json`, set by hand, probes integrated graphics even
   when the memory rule says no. It is never shown in the app.
6. **First measurement.** The owner's laptop, Intel Arc integrated graphics (driver 32.0.101.8132): the probe ran on the
   card and gave valid output at 30.3 tokens a second against 32.8 on the processor, so the processor is used
   ([evidence](../release/evidence/w7/graphics-card-2026-10-16.md)). Integrated graphics on this laptop are "checked and
   not faster"; no speed-up is claimed for any card yet (decision 8).
