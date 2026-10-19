// Keeps the document's appearance in step with the saved preference (app database, through
// `appearance.get`) and with the operating system while the mode is System. Mounted once, at the
// application root. Changing a theme or mode never reloads project data.

import { useCallback, useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useOp } from "../ipc/query";
import type { AppearanceState } from "../ipc/generated/AppearanceState";
import type { MyTheme } from "../ipc/generated/MyTheme";
import { applyAppearance, followSystem, type ThemeData } from "./engine";

/** Table name the Rust appearance module reports changes for. */
export const APPEARANCE_TABLE = "sys_appearance";

export function useAppearanceState() {
  return useOp<AppearanceState>("appearance.get", {}, [APPEARANCE_TABLE]);
}

/** Every appearance command returns the new state; show it at once (the change event also refreshes it). */
export function useStoreAppearance(): (s: AppearanceState) => void {
  const client = useQueryClient();
  return useCallback((s: AppearanceState) => client.setQueryData(["appearance.get", {}], s), [client]);
}

/** Usable themes from My Themes, in the engine's shape. */
export function myThemeData(mine: MyTheme[] | undefined): ThemeData[] {
  const out: ThemeData[] = [];
  for (const m of mine ?? []) {
    const t = m.theme;
    if (!t) continue;
    out.push({
      ...t,
      builtIn: false,
      light: Object.fromEntries(Object.entries(t.light).filter((e): e is [string, string] => typeof e[1] === "string")),
      dark: Object.fromEntries(Object.entries(t.dark).filter((e): e is [string, string] => typeof e[1] === "string")),
    });
  }
  return out;
}

export function useAppearance(): void {
  const state = useAppearanceState();
  const data = state.data;
  useEffect(() => followSystem(), []);
  useEffect(() => {
    if (data) applyAppearance(data.mode, data.themeId, myThemeData(data.myThemes));
  }, [data]);
}
