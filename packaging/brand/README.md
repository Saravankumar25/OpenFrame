# Straft Studio brand files

These files all come from one image the owner supplied, `straft-logo-source.jpeg` (1406 x 360). Keep that image as it
is. Everything else here can be regenerated from it.

## Files

| File | What it is |
|---|---|
| `straft-logo-source.jpeg` | The owner's logo, untouched. The reference for every comparison. |
| `straft-logo-manrope.svg` | The full logo as a true vector, on the dark background (`#1a1d26`). Paths only, no font needed. The name comes from the task that asked for it. No Manrope font file was supplied, so the wordmark outlines were traced from the image; a comment in the file says so. Delete the `background` rect for a transparent logo. |
| `straft-logo-4k.png` | The logo at 3840 x 983 on the dark background. |
| `straft-logo-4k-transparent.png` | The same at 3840 x 983 with a transparent background. "Straft" is white, so it suits dark surfaces only. |
| `straft-mark.svg` | The film-frame mark alone, on the 24 by 24 grid, transparent. |
| `straft-mark-1024.png` | The mark alone at 1024 x 1024, transparent. |
| `trace_logo.py` | Traces the wordmark and writes the two SVG files. |
| `build_assets.py` | Rasterizes the SVG and writes the logo PNGs listed on this page. |

The mark is not traced. Its geometry is the 24 by 24 grid in `apps/desktop/src-tauri/icons/source.svg`, stroke 1.6, with
gradient stops `#ff5d6c`, `#ff9a3d` at 55 percent and `#ffc23d`. The trace script only finds where it sits on the image.

## Where the logo is used

- Exports and the PDF design package: `crates/straft-import-export/assets/straft-logo.png` and
  `docs/specs/pdf-design-package-v2/00_SYSTEM/assets/straft-logo.png`, 735 x 163 on transparency, cropped to the artwork.
  This is the one variant that changes a colour: paper is white, so "Straft" prints in the dark ink of the earlier logo
  (`#1d1a2e`) instead of white. The mark and "Studio" keep their colours.
- `docs/specs/pdf-design-package-v2/00_SYSTEM/assets/straft-logo-original.png` is the owner's image, pixel for pixel, as
  a PNG.

The app icons, store tiles and splash screens show the mark only, so they do not come from this folder. To regenerate
them, follow `docs/engineering/30-appearance-themes.md`.

## How to regenerate

Run these from the repository root. They need Python 3 with numpy, scipy, Pillow, OpenCV and matplotlib (for contourpy),
and nothing is downloaded.

```
python3 packaging/brand/trace_logo.py              # SVG files, only if the source image changes
python3 packaging/brand/build_assets.py logo       # 4K logo PNGs and the mark PNG
python3 packaging/brand/build_assets.py print      # logos for exports and the PDF package
python3 packaging/brand/build_assets.py verify     # compares the 4K logo with the source image
```

## Rules

Do not stretch the logo, change the mark's gradient or retype the wordmark in a font. If the owner supplies the real font
file or a vector original, replace the traced outlines with it and update this page.
