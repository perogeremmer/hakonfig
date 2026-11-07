# Hakonfig

**Hakonfig = Harness Konfiguration.** A free, open-source desktop app that sets up
AI providers for your coding tools, without hand-editing JSON or YAML.

> The UI ships in **English** by default; Bahasa Indonesia is available in Settings.
> This document is in English.

You have a free API key (Groq, Gemini, Cerebras, ...) and no idea where it goes: which
file, which key, and why the key sometimes lives in a different file than the model
catalog. Hakonfig does it for you: pick your tool, pick a provider, paste the key,
review the diff, write it.

Hakonfig is **not** a proxy or a gateway, and it never stores your keys in its own
place. It only writes to the files your tools already read.

- **Stack:** Tauri v2 (Rust) + HTML + Tailwind CSS v4 + vanilla JS, no frontend framework
- **UI:** Plus Jakarta Sans bundled locally (offline), light and dark themes, English + Bahasa Indonesia
- **Tests:** 80 tests (51 Rust + 29 JS), all offline, no network required
- **License:** [MIT](LICENSE), free to use, modify, and redistribute

### What it does, in one paragraph

Pick your tool, pick a provider, paste one API key. Hakonfig tests the key, pulls the
model list straight from the provider, lets you tick the models you want, shows a
human-readable summary of what will change, and only then writes the config files -
with an automatic backup you can restore from History.

---

## Contents

