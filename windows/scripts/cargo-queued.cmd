@echo off
rem Cargo runner for `tauri build --runner`: sends Tauri's cargo call through the machine-wide build
rem queue (scripts/cargo-queued.mjs), so app builds wait for other heavy builds and use 4 jobs.
node "%~dp0cargo-queued.mjs" %*
