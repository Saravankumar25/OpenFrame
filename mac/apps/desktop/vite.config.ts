/// <reference types="vitest/config" />
import { availableParallelism } from "node:os";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri expects a fixed dev port and must not clear the terminal.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    sourcemap: false,
    chunkSizeWarningLimit: 900,
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: false,
    // jsdom suites are memory- and CPU-heavy. One worker per logical core exhausted a 16 GB
    // machine, and even half of 18 cores let whole-tab tests time out at random under contention
    // (2026-10-05, build plan item baseline run). At most 4 workers was stable across repeated runs (about
    // 14 s) and also suits 2- and 4-core CI runners.
    maxWorkers: Math.max(1, Math.min(4, Math.floor(availableParallelism() / 2))),
    testTimeout: 15_000,
  },
});
