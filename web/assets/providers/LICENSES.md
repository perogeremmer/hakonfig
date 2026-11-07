# Logo licenses

Logos are trademarks of their respective owners and are used here only to
identify the service they belong to (nominative use, no affiliation claim).

All files are **stored locally** in this folder - the UI never loads a logo from
a remote URL/CDN. Sources are copied from `rekonfig-landing/assets/logos/`, whose
provenance is recorded there; summarized below.

| File (app id) | Sumber | Catatan |
|---|---|---|
| `pi.svg` | `pi.dev/favicon.svg` | glyph gelap - untuk tema terang |
| `pi-dark.svg` | `pi.dev/favicon.svg` | glyph putih - varian tema gelap |
| `omp.svg` | `omp.sh/favicon.svg` | kotak gradient milik mark sendiri |
| `opencode.png`, `opencode-go.png` | apple-touch-icon situs resmi OpenCode | kotak gelap milik mark |
| `hermes.png` | zonalogo.com (`hermes-agent`) | glyph gelap, pola kotak-kotak dibuang |
| `hermes-dark.png` | zonalogo.com (`hermes-agent`) | glyph putih - varian tema gelap |
| `claude-code.png` | apple-touch-icon / og:image Anthropic | |
| `codex.png` | avatar organisasi GitHub OpenAI | dipakai untuk Codex CLI |
| `gemini.png`, `gemini-cli.png` | zonalogo.com (`gemini-cli`) | |
| `aider.png` | avatar organisasi GitHub Aider | |
| `crush.png` | avatar organisasi GitHub Crush | |
| `qwen-code.png` | avatar organisasi GitHub Qwen | |
| `together.png` | og:image Together AI | |
| `nvidia.png` | favicon `nvidia.com` (mata saja) | |
| `ollama.png` | zonalogo.com (`ollama`) | glyph gelap, pola kotak-kotak dibuang |
| `ollama-dark.png` | zonalogo.com (`ollama`) | glyph putih - varian tema gelap |

| `openai.png` | apple-touch-icon OpenAI (disalin dari `rekonfig-landing`) | mark OpenAI |
| `xai.svg` | Simple Icons (`x`) | mark X / xAI |
| `groq.png` | `groq.com/apple-touch-icon.png` | kotak oranye milik mark |
| `cerebras.png` | favicon `cerebras.ai` (via CDN Sanity) | |
| `deepseek.svg` | Simple Icons (`deepseek`) | |
| `openrouter.svg` | Simple Icons (`openrouter`) | |
| `mistral.svg` | Simple Icons (`mistralai`) | |
| `moonshot.svg` | Simple Icons (`moonshotai`) | Kimi / Moonshot |
| `zhipu.svg` | `z-cdn.chatglm.cn/z-ai/static/logo.svg` | Z.ai GLM |
| `minimax.svg` | Simple Icons (`minimax`) | |
| `dashscope.svg` | Simple Icons (`alibabacloud`) | dipakai untuk DashScope |
| `cline.svg` | Simple Icons (`cline`) | |
| `commandcode.png` | `commandcode.ai/apple-touch-icon.png` | |
| `deepinfra.png` | favicon `deepinfra.com` | |
| `sambanova.png` | favicon `sambanova.ai` | |
| `fireworks.png` | favicon `docs.fireworks.ai` | |
| `nebius.png` | `nebius.com/apple-touch-icon.png` | |
| `vercel-gateway.svg` | Simple Icons (`vercel`) | |
| `huggingface.svg` | Simple Icons (`huggingface`) | |

## Konvensi nama

- `<id>.<ext>` dipakai untuk tema terang.
- `<id>-dark.<ext>` = varian untuk tema gelap (hanya untuk mark yang glyph-nya
  gelap di atas transparan: `pi`, `hermes`, `ollama`). Didaftarkan di
  `LOGO_DARK_VARIANT` (`web/main.js`).
- Ekstensi non-SVG didaftarkan di `LOGO_EXT` (`web/main.js`); default `.svg`.

Aset zonalogo.com tidak punya alpha asli: latar transparan disimulasikan dengan
pola kotak-kotak, yang sudah di-key-out dua warnanya sebelum masuk ke repo ini.
