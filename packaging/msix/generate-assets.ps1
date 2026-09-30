# Generates the MSIX visual assets (tiles, app-list icons, Store logo, splash) from the app icon.
# Called by `node scripts/build-msix.mjs --generate-assets`, which passes the size spec. The output
# is committed under packaging/msix/Assets so designers can hand-tune small sizes if needed.
#
#   -Source    square source PNG (apps/desktop/src-tauri/icons/source.png, 1024x1024)
#   -SpecPath  JSON array of { file, width, height, iconFraction } (iconFraction = icon edge as a
#              fraction of min(width, height); 1 = full-bleed, < 1 = centered on transparency)
#   -OutDir    destination folder
param(
  [Parameter(Mandatory = $true)][string]$Source,
  [Parameter(Mandatory = $true)][string]$SpecPath,
  [Parameter(Mandatory = $true)][string]$OutDir
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$spec = Get-Content -Raw -Path $SpecPath | ConvertFrom-Json
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$src = [System.Drawing.Image]::FromFile((Resolve-Path $Source).Path)
try {
  if ($src.Width -ne $src.Height -or $src.Width -lt 512) {
    throw "Source icon must be square and at least 512x512 (got $($src.Width)x$($src.Height))."
  }
  $attrs = New-Object System.Drawing.Imaging.ImageAttributes
  $attrs.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
  foreach ($item in $spec) {
    $w = [int]$item.width; $h = [int]$item.height
    $edge = [int][Math]::Round([Math]::Min($w, $h) * [double]$item.iconFraction)
    $x = [int][Math]::Floor(($w - $edge) / 2); $y = [int][Math]::Floor(($h - $edge) / 2)
    $bmp = New-Object System.Drawing.Bitmap($w, $h, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
      $g = [System.Drawing.Graphics]::FromImage($bmp)
      try {
        $g.Clear([System.Drawing.Color]::Transparent)
        $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver
        $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $dest = New-Object System.Drawing.Rectangle($x, $y, $edge, $edge)
        $g.DrawImage($src, $dest, 0, 0, $src.Width, $src.Height, [System.Drawing.GraphicsUnit]::Pixel, $attrs)
      } finally { $g.Dispose() }
      $bmp.Save((Join-Path $OutDir $item.file), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bmp.Dispose() }
  }
} finally { $src.Dispose() }
Write-Output "Generated $($spec.Count) assets in $OutDir"
