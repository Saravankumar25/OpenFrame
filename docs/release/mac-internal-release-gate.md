# Mac internal release gate

> Written 2026-10-18 at the end of the Mac M6 to M10 session, modelled on
> [windows-internal-release-gate.md](windows-internal-release-gate.md). The team receives the internal disk image only
> when every check below says **Pass** with saved evidence, or **Exception** with the owner's approval and date. A code
> change after the final build re-opens the build checks (G1 to G4). Evidence lives in `evidence/mac/`.

Status words: Open, Pass, Fail, Exception (owner approved), Exception (waiting for the owner).

The owner's word for this build: "build the next phases M6 to M10, properly test them end to end and then give me the
.dmg app", with the team password given in that conversation and full autonomy (2026-10-18). Building the internal disk
image is therefore authorised; signing, notarisation and the stores are Chapter 3 and out of scope.

## The build

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| G1 | One build | The disk image was built once from a recorded commit, with the team gate on; SHA-256, version, commit, date and size recorded | FINAL_G1 | FINAL_G1_EV |
| G2 | Tested as shipped | The end-to-end suite drove the app copied out of exactly that disk image | FINAL_G2 | FINAL_G2_EV |
| G3 | SBOM | CycloneDX for the Rust core and the AI engine files next to the image | FINAL_G3 | FINAL_G3_EV |
| G4 | Size | Image size recorded | FINAL_G4 | FINAL_G4_EV |

## Security

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| S1 | Mac security review | FFI boundary, team gate, the end-to-end driver, file paths and panels, the AI engine process, packaging, links and AI answers reviewed; every Critical and High finding fixed | Pass | [m9-security-review.md](evidence/mac/m9-security-review.md): no Critical; 2 High and 2 Medium fixed (commit d0bc17e) |
| S2 | Team gate | Wrong password refused, right password opens; the gate holds against environment variables, launch arguments, defaults, data files and the driver | FINAL_S2 | FINAL_S2_EV |
| S3 | Hardened runtime | The app is signed with the hardened runtime and only the microphone entitlement; no DYLD or library-validation exception | FINAL_S3 | FINAL_S3_EV |
| S4 | Secrets and personal paths | The image, the app and its files hold no key, token, plain team password, personal path or user name | FINAL_S4 | FINAL_S4_EV |
| S5 | AI engine | The pinned llama.cpp b11259 Metal build, every file matching the lock's SHA-256; listens on 127.0.0.1 only; ends with the app, also after a crash | FINAL_S5 | FINAL_S5_EV |
| S6 | Prompt injection | Poisoned-retrieval corpus, 60 cases in 12 languages, all safe on the Mac | Pass | [m8-ai-evaluation.md](evidence/mac/m8-ai-evaluation.md) |
| S7 | Store build readiness | Sandbox entitlements, privacy manifest and StoreKit 2 adapter in the repository; the store build itself is Chapter 3 | Pass | `apps/macos/packaging/`, commit e7d4eeb |

## Quality

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| Q1 | Unit and model tests | Swift suite and Rust workspace green on the final commit | FINAL_Q1 | FINAL_Q1_EV |
| Q2 | End-to-end on the shipped app | Every shared scenario implemented on the Mac runs on the app from the image; failures are fixed or recorded with their class | FINAL_Q2 | FINAL_Q2_EV |
| Q3 | Clean account | The same suite subset on an empty home folder with the app's real data locations | FINAL_Q3 | FINAL_Q3_EV |
| Q4 | AI bar | English: 90 percent correct tool, 85 percent correct end state, on Apple silicon with the default provider | Pass | 96.7 and 88.6 percent, [m8-ai-evaluation.md](evidence/mac/m8-ai-evaluation.md) |
| Q5 | Every screen as HTML | Every interface in Light and Dark as an HTML replica made from the final build | FINAL_Q5 | FINAL_Q5_EV |

## Records

| Id | Check | Pass means | Status | Evidence |
|---|---|---|---|---|
| R1 | Pilot kit | Mac install note with the Gatekeeper steps and the SHA-256 check | Pass | [mac-install.md](../testing/pilot-kit/mac-install.md) |
| R2 | Documents | Parity matrix, evidence, handoff and progress log current; links and writing checks green | FINAL_R2 | FINAL_R2_EV |

## Known limits of this build

FINAL_LIMITS
