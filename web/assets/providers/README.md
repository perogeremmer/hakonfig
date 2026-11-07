# Provider & harness logos

Drop files here named exactly after the provider/harness **id**:

```
groq.svg  gemini.png  cerebras.svg  deepseek.svg  openrouter.svg
mistral.svg  together.png  nvidia.png  opencode-go.png  ollama.png
pi.svg  omp.svg  opencode.png  hermes.png
claude-code.png  codex.png  gemini-cli.png  aider.png  crush.png  qwen-code.png
```

Extension is `.svg` by default; PNG files must be registered in `LOGO_EXT`
(`web/main.js`). Marks whose glyph is dark on transparency also ship a
`<id>-dark.<ext>` variant (light glyph) for the dark theme - list them in
`LOGO_DARK_VARIANT`.

Everything is **local**: the UI never points at a remote URL/CDN.

Rules the UI relies on:

- **Transparent background**, no baked-in white box. The tile behind each logo
  gets a tinted background, so a logo with its own backing plate looks wrong.
- Square viewBox (e.g. `0 0 24 24`) and no padding - the UI adds ~18% padding.
- Monochrome logos using `currentColor` would be ideal but full-color brand
  marks are fine too.

If a file is missing, the UI falls back to a two-letter monogram tile. Nothing
breaks - logos can be added incrementally.

Get marks from the vendor's official brand/press page, or from
[Simple Icons](https://simpleicons.org) (CC0-1.0). Record the source and license
of each file in `LICENSES.md` next to this README.
