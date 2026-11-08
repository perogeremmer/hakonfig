# Brand assets

Source of truth for the visual identity. The full guide (colour tokens, type scale,
logo story) lives outside this repo in `~/Documents/Hakonfig/` (`WARNA.md`,
`CERITA-LOGO.md`, `tokens/`, `logo-svg/`, `favicon/`).

| File | What it is |
|---|---|
| `mark.svg` | The mark as SVG: H built from two bars plus a crossbar, one knob top-left and one maroon knob bottom-right. Strokes use `currentColor`, the knob uses `--logo-knob`, so one file works in both themes. |
| `mark-1024.png` | Raster master of the mark, transparent background. |
| `app-icon-1024.png` | App icon: the mark on the neutral canvas tile (`#faf7f2`). Source for `bunx tauri icon`. |
| `app-icon-dark-1024.png` | Same icon for dark surfaces (white mark, `#dca3ac` knob). |

Rules that matter for the UI:

- The app icon is **not** a full brand tile. The mark stays ink coloured with a single
  maroon knob; the tile is just a neutral canvas so the shape survives at 16 px.
- Never recolour the mark per screen. It is ink (`--color-ink`) plus `--logo-knob`
  (`#8c3345` light, `#dca3ac` dark).
- The wordmark is Plus Jakarta Sans 800, already outlined in the lockup; do not retype it
  in another family.

Regenerating the desktop icons after changing the source:

```bash
bunx tauri icon web/assets/brand/app-icon-1024.png
rm -rf src-tauri/icons/android src-tauri/icons/ios   # desktop-only app
```