- [Open source](#open-source)
- [Features](#features)
- [Supported tools](#supported-tools)
- [Requirements](#requirements)
- [Running (dev mode)](#running-dev-mode)
- [Building for distribution](#building-for-distribution)
- [Automated releases](#automated-releases)
- [Tests](#tests)
- [How to use it](#how-to-use-it)
- [Repository layout](#repository-layout)
- [Sandboxing with `HAKONFIG_ROOT`](#sandboxing-with-hakonfig_root)
- [Adding a new tool](#adding-a-new-tool)
- [Security model](#security-model)
- [Known limitations](#known-limitations)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)

---

## Open source

Hakonfig is open source under the [MIT License](LICENSE). That means you can use it
commercially, fork it, embed it, change it, and ship it, as long as the copyright
notice and permission notice are kept. There is no paid tier, no telemetry, and no
account: the app runs entirely on your machine and only talks to the provider endpoint
you configure.

Issues and pull requests are welcome. See [Contributing](#contributing).

---

## Features

- **Three-step provider setup:** pick a provider card with its real logo, paste the API
  key, hit **Test & find models**, tick the models you want. Base URL, API type, and the
  model list are filled in for you; unticked models are never written.
- **24 built-in provider presets** (Groq, Gemini, Cerebras, DeepSeek, OpenRouter,
  Mistral, Together, NVIDIA NIM, OpenCode Go, Ollama, OpenAI, xAI, Moonshot, Z.ai,
  MiniMax, DashScope, Cline, Command Code, DeepInfra, SambaNova, Fireworks, Nebius,
  Vercel AI Gateway, Hugging Face Router) plus **Custom** for any OpenAI-compatible
  endpoint.
- **Custom providers get a neutral mark.** Anything not in the catalog uses the
  OpenAI-compatible logo instead of a two-letter monogram.
- **Registered providers only ask for the key.** For providers that come from the
  catalog, the edit form hides Base URL, API type, id, and display name, because the
  endpoint is already known. Custom providers keep the full form.
- **Light and dark theme** (light by default), with locally bundled fonts so it looks
  identical offline.
- **Bilingual UI:** English by default, Bahasa Indonesia in Settings, and it switches
  instantly, without a restart.
- **Tool versions on the tile** (e.g. `v1.5.0`), detected from `<bin> --version`.
- **Install/uninstall tools** with a progress bar and streamed log, after confirming the
  exact command.
- **Tool catalog** for tools that are not installed yet (Claude Code, Codex CLI,
  Gemini CLI, Aider, Crush, Qwen Code) with an **Install** button that runs the official
  command (shown to you first for confirmation).
- **Detects config paths** and shows them, with a button to open the folder.
- **Reads the current config** as a list: provider, base URL, API type, key status
  (masked), model count.
- **Diff preview before writing.** Nothing is written silently.
- **Disable is not delete.** On Oh My Pi/OpenCode the provider is added to the tool's own
  disable list; on Pi/Hermes the entry is *commented out* in the file
  (`// hakonfig:disabled groq`). Data disappears only when you press **Delete**.
- **Automatic backup** on every write, kept at 5 per tool, plus a **restore** panel.
- **Connection test** against the provider endpoint (key validity, latency, sample models).
- **Change the default model** per tool, written to `settings.json` / `config.yml` /
  `opencode.json` / `config.yaml` depending on the tool.
- **Safe writes:** merge (unknown fields preserved), atomic write (`tmp` + `rename`),
  `0600` permissions for files that hold credentials.
- **Secrets masked** everywhere in the preview (API keys, tokens, passwords).

---

## Supported tools

| Tool | Status | Files managed |
|---|---|---|
| **Pi** | stable | `~/.pi/agent/models.json` (JSONC) + `auth.json` + `settings.json`¹ |
| **Oh My Pi** | stable | `~/.omp/agent/models.yml` + `config.yml`¹ |
| **OpenCode** | stable | `~/.config/opencode/opencode.json` (or `.jsonc`) + `~/.local/share/opencode/auth.json` |
| **Hermes** | experimental | `~/.hermes/config.yaml` + `~/.hermes/.env` |

¹ read-only, used to show the current default provider/model; never written.

---

## Requirements

### All platforms

| Need | Version | Check |
|---|---|---|
| Rust (rustup) | ≥ 1.82 | `cargo --version` |
| Bun | ≥ 1.1 (Node ≥ 20 also works) | `bun --version` |

You do **not** need a global Tauri CLI: it is already a devDependency
(`@tauri-apps/cli`).

### Linux

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev \
  libwebkit2gtk-4.1-dev librsvg2-dev libxdo-dev

# Fedora
sudo dnf install -y webkit2gtk4.1-devel openssl-devel curl wget file \
  librsvg2-devel libxdo-devel

# Arch
sudo pacman -S --needed base-devel curl wget file openssl webkit2gtk-4.1 librsvg libxdo
```

Verify:

```bash
pkg-config --modversion webkit2gtk-4.1   # must print a version number
```

### macOS

```bash
xcode-select --install
```

### Windows

1. **Visual Studio Build Tools** with the *Desktop development with C++* workload
   (MSVC + Windows SDK).
2. **WebView2 Runtime** (preinstalled on Windows 11, downloadable for Windows 10).
3. MSVC Rust toolchain:

```powershell
rustup default stable-msvc
```

---

## Running (dev mode)

```bash
bun install     # once
bun run dev     # build CSS, then start the app
```

`bun run dev` runs `bun run css` first (Tailwind → `web/styles.css`), then `tauri dev`.

### Edit loop you should know about

Tauri **embeds the `web/` assets at compile time**, so there is no hot reload:

| What you changed | What to do |
|---|---|
| Rust files (`src-tauri/src/**`) | `tauri dev` rebuilds and restarts automatically |
| `web/*.html`, `web/*.js` | stop dev and run `bun run dev` again (about 3 s rebuild) |
| `web/styles.src.css` | run `bun run css` (or `bun run css:watch` in another terminal), then restart dev |

The `tauri dev` watcher only watches `src-tauri`, so changes under `web/` do not trigger
a restart. That is the trade-off of shipping without a bundler/dev-server.

To try things without touching your real configs, see
[Sandboxing with `HAKONFIG_ROOT`](#sandboxing-with-hakonfig_root).

---

## Building for distribution

```bash
bun run build          # = bun run css && tauri build
```

Output:

```
src-tauri/target/release/hakonfig                      # binary
src-tauri/target/release/bundle/                       # installers
```

Bundles depend on the OS you build on:

| Build OS | Artifacts |
|---|---|
| Linux | `.deb`, `.rpm`, `.AppImage` |
| macOS | `.app`, `.dmg` |
| Windows | `.msi`, `.exe` (NSIS) |

Build a single bundle type (faster):

```bash
bunx tauri build --bundles deb     # Linux
bunx tauri build --bundles dmg     # macOS
bunx tauri build --bundles nsis    # Windows
```

> **AppImage note:** creating an AppImage downloads `linuxdeploy`/`appimagetool` on first
> use. The result is much larger (**~84 MB**) because it bundles GTK/WebKit, while
> `.deb`/`.rpm` are only **~2.4 MB** since they use system libraries. Offline or
> size-sensitive? Use `--bundles deb`.

Example output of `bun run build` on Linux:

```
bundle/deb/Hakonfig_0.1.0_amd64.deb           2.4 MB
bundle/rpm/Hakonfig-0.1.0-1.x86_64.rpm        2.4 MB
bundle/appimage/Hakonfig_0.1.0_amd64.AppImage 84 MB
```

> **Cross-compilation note:** Tauri **cannot** cross-compile. The three-OS release is
> produced by a GitHub Actions matrix (see [Automated releases](#automated-releases)):
> `macos-latest`, `ubuntu-22.04`, and `windows-latest`.

---

## Automated releases

Two workflows live in `.github/workflows/`:

| Workflow | Trigger | What it does |
|---|---|---|
| `ci.yml` | push to `master`, every PR, manual | version check, JS tests, Tailwind build, `cargo test`, `cargo check`, and a `.deb` dry-run build (no upload) |
| `release.yml` | tag `v*.*.*`, or manual | builds macOS (universal `.dmg`), Windows (NSIS `.exe`), Ubuntu (`.deb`) and uploads them to one **draft release** |

Release flow:

```bash
node scripts/version.mjs bump minor     # 0.1.0 -> 0.2.0 (all files updated)
git commit -am "chore: release v0.2.0"
git tag v0.2.0
git push --follow-tags
```

When the workflow finishes, open the **Releases** tab, check the assets, then **Publish**.
Releases are created as drafts on purpose so you can inspect them first.

What CI guards against:

- `node scripts/version.mjs check` fails if `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json`, and `web/version.js` drift apart.
- The tag must match the app version (`v0.2.0` vs `0.2.0`); otherwise the job stops
  before building anything.
- `build.beforeBuildCommand` in `tauri.conf.json` runs `bun run css`, so `styles.css`
  is always regenerated on the runner.

CI builds are **unsigned**. If you ever need signing, add `APPLE_CERTIFICATE`,
`APPLE_ID`, `APPLE_TEAM_ID` (macOS) and `TAURI_SIGNING_PRIVATE_KEY` /
`WINDOWS_CERTIFICATE` (Windows) as repository secrets and pass them to `tauri-action`.

### Signing (optional, for public releases)

- macOS: requires an Apple Developer certificate plus notarization; without it users must
  `Right click → Open` the first time.
- Windows: without a code signing certificate, SmartScreen shows an extra warning.

Unsigned builds are usually fine for internal or classroom use.

---

## Tests

```bash
bun run test        # 51 Rust tests + 29 JS tests
bun run test:rust   # cargo test
bun run test:js     # node --test web/diff.test.js web/i18n.test.js web/form.test.js
```

All tests run **offline** and never touch your real configs (Rust uses a `HAKONFIG_ROOT`
sandbox).

| Layer | Count | Coverage |
|---|---|---|
| Unit (`engine/doc.rs`) | 16 | JSONC parsing (comments + trailing commas), YAML (empty/null/broken), comment detection, `mask_key`, `is_slug`, `env_name_from`, secret redaction, `.env` edits, `is_changed` |
| Unit (`engine/backup.rs`) | 6 | empty backup, create→list→restore roundtrip, corrupt manifest, newest-first order, retention cap, orphan cleanup |
| Unit (`engine/write.rs`) | 4 | parent dirs, overwrite, no leftover `.hakonfig-tmp`, `0644` vs `0600` permissions |
| Unit (`engine/paths.rs`) | 2 | `HAKONFIG_ROOT` override, `opencode.jsonc` vs `.json` pick |
| Unit (`web/diff.js`) | 20 | `lcsDiff`, context trimming, empty files, huge blocks (anti-hang), XSS escaping, output trimming |
| Unit (`web/i18n.test.js`) | 6 | key parity EN/ID, empty values, placeholder parity, untranslated strings, no em dash |
| Unit (`web/form.test.js`) | 3 | no DOM reads after the form closes, payload built from copies |
| Integration (`engine/tests.rs`) | 23 | merge of unknown fields, key written to the right file, `$ENV` mode, disable/comment behavior, remove, no-op remove, backup/restore, preview redaction, input validation, per-adapter `stored_key` |

---

## How to use it

1. Open Hakonfig. Installed tools are detected immediately, together with their paths.
2. Pick a tool (e.g. **Pi**).
3. Click **+ Add provider** → pick a preset (e.g. **Groq**) → paste the API key →
   optionally run **Test connection**.
4. Press **Save** → a **diff preview** appears → press **Write to disk**.

That's it. If something looks wrong, open **History** → **Restore**.

---

## Repository layout

```
hakonfig/
├── web/                          # frontend: HTML + Tailwind + vanilla JS (no framework)
│   ├── index.html
│   ├── main.js                   # UI and flows
│   ├── version.js                # APP_VERSION, generated by scripts/version.mjs
│   ├── diff.js                   # diff logic (separated so it can be tested)
│   ├── diff.test.js              # diff unit tests
│   ├── i18n/                     # en.js, id.js, index.js
│   ├── assets/providers/         # logos (local files, no CDN)
│   ├── fonts/                    # bundled webfonts + licenses
│   ├── styles.src.css            # Tailwind source
│   └── styles.css                # build output (gitignored)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs           # UI <-> engine bridge (Tauri commands)
│   │   └── engine/
│   │       ├── model.rs          # shared types
│   │       ├── paths.rs          # per-OS path resolution (only OS-specific place)
│   │       ├── doc.rs            # documents: parse / mutate / render / redact
│   │       ├── backup.rs         # snapshot + restore + manifest + retention
│   │       ├── write.rs          # atomic writes + permissions
│   │       ├── preset.rs         # preset loader
│   │       ├── catalog.rs        # tool catalog for the "not installed" section
│   │       ├── version.rs        # `<bin> --version` detection
│   │       ├── tests.rs          # integration tests (through the commands layer)
│   │       ├── testutil.rs       # sandbox + mutex for tests
│   │       └── adapters/         # one file per tool
│   │           ├── pi.rs
│   │           ├── omp.rs
│   │           ├── opencode.rs
│   │           └── hermes.rs
│   ├── presets/providers.json    # provider presets (edit data, not code)
│   ├── presets/harnesses.json    # tool catalog + install commands
│   ├── capabilities/
│   ├── icons/
│   └── tauri.conf.json
├── docs/UI-CONTRACT.md           # command and type contract between UI and Rust
├── scripts/version.mjs           # single source of truth for the version number
├── scripts/make-demo-sandbox.sh  # demo sandbox for UI work
├── CHANGELOG.md
├── PRD.md                        # product document
├── LICENSE
└── README.md
```

The engine (`src-tauri/src/engine/`) **does not import Tauri at all**: it is a plain
library. The UI has no framework either. Both can be reused if the shell is ever
replaced.

---

## Sandboxing with `HAKONFIG_ROOT`

For development and testing, every tool directory can be redirected to one root:

```bash
./scripts/make-demo-sandbox.sh /tmp/hk-demo
HAKONFIG_ROOT=/tmp/hk-demo bun run dev
```

Mapping: `$HAKONFIG_ROOT/pi`, `/omp`, `/hermes`, `/opencode/config`, `/opencode/data`.

The Rust tests use the same mechanism, so your real configs are never touched.

---

## Adding a new tool

1. Create `src-tauri/src/engine/adapters/<name>.rs` implementing the `Adapter` trait:
   `specs()`, `backup_base()`, `snapshot()`, `build()`, plus optional
   `stored_key()` (used by the backend for Test/Sync when the key field is left empty),
   `experimental()`, and `notes()`.
2. Register it in `adapters::all()` (`src-tauri/src/engine/adapters/mod.rs`).
3. Add integration tests in `engine/tests.rs`.

The UI **does not need changes**: the tool list, provider list, and forms are all
rendered from backend data.

---

## Security model

- Keys stay in the tool's own files (that is how the tools work). Hakonfig never copies
  them into storage of its own.
- Stored keys are read back **only in the backend** (`Adapter::stored_key`) for Test/Sync;
  the value is never sent to the UI.
- Keys are never logged.
- Saving without filling in the key field never touches existing credentials, so renaming
  a provider or editing its models is safe.
- The preview masks secret values, including inside the diff.
- Always **backup first**: `<tool dir>/.hakonfig/backups/<timestamp>/` + `manifest.json`.
  Only the 5 newest backups per tool are kept.
- Atomic writes: a temp file in the same directory, then `rename`.
- Files holding credentials are written with `0600` permissions.

---

## Known limitations

1. **Comments are lost in JSON/JSONC/YAML files.** Rewriting `models.json`, `models.yml`,
   `opencode.json`, or `config.yaml` drops comments inside them. Hakonfig **warns** you
   and always makes a backup. `.env` files are unaffected (edited line by line, comments
   preserved).
2. **Hermes is still experimental.** Only providers and API keys are written; Hermes
   fetches its own model list from the endpoint.
3. **Oh My Pi has no stable separate auth file**, so the key is written as plaintext
   `apiKey` in `models.yml` (`0600` permissions).
4. **OpenCode paths on macOS/Windows** try several candidates (XDG,
   `Library/Application Support`, `%APPDATA%`). Unusual installs may need a tweak in
   `engine/paths.rs`.
5. **Preset model lists are static.** Providers add and retire models constantly, so
   update `src-tauri/presets/providers.json`. The ids are a starting point; the
   **Sync models** button pulls the live list from the endpoint. Providers whose catalog
   cannot be read without a key (Fireworks, Nebius) intentionally ship with an empty
   list, filled in via Sync after the key is set.
6. **No hot reload** for `web/` (no bundler/dev-server).
7. Tool config schemas can change between versions. Always review the diff and use the
   backup panel if the result looks off.

---

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `The system library webkit2gtk-4.1 was not found` | Linux dependencies missing → see [Requirements](#requirements). Check with `pkg-config --modversion webkit2gtk-4.1` |
| `error: linker cc not found` (Linux) | `sudo apt install build-essential` |
| `link.exe not found` / MSVC errors (Windows) | Install Visual Studio Build Tools, run `rustup default stable-msvc`, open a new shell |
| `Gtk-Message: Failed to load module "xapp-gtk3-module"` | **Safe to ignore**: a Linux Mint/Cinnamon theme module that is simply absent |
| White window / empty UI | `web/` assets not rebuilt → `bun run css && bun run dev` |
| Unstyled UI / `styles.css` 404 | `bun run css` was not run |
| AppImage build fails | It downloads tools from the internet → use `--bundles deb` |
| Provider missing from a tool | Check the path shown in the UI; make sure the tool has been started at least once |
| `config file could not be parsed` | Hakonfig fails loudly rather than overwriting a broken file. Fix the JSON/YAML first (a backup is available) |
| Want to experiment safely | Use `HAKONFIG_ROOT=/tmp/hk-demo` |

---

## Contributing

Pull requests, issues, and new tool adapters are very welcome. To add a tool, see
[Adding a new tool](#adding-a-new-tool); it is a single adapter file, with no UI changes.

Before sending a PR:

```bash
bun run test            # 51 Rust + 29 JS tests must pass
bun run version:check   # version must be consistent across files
bun run css             # make sure Tailwind still builds
```

Keep UI copy in both `web/i18n/en.js` and `web/i18n/id.js` (a test enforces key parity),
and keep new logos as local files in `web/assets/providers/` (no CDN links).

---

## License

[MIT](LICENSE) © 2026 perogeremmer and contributors.

In short: anyone is free to use, copy, modify, merge, publish, distribute, sublicense,
and/or sell copies of this software, provided the copyright notice and permission notice
are included. The software is provided **without warranty** of any kind.
