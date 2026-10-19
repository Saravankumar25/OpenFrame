// The one way the interface shows a file from disk: the app's restricted `ofasset` protocol, which
// serves only the managed `assets/` folders of the open project and of the Global Idea Vault
// (`src-tauri/src/asset_scope.rs`). Tauri's own asset protocol is switched off in
// `tauri.conf.json`, so a URL built for it never loads (the Idea Vault, character pictures and
// call sheet attachments once used it and showed nothing).

import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "./client";
import type { AssetInfo } from "./generated/AssetInfo";

export const ASSET_SCHEME = "ofasset";

/** URL for a file path, or null outside the desktop app or when the path cannot be converted. */
export function fileSrc(path: string | null | undefined): string | null {
  if (!path || !inTauri()) return null;
  try {
    return convertFileSrc(path, ASSET_SCHEME);
  } catch {
    return null;
  }
}

/** URL for a managed asset that is on disk, or null. */
export function assetSrc(asset: AssetInfo | null | undefined): string | null {
  if (!asset || !asset.available) return null;
  return fileSrc(asset.path);
}
