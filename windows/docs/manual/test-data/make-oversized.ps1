# Makes two oversized files in the current folder. Run it in PowerShell.
# 1. oversized-5MiB-plus-1.txt: 5,242,881 bytes of text. The pasted-text limit in the import dialog is 5 MiB.
#    Open it, copy all, and paste it into the paste box.
# 2. oversized-100MiB-plus-1.fountain: 104,857,601 bytes. The import file limit is 100 MiB.
#    The size check runs before the file is read, so the content does not matter.
$bytes = 5MB + 1
$text = "INT. TEST - DAY`r`n`r`n" + ("A" * ($bytes - 19))
[System.IO.File]::WriteAllText("$PWD\oversized-5MiB-plus-1.txt", $text)
(Get-Item "$PWD\oversized-5MiB-plus-1.txt").Length

$fs = [System.IO.File]::Create("$PWD\oversized-100MiB-plus-1.fountain")
$fs.SetLength(100MB + 1)
$fs.Close()
(Get-Item "$PWD\oversized-100MiB-plus-1.fountain").Length
