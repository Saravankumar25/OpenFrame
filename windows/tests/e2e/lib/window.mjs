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
public delegate bool EnumProc(System.IntPtr hWnd, System.IntPtr lParam);
[DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, System.IntPtr lParam);
[DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(System.IntPtr hWnd, out uint pid);
[DllImport("user32.dll")] static extern bool IsWindowVisible(System.IntPtr hWnd);
[DllImport("user32.dll")] static extern int GetWindowTextLengthW(System.IntPtr hWnd);
[DllImport("user32.dll")] static extern int GetWindowLongW(System.IntPtr hWnd, int index);
// Process.MainWindowHandle can answer the event loop's hidden message window, whichever comes first
// in the window order. A close sent there kills event delivery and leaves the app running. The
// app's window is the visible one with a title that is not a tool window.
public static System.IntPtr AppWindow(int processId) {
  System.IntPtr found = System.IntPtr.Zero;
  EnumWindows((h, _) => {
    uint pid;
    GetWindowThreadProcessId(h, out pid);
    if (pid == (uint)processId && IsWindowVisible(h) && GetWindowTextLengthW(h) > 0 && (GetWindowLongW(h, -20) & 0x80) == 0) {
      found = h;
      return false;
    }
    return true;
  }, System.IntPtr.Zero);
  return found;
}
// The close itself is compiled: after the go signal no PowerShell line is interpreted, so the
// close goes out within milliseconds (the interpreted loop took 114 ms on the installed build).
public static string Fire(System.IntPtr hWnd, bool altF4) {
  if (!altF4) {
    PostMessage(hWnd, 0x0112, (System.IntPtr)0xF060, System.IntPtr.Zero);
    return "sent " + System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
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
  return "sent " + System.DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
}
'@
$deadline = (Get-Date).AddSeconds(20)
$hwnd = [System.IntPtr]::Zero
while ($hwnd -eq [System.IntPtr]::Zero -and (Get-Date) -lt $deadline) {
  $p = Get-Process | Where-Object { $_.Path -eq $env:E2E_APP } | Select-Object -First 1
  if ($p) { $hwnd = [OfE2e.Close]::AppWindow($p.Id) }
  if ($hwnd -eq [System.IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
}
if ($hwnd -eq [System.IntPtr]::Zero) { throw 'The app window was not found.' }
$altF4 = $env:OF_CLOSE_METHOD -eq 'alt-f4'
if ($altF4) { [void][OfE2e.Close]::SetForegroundWindow($hwnd) }
[void][OfE2e.Close]::GetForegroundWindow()
[Console]::Out.WriteLine('ready')
[Console]::Out.Flush()
[void][Console]::In.ReadLine()
$result = [OfE2e.Close]::Fire($hwnd, $altF4)
if (-not $result.StartsWith('sent')) { throw 'The app window is not in front; Alt+F4 was not sent.' }
[Console]::Out.WriteLine($result)
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
      const has = () => (word instanceof RegExp ? word.test(out) : out.includes(word));
      const check = () => has() && resolve();
      p.stdout.on("data", (d) => {
        out += d;
        check();
      });
      p.stderr.on("data", (d) => (err += d));
      p.once("exit", (code) => (has() ? resolve() : reject(new Error(`close helper ended (${code}): ${err.trim()}`))));
      check();
    });
  return waitFor("ready").then(() => ({
    go: () => {
      p.stdin.write("go\n");
      // The helper names the moment the close left, so the way back through its pipe is not counted.
      const SENT = /sent (\d+)\r?\n/;
      return waitFor(SENT).then(() => Number(SENT.exec(out)[1]));
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
