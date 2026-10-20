// Close the app window the way a person does, from outside the web view: the window's close button
// (the system close command it sends, WM_SYSCOMMAND SC_CLOSE) or Alt+F4 pressed on the keyboard.
// WebDriver cannot do either. A PowerShell helper is started first and finds the app window; it then
// waits for one line on its input, so the close happens within milliseconds of the spec's go signal
// instead of after PowerShell's own start-up time.

import { execFileSync, spawn } from "node:child_process";

const SCRIPT = String.raw`
$ErrorActionPreference = 'Stop'
# Without this, PowerShell's progress records fill stderr and hide the real error.
$ProgressPreference = 'SilentlyContinue'
Add-Type -Namespace OfE2e -Name Close -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool PostMessage(System.IntPtr hWnd, int msg, System.IntPtr wParam, System.IntPtr lParam);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr hWnd);
[DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, int flags, System.UIntPtr extra);
// The close itself is compiled: after the go signal no PowerShell line is interpreted, so the
// close goes out within milliseconds (the interpreted loop took 114 ms on the installed build).
public static string Fire(System.IntPtr hWnd, bool altF4) {
  if (!altF4) {
    PostMessage(hWnd, 0x0112, (System.IntPtr)0xF060, System.IntPtr.Zero);
    return "sent";
  }
  // Keys go to whatever window is in front: never press Alt+F4 unless it is the app's window.
  // Windows may refuse a plain SetForegroundWindow (its foreground lock); a key press first is the
  // documented way to be allowed, so try a few times with one before giving up.
  for (int i = 0; i < 10 && GetForegroundWindow() != hWnd; i++) {
    if (i > 0) {
      keybd_event(0x12, 0, 0, System.UIntPtr.Zero);
      keybd_event(0x12, 0, 2, System.UIntPtr.Zero);
      System.Threading.Thread.Sleep(50);
    }
    SetForegroundWindow(hWnd);
  }
  if (GetForegroundWindow() != hWnd) return "not in front";
  keybd_event(0x12, 0, 0, System.UIntPtr.Zero);
  keybd_event(0x73, 0, 0, System.UIntPtr.Zero);
  keybd_event(0x73, 0, 2, System.UIntPtr.Zero);
  keybd_event(0x12, 0, 2, System.UIntPtr.Zero);
  return "sent";
}
'@
$deadline = (Get-Date).AddSeconds(20)
$hwnd = [System.IntPtr]::Zero
while ($hwnd -eq [System.IntPtr]::Zero -and (Get-Date) -lt $deadline) {
  $p = Get-Process | Where-Object { $_.Path -eq $env:E2E_APP -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
  if ($p) { $hwnd = $p.MainWindowHandle } else { Start-Sleep -Milliseconds 200 }
}
if ($hwnd -eq [System.IntPtr]::Zero) { throw 'The app window was not found.' }
$altF4 = $env:OF_CLOSE_METHOD -eq 'alt-f4'
if ($altF4) { [void][OfE2e.Close]::SetForegroundWindow($hwnd) }
[void][OfE2e.Close]::GetForegroundWindow()
[Console]::Out.WriteLine('ready')
[Console]::Out.Flush()
[void][Console]::In.ReadLine()
$result = [OfE2e.Close]::Fire($hwnd, $altF4)
if ($result -ne 'sent') { throw 'The app window is not in front; Alt+F4 was not sent.' }
[Console]::Out.WriteLine('sent')
[Console]::Out.Flush()
`;

/**
 * Start a helper that will close the app window by `method` ("button" or "alt-f4"). Resolves once
 * the window is found, with `go()`, which sends the close and resolves when it was sent.
 */
export function prepareClose(method) {
  const encoded = Buffer.from(SCRIPT, "utf16le").toString("base64");
  const p = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-EncodedCommand", encoded], {
    env: { ...process.env, OF_CLOSE_METHOD: method },
    stdio: ["pipe", "pipe", "pipe"],
  });
  let out = "";
  let err = "";
  const waitFor = (word) =>
    new Promise((resolve, reject) => {
      const check = () => out.includes(word) && resolve();
      p.stdout.on("data", (d) => {
        out += d;
        check();
      });
      p.stderr.on("data", (d) => (err += d));
      p.once("exit", (code) => (out.includes(word) ? resolve() : reject(new Error(`close helper ended (${code}): ${err.trim()}`))));
      check();
    });
  return waitFor("ready").then(() => ({
    go: () => {
      p.stdin.write("go\n");
      return waitFor("sent");
    },
  }));
}

/** True while a process runs from the app's executable. */
export function appRunning() {
  const out = execFileSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", "@(Get-Process | Where-Object { $_.Path -eq $env:E2E_APP }).Count"],
    { encoding: "utf8" },
  );
  return Number(out.trim()) > 0;
}
