//! Development tools and tests only (feature `test-support`, never in the app): install the models
//! the compiled lock pins from the local build inputs (`.ai-assets/models/<model-id>/<file>`) the
//! way a finished download leaves them in app data. Returns the Hugging Face model source; it is
//! never contacted because the set is already installed.

use std::path::Path;

use openframe_domain::{AppError, AppResult};

use crate::bundle::AssetLock;
use crate::download::pins;
use crate::fetch::ModelSource;
use crate::install::Active;
use crate::model_download::models_dir;

/// Install the lock's chat and embedding models from `assets_dir` into `app_data_dir/models`.
/// Files are hard-linked when the drive allows it (no second copy of the weights).
pub fn install_pinned_models(
    app_data_dir: &Path,
    assets_dir: &Path,
    lock: &AssetLock,
) -> AppResult<ModelSource> {
    let fail = |e: std::io::Error| AppError::internal(format!("installing test models: {e}"));
    let dir = models_dir(app_data_dir);
    let (version, tier) = pins(lock);
    let set = dir.set_dir(&version);
    std::fs::create_dir_all(&set).map_err(fail)?;
    for (m, f) in lock.models.iter().zip(&tier.files) {
        let src = assets_dir
            .join("models")
            .join(&m.model_id)
            .join(&m.file.path);
        let dst = set.join(&f.name);
        if !dst.exists() {
            std::fs::hard_link(&src, &dst)
                .or_else(|_| std::fs::copy(&src, &dst).map(|_| ()))
                .map_err(fail)?;
        }
    }
    let active = Active {
        version,
        tier: tier.id,
        previous: None,
        activated_at: 0,
    };
    std::fs::write(
        dir.root.join("active.json"),
        serde_json::to_vec(&active).map_err(|e| AppError::internal(e.to_string()))?,
    )
    .map_err(fail)?;
    Ok(ModelSource::huggingface())
}
