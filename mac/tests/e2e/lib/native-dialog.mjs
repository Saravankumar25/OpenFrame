// Fill a native Windows file dialog (Save or Open) the way a user does: find the dialog window
// by its title, type the full path into "File name" and press the default button. WebDriver
// cannot reach these OS dialogs, so this finds them through Windows UI Automation in PowerShell
// and sends the typing and the click to the dialog's own controls only. The title and path travel
// in environment variables, never through a command line.

import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SCRIPT = String.raw`
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -Namespace OfE2e -Name Win -MemberDefinition @'
[DllImport("user32.dll")] public static extern System.IntPtr SendMessage(System.IntPtr hWnd, int msg, System.IntPtr wParam, System.IntPtr lParam);
'@
$A = [System.Windows.Automation.AutomationElement]
$T = [System.Windows.Automation.TreeScope]
$C = [System.Windows.Automation.ControlType]
$title = $env:OF_DIALOG_TITLE
$path = $env:OF_DIALOG_PATH
$deadline = (Get-Date).AddSeconds([int]$env:OF_DIALOG_TIMEOUT)
$byName = New-Object System.Windows.Automation.PropertyCondition($A::NameProperty, $title)
$isWindow = New-Object System.Windows.Automation.PropertyCondition($A::ControlTypeProperty, $C::Window)
# Only the native dialog window (class #32770): the app's own in-page dialog often has the same
# title, and UI Automation sees into the webview too.
$isNative = New-Object System.Windows.Automation.PropertyCondition($A::ClassNameProperty, '#32770')
$both = New-Object System.Windows.Automation.AndCondition($byName, $isWindow, $isNative)
function Find-Dialog { $A::RootElement.FindFirst($T::Descendants, $both) }
# The File name box (window class Edit; id 1001 in a Save dialog, 1148 in an Open dialog) and the
# Save or Open button (class Button, id 1).
# Windows may expose them as plain panes rather than as an edit box and a button, so they are found
# by id and window class and driven through their own window handles. Nothing is sent to any other
# window: no simulated keyboard, which would type into whatever window is in front.
function Find-Part($dlg, $id, $class) {
  $c = New-Object System.Windows.Automation.AndCondition(
    (New-Object System.Windows.Automation.PropertyCondition($A::AutomationIdProperty, $id)),
    (New-Object System.Windows.Automation.PropertyCondition($A::ClassNameProperty, $class)))
  $dlg.FindFirst($T::Descendants, $c)
}
$dlg = $null
while (-not $dlg -and (Get-Date) -lt $deadline) {
  $dlg = Find-Dialog
  if (-not $dlg) { Start-Sleep -Milliseconds 250 }
}
if (-not $dlg) { throw "No file dialog titled '$title' appeared." }
# The dialog fills in its own default name after it appears, and text set behind its back
# (WM_SETTEXT) is not what it saves. So the name is typed into the box the way typing reaches it:
# select everything, then one WM_CHAR per character. The name is read back, Save is clicked, and
# the dialog must close; otherwise it is tried again.
$EM_SETSEL = 0x00B1
$WM_CHAR = 0x0102
$BM_CLICK = 0x00F5
$Z = [System.IntPtr]::Zero
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Milliseconds 400
  $dlg = Find-Dialog
  if (-not $dlg) { exit 0 }
  $box = Find-Part $dlg '1001' 'Edit'
  if (-not $box) { $box = Find-Part $dlg '1148' 'Edit' }
  $ok = Find-Part $dlg '1' 'Button'
  if (-not $box -or -not $ok) { continue }
  $h = [System.IntPtr]$box.Current.NativeWindowHandle
  [void][OfE2e.Win]::SendMessage($h, $EM_SETSEL, $Z, [System.IntPtr](-1))
  foreach ($ch in $path.ToCharArray()) { [void][OfE2e.Win]::SendMessage($h, $WM_CHAR, [System.IntPtr][int]$ch, $Z) }
  if ($box.Current.Name -ne $path) { continue }
  [void][OfE2e.Win]::SendMessage([System.IntPtr]$ok.Current.NativeWindowHandle, $BM_CLICK, $Z, $Z)
  $closeBy = (Get-Date).AddSeconds(3)
  while ((Get-Date) -lt $closeBy) {
    if (-not (Find-Dialog)) { exit 0 }
    Start-Sleep -Milliseconds 200
  }
}
throw "The file dialog '$title' did not take the path $path."
`;

/** The script as a file: fed through standard input, PowerShell reads it like an interactive prompt
 * and the multi-line loops did not run as written. */
let file = "";
function scriptFile() {
  if (!file) {
    file = join(mkdtempSync(join(tmpdir(), "openframe-e2e-dialog-")), "choose-file.ps1");
    writeFileSync(file, SCRIPT, "utf8");
  }
  return file;
}

/**
 * Wait for the native file dialog titled `title` and choose `path` in it. Call it right after the
 * click that opens the dialog (the click returns while the dialog is open).
 */
export function chooseInFileDialog(title, path, { timeoutSeconds = 20 } = {}) {
  return new Promise((resolve, reject) => {
    const p = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", scriptFile()], {
      env: { ...process.env, OF_DIALOG_TITLE: title, OF_DIALOG_PATH: path, OF_DIALOG_TIMEOUT: String(timeoutSeconds) },
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    let err = "";
    p.stderr.on("data", (d) => (err += d));
    p.stdout.on("data", () => undefined);
    p.on("error", reject);
    p.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`File dialog "${title}": ${err.trim() || `exit ${code}`}`))));
    p.stdin.end();
  });
}
