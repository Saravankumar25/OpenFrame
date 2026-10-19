// The team password for installed-build runs: env OPENFRAME_TEAM_PASSWORD, else the Windows User
// scope. Never printed or logged by callers.

import { execFileSync } from "node:child_process";

export function teamPassword() {
  let p = process.env.OPENFRAME_TEAM_PASSWORD;
  if (!p) {
    p = execFileSync("powershell.exe", ["-NoProfile", "-Command", "[Environment]::GetEnvironmentVariable('OPENFRAME_TEAM_PASSWORD','User')"], {
      encoding: "utf8",
      windowsHide: true,
    }).trim();
  }
  if (!p) throw new Error("No team password: set OPENFRAME_TEAM_PASSWORD (environment or Windows User scope).");
  return p;
}
