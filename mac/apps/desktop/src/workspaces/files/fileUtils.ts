// Shared Project Files helpers (Files workspace and the Project Home files card).

import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { call, openAsset, revealLocation } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ProjectFileDto } from "../../ipc/generated/ProjectFileDto";
import { toast } from "../../app/toast";

/** Tables `files.list` reads (drives query invalidation). */
export const FILE_TABLES = ["project_file", "project_file_folder", "asset"];

/** Badge class + short text for the `.fic` file icon (mock 012 / 154). */
export function fileBadge(mediaType: string, name: string): { cls: string; text: string } {
  const ext = (name.split(".").pop() ?? "").toUpperCase().slice(0, 4);
  if (mediaType === "application/pdf") return { cls: "pdf", text: "PDF" };
  if (mediaType.includes("word") || mediaType === "application/rtf" || mediaType === "application/msword") return { cls: "doc", text: "DOC" };
  if (mediaType.startsWith("audio/")) return { cls: "aud", text: "AUD" };
  if (mediaType.startsWith("video/")) return { cls: "vid", text: "VID" };
  if (mediaType.startsWith("image/")) return { cls: "", text: "IMG" };
  return { cls: "", text: ext || "FILE" };
}

/** Human type column (mock 154: PDF, Document, Spreadsheet, Artwork…). */
export function fileKind(mediaType: string, name: string): string {
  const lower = name.toLowerCase();
  if (mediaType === "application/pdf") return "PDF";
  if (mediaType.includes("spreadsheet") || mediaType === "application/vnd.ms-excel" || lower.endsWith(".csv")) return "Spreadsheet";
  if (mediaType.includes("presentation") || mediaType === "application/vnd.ms-powerpoint") return "Presentation";
  if (mediaType.includes("word") || mediaType === "application/rtf" || mediaType === "application/msword") return "Document";
  if (mediaType.startsWith("text/")) return lower.endsWith(".fountain") || lower.endsWith(".fdx") ? "Screenplay" : "Text";
  if (mediaType === "application/xml" && lower.endsWith(".fdx")) return "Screenplay";
  if (mediaType.startsWith("image/")) return "Image";
  if (lower.endsWith(".psd") || lower.endsWith(".ai")) return "Artwork";
  if (mediaType.startsWith("audio/")) return "Audio";
  if (mediaType.startsWith("video/")) return "Video";
  return "File";
}

export type WhereState = "stored" | "linked" | "unavailable";
export function whereOf(f: ProjectFileDto): WhereState {
  if (!f.asset.available) return "unavailable";
  return f.asset.storageMode === "external" ? "linked" : "stored";
}

/** Folder part of an absolute path, for "Linked (external) D:\Art\". */
export function folderOf(path: string | null | undefined): string {
  if (!path) return "";
  const i = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return i > 0 ? path.slice(0, i + 1) : path;
}

/** Add files to the project (copy into the project, or link to where they are). */
export async function addFiles(paths: string[], mode: "copy" | "link", folderId: string | null = null): Promise<ProjectFileDto[] | null> {
  if (paths.length === 0) return null;
  try {
    const added = await call<ProjectFileDto[]>("files.add", { paths, mode, folderId });
    toast.undoable(added.length === 1 ? `Added “${added[0].displayName}”` : `Added ${added.length} files`);
    return added;
  } catch (e) {
    reportError(e);
    return null;
  }
}

// ------------------------------------------------------------ file actions

/** Open externally with the default Windows app (the path never comes from the UI). */
export async function openFile(f: ProjectFileDto): Promise<void> {
  try {
    await openAsset(f.asset.id);
  } catch (e) {
    reportError(e);
  }
}

export async function revealFile(f: ProjectFileDto): Promise<void> {
  try {
    await openAsset(f.asset.id, { reveal: true });
  } catch (e) {
    reportError(e);
  }
}

/** "Export/copy": save a copy somewhere the user chooses. The project is unchanged. */
export async function exportCopy(f: ProjectFileDto): Promise<void> {
  const dest = await saveDialog({ title: `Save a copy of “${f.displayName}”`, defaultPath: f.asset.originalName });
  if (!dest) return;
  try {
    const written = await call<string>("files.export_copy", { id: f.id, destPath: dest });
    toast.info(`Saved a copy of “${f.displayName}”.`, {
      label: "Show",
      run: () => void revealLocation("exported", undefined, written).catch(reportError),
    });
  } catch (e) {
    reportError(e);
  }
}

/** Point a linked file at its new location; the file keeps its identity, notes and folder. */
export async function relinkFile(f: ProjectFileDto): Promise<void> {
  const picked = await openDialog({ multiple: false, directory: false, title: `Locate “${f.asset.originalName}”` });
  if (typeof picked !== "string") return;
  try {
    await call("files.relink", { id: f.id, path: picked });
    toast.undoable(`Relinked “${f.displayName}”`);
  } catch (e) {
    reportError(e);
  }
}

export async function deleteFile(f: ProjectFileDto): Promise<boolean> {
  try {
    await call("files.delete", { id: f.id });
    toast.undoable(`Moved “${f.displayName}” to Recently Deleted`);
    return true;
  } catch (e) {
    reportError(e);
    return false;
  }
}

export async function moveFile(f: ProjectFileDto, folderId: string | null, folderName: string): Promise<void> {
  if ((f.folderId ?? null) === folderId) return;
  try {
    await call("files.move", { id: f.id, folderId });
    toast.undoable(`Moved “${f.displayName}” to ${folderName}`);
  } catch (e) {
    reportError(e);
  }
}

/** Pick files with the system dialog and add them. */
export async function pickAndAddFiles(mode: "copy" | "link", folderId: string | null = null): Promise<ProjectFileDto[] | null> {
  const picked = await openDialog({ multiple: true, directory: false, title: mode === "copy" ? "Add files to this project" : "Link files to this project" });
  const paths = Array.isArray(picked) ? picked : typeof picked === "string" ? [picked] : [];
  return addFiles(paths, mode, folderId);
}
