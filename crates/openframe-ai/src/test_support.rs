//! Development tools and tests only (feature `test-support`, never in the app): install the models
//! the compiled lock pins from the local build inputs (`.ai-assets/models/<model-id>/<file>`) the
//! way a finished download leaves them in app data, with a `set.json` signed by a throwaway key
//! that exists only in this process. The returned `ModelHost` carries that key so the manager
//! accepts the set; its address is never contacted.

use std::path::Path;

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use openframe_domain::{AppError, AppResult};

use crate::bundle::{AssetLock, ModelRole};
use crate::install::{Active, SET_FILE};
use crate::model_download::{ModelHost, PROFILE_VERSION, models_dir};

/// Install the lock's chat and embedding models from `assets_dir` into `app_data_dir/models`.
/// Files are hard-linked when the drive allows it (no second copy of the weights).
pub fn install_pinned_models(
    app_data_dir: &Path,
    assets_dir: &Path,
    lock: &AssetLock,
) -> AppResult<ModelHost> {
    let fail = |e: std::io::Error| AppError::internal(format!("installing test models: {e}"));
    let runtime = lock
        .runtime_for_this_computer()
        .ok_or_else(|| AppError::internal("no engine pinned for this computer"))?;
    let dir = models_dir(app_data_dir);
    let version = lock.bundle_version.clone();
    let set = dir.set_dir(&version);
    std::fs::create_dir_all(&set).map_err(fail)?;
    let mut files = Vec::new();
    for m in &lock.models {
        let src = assets_dir
            .join("models")
            .join(&m.model_id)
            .join(&m.file.path);
        let name = format!("{}.gguf", m.model_id);
        let dst = set.join(&name);
        if !dst.exists() {
            std::fs::hard_link(&src, &dst)
                .or_else(|_| std::fs::copy(&src, &dst).map(|_| ()))
                .map_err(fail)?;
        }
        let role = match m.role {
            ModelRole::Chat => "chat",
            ModelRole::Embedding => "embedding",
        };
        files.push(serde_json::json!({ "name": name, "path": format!("{version}/{name}"), "bytes": m.file.bytes, "sha256": m.file.sha256, "role": role }));
    }
    let payload = serde_json::to_vec(&serde_json::json!({
        "manifestVersion": version,
        "profileVersion": PROFILE_VERSION,
        "tiers": [{ "id": "small", "label": "Standard", "minMemoryGb": 8, "requiresRuntime": runtime.version, "files": files }],
        "runtimes": []
    }))
    .map_err(|e| AppError::internal(e.to_string()))?;
    let key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
    let b64 = base64::engine::general_purpose::STANDARD;
    let envelope = serde_json::json!({
        "format": crate::download::ENVELOPE_FORMAT,
        "keyId": "test-support",
        "payload": b64.encode(&payload),
        "signature": b64.encode(key.sign(&payload).to_bytes()),
    });
    std::fs::write(set.join(SET_FILE), envelope.to_string()).map_err(fail)?;
    let active = Active {
        version,
        tier: "small".into(),
        previous: None,
        activated_at: 0,
    };
    std::fs::write(
        dir.root.join("active.json"),
        serde_json::to_vec(&active).map_err(|e| AppError::internal(e.to_string()))?,
    )
    .map_err(fail)?;
    Ok(ModelHost {
        base: "https://models.invalid/".into(),
        keys: vec![key.verifying_key()],
    })
}
