# hakonfig-cli

The command-line companion to the Hakonfig desktop app. It calls the same
`hakonfig_core::api` entry point, so writes, backups, and secret redaction
behave identically to the GUI.

## Install

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/perogeremmer/hakonfig/master/scripts/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/perogeremmer/hakonfig/master/scripts/install.ps1 | iex
```

Other channels:

| Channel | Command |
|---|---|
| Homebrew | `brew install perogeremmer/tap/hakonfig-cli` |
| npm | `npm i -g hakonfig` |
| crates.io | `cargo install hakonfig-cli` |

The install scripts download the binary from the latest **published** release,
verify it against the release's `SHA256SUMS`, and install into
`~/.local/bin` (override with `HAKONFIG_INSTALL_DIR`). A specific version can
be installed by passing the tag, e.g. `install.sh v0.4.2`.

## Global flags

| Flag | Effect |
|---|---|
| `--home <DIR>` | Treat `DIR` as the HOME of the target user, e.g. an agent's dedicated home (`/var/lib/openbot`). Harness directories resolve inside it with their **real** layout (`<DIR>/.omp`, `<DIR>/.pi`, ...), and version probes use its `.local/bin`. Same as `HAKONFIG_HOME`. |
| `--root <DIR>` | Redirect every harness directory under `DIR` as a **test** layout (`<DIR>/<harness>/...`). For tests and sandboxes — this is not `--home`. Same as `HAKONFIG_ROOT`. |
| `--json` | Print machine-readable JSON instead of tables. Also forces non-interactive mode. |
| `--lang <en\|id>` | UI language. Default `en`; `HAKONFIG_LANG` env var is also honoured. |

`--home` wins over `--root` when both are given.

## Harnesses that run as another user (ADE / VPS)

Agentic setups often give every agent its own user and home (systemd services
with `User=openbot`, home `/var/lib/openbot`). The harness then reads its
config from **that** home, so Hakonfig must target it:

```bash
hakonfig-cli --home /var/lib/openbot show omp
hakonfig-cli --home /var/lib/openbot add --harness omp --provider deepseek --key-stdin
```

Rules of thumb:

- **`--home` is the real layout; `--root` is the test layout.** Passing
  `--root /var/lib/openbot` silently sees an empty machine
  (`<root>/omp/agent` does not exist there). If you meant a real home, pass
  `--home`.
- **Never run as root against another user's home.** Files you write would be
  owned by root and the agent could lose access to its own keys. Run as the
  owner instead:

  ```bash
  sudo -u openbot -H env PATH="/var/lib/openbot/.local/bin:$PATH" hakonfig-cli providers
  ```

  `sudo -u` alone does not set `HOME`; `-H` does (or use `sudo -iu openbot`).
  `PATH` must be explicit because sudo uses a secure path. The binary must be
  executable by that user, so install it outside a private home (`.deb`,
  `/usr/local/bin`), not in `~/.npm-global` of the admin user.
- Writes into a home you cannot write report the fix directly: the error
  message names `sudo -u <user> -H hakonfig-cli ...`.
- If the agent runs its harness **inside a container**, none of the above
  applies: target it through `docker exec` / `kubectl exec` instead.

Rules that hold everywhere:

- With any flag or `--json`, the command never prompts (scripts and servers are safe).
- In a terminal without flags, `add` runs as a wizard.
- An explicit `--test` that fails aborts with a non-zero exit code and writes nothing.
- Read-only commands (`list`, `show`, `providers`, `paths`, `doctor`) never touch files unless `doctor --fix` is passed.
- Cancelling a prompt (Esc / Ctrl-C) exits with code 0, not an error.

## Commands

### `list` (default)

Harnesses detected on this machine. Human output is a table; with `--json`
it prints an array of `HarnessInfo`:

```json
{
  "id": "pi",
  "name": "Pi",
  "installed": true,
  "experimental": false,
  "files": [{ "path": "...", "role": "models", "exists": true, "sensitive": false }],
  "notes": "",
  "providerCount": 2,
  "connectedCount": 1,
  "readError": null
}
```

### `show [HARNESS]`

Config contents of one harness. The harness argument is optional when exactly
one installed harness exists. `--json` prints a `HarnessSnapshot`:

```json
{
  "harness": "pi",
  "files": [{ "path": "...", "role": "models", "exists": true, "sensitive": false }],
  "providers": [
    {
      "id": "groq",
      "name": "Groq",
      "baseUrl": "https://api.groq.com/openai/v1",
      "api": "openai-completions",
      "hasKey": true,
      "keyDisplay": "gsk-****x",
      "models": [{ "id": "llama-3.3-70b-versatile", "contextWindow": 128000 }],
      "enabled": true,
      "canToggle": true
    }
  ],
  "defaultProvider": "groq",
  "defaultModel": "llama-3.3-70b-versatile",
  "warnings": []
}
```

Key values are never included: only `hasKey` (boolean) and `keyDisplay`
(masked) are exposed.

### `providers [--harness <id>] [--provider <id>]`

**Provider yang terpasang**, bukan katalog. Tanpa argumen di terminal, ini pintu
kelola (sama seperti layar provider di aplikasi desktop): pilih harness → pilih
provider → menu detail.

```
gateway  ·  Gateway  ·  pi  ·  3 models  ·  key saved  ·  ★ gateway/live-b
> What do you want to change?
  Change the API key only
  Choose models (key stays)
  Set the default model
  Change endpoint and name
  Remove this provider
  Back
```

Non-interaktif (`--json` atau bukan TTY) mencetak tabel:

```
HARNESS  ID       NAME     MODELS  KEY  DEFAULT
pi       gateway  Gateway  3       yes  ★
```

### `catalog [--search <q>] [--free]` (alias `presets`, `provider-list`)

Katalog preset yang didukung build ini: `ID NAME PRESET MODELS BADGE ENV`. Ini
isi perintah `providers` sebelum 0.5.0; `providers --search`/`--free` masih
diteruskan ke sini supaya skrip lama tidak patah.

### `default [--harness <id>]`

Default provider + model tiap harness, dibaca dari config harness itu:

```
HARNESS  DEFAULT PROVIDER  DEFAULT MODEL
pi       deepseek           deepseek/deepseek-v4-pro
```

### `set <key|models|default>`

Mengubah **satu hal** pada provider yang sudah terpasang:

```bash
hakonfig-cli set key --harness pi --provider deepseek          # key dari stdin
hakonfig-cli set models --harness pi --provider deepseek --sync
hakonfig-cli set models --harness pi --provider deepseek --models a,b
hakonfig-cli set default --harness pi --provider deepseek --model deepseek-v4-pro
```

Default itu milik **harness**, jadi harness dipilih lebih dulu dan nilainya
ditulis sebagai referensi `provider/model`. Interaktif tanpa `--provider` /
`--model` menanyakan provider lalu model dari provider itu.

### `remove provider [--harness <id>] [--provider <id>] [--yes]`

Menghapus provider dari config harness, termasuk entry kredensialnya.
Konfirmasi defaultnya "tidak", dan backup dibuat lebih dulu.

### `doctor [--harness <id>] [--fix] [--online]`

`add` already reconciles the obvious cases while it writes — it widens a saved
pi scope, lifts a disable flag for the provider it just wrote, and (for pi)
keeps `enabledModels` free of entries that no longer point at anything — so the
new models show up without extra steps. `doctor` is the safety net for state that
drifted afterwards (a scope saved later with `Ctrl+S`, a config written by an
older Hakonfig, an id that has to be renamed):

| Notice | Meaning | Fix |
|---|---|---|
| `pi.scope` | pi limits the model picker to `enabledModels` in `settings.json`; models of a provider are outside it (a saved scope from `Ctrl+S`, or a pattern like `provider/*` that does not match ids containing `/`). | Adds `<provider>/**` and drops the narrower `<provider>/*`. |
| `omp.disabled` / `opencode.disabled` | The provider is listed in `disabledProviders` / `disabled_providers`, so its models stay hidden even though they are in `models.yml` / `opencode.json`. | Removes the provider from the list (the entry and its models are kept). |
| `catalog.credential-without-provider` | pi punya kredensial untuk provider itu (`auth.json` atau env seperti `XAI_API_KEY`), jadi pi me-refresh katalognya dari pi.dev — tetapi `models.json` tidak punya entry provider tersebut, jadi yang bisa dipakai hanya katalog bawaan. | Manual: tambahkan provider itu lewat `add`, atau hapus kredensialnya kalau tidak dipakai. |
| `catalog.empty-cache` | Cache katalog pi.dev untuk provider itu tersimpan kosong/tanpa timestamp — permintaan terakhirnya gagal atau ditolak, dan pi jatuh ke daftar bawaan versi terpasang. | Manual: cek jaringan lalu buka `/model` di pi (refresh ulang). |
| `catalog.refresh` | Muncul hanya dengan `--online`: permintaan katalog ke pi.dev gagal sekarang, dengan status/error aslinya. Inilah yang tampil di pi sebagai "Could not refresh <provider>; showing cached models". | Manual: jaringan, atau kredensial yang setengah hidup (mis. OAuth kedaluwarsa). |
| `harness.builtin-id` | The provider id is also a built-in harness provider (`openrouter`, `deepseek`, `opencode-go`, …), so the remote catalog for that id is merged with your models and refresh warnings may mention it. | Manual: rename the gateway (for example `openrouter-gw`). |

```
$ hakonfig-cli doctor
harness  provider  issue                                                     fix
pi       gateway   1 of 2 models of 'gateway' are outside pi's enabledModels…  Add 'gateway/**' to enabledModels

$ hakonfig-cli doctor --fix
fixed: enabledModels in settings.json: removed 'gateway/*'; added 'gateway/**'
```

Read-only without `--fix`. Exit code is non-zero while an issue that `--fix`
could repair remains, so it can gate a setup script; notices that are
manual-only never fail the run.

`--online` is the only part that uses the network: it asks every remote catalog
the harness would refresh (one request per provider, 4 s per attempt, two
retries, mirroring pi) and reports the real status/error, because a failed
refresh leaves no trace in the config files — it only shows up as a warning
inside the harness. Without `--online`, `doctor` stays completely offline. The
catalog base URL can be redirected with `HAKONFIG_PI_CATALOG_BASE` (tests,
proxies). `--json` prints the notices as they are
returned by the API (`code`, `harness`, `provider`, `message`, `fix`).

The same notices are included in `add`'s `ApplyResult`, so anything Hakonfig
could not repair itself is reported immediately instead of after a confused
model-picker refresh. Automatic repairs appear in the preview as an ordinary
diff of `settings.json` / `config.yml` / `opencode.json` and are covered by the
same backup.

### `paths [HARNESS]`

Config file paths used by a harness. `--json` prints an array of
`{ "path": "...", "role": "models", "exists": true, "sensitive": false }`.

### `add`

Menambah provider baru. Kalau provider itu sudah terpasang, `add` membuka
detailnya lewat menu `providers` (bukan menu terpisah). Untuk skrip, flag sempit
tetap berlaku dan hanya menyentuh satu hal:

| Yang diberikan | Yang dilakukan |
|---|---|
| hanya key (`--key-stdin`) | `set_key` |
| `--models` / `--sync` | `set_models` |
| `--default-model <id>` | `set_default` |
| `--everything` | tulis ulang penuh |

Non-interactive contract:

| Flag | Effect |
|---|---|
| `--harness <id>` | Target harness. Optional when unambiguous. |
| `--provider <id>` | Catalog provider id, or your own id for a custom endpoint. |
| `--name <NAME>` | Display name (required for a custom provider). |
| `--base-url <URL>` | Base URL (required for a custom provider). |
| `--api <TYPE>` | API type for a custom provider. Default `openai-completions`. |
| `--key-stdin` | Read the API key from piped stdin, e.g. `printf '%s' "$KEY" \| hakonfig-cli add ...`. |
| `--key-env` | Store an environment reference (`$PROVIDER_API_KEY`) instead of the inline key. |
| `--models <ID1,ID2>` | Comma separated model ids. Default: every preset model. |
| `--key-only` | Provider sudah ada: ganti API key saja (alias `set key`). |
| `--models-only` | Provider sudah ada: ganti daftar model saja (alias `set models`). |
| `--default-model <ID>` | Provider sudah ada: jadikan model ini default harness (alias `set default`). |
| `--everything` | Provider sudah ada: tulis ulang penuh (endpoint, nama, model). |
| `--sync` | Ask the endpoint which models it has and use that list instead of the preset list. Needs the key itself (`--key-stdin`) unless the endpoint is keyless; a failure aborts. |
| `--test` | Test the endpoint even when non-interactive; a failure aborts. |
| `--no-test` | Skip the connection test. |
| `--dry-run` | Show the preview only, write nothing. |
| `--yes`, `-y` | Do not ask for confirmation (only relevant in a terminal). |

Idempotent: running the same `add` twice writes nothing the second time.
A backup is created automatically before any overwrite.

Where the model list comes from, in order:

1. `--models a,b,c` — explicit ids, no network call at all.
2. `--sync` (or, in a terminal, the "fetch the model list" question that
   defaults to yes once a key is entered) — `GET {base}/models`, then the ids
   are merged with preset metadata: context window from the endpoint wins,
   name/max tokens/reasoning come from the preset when the id is known, and
   ids the preset does not know are added. The desktop app does the same after
   a connection test.
3. The preset list (every model selected, `space` to deselect).
4. Nothing available — interactive asks for a comma separated list, and
   non-interactive exits with "this provider has no model list yet".

With `--json`, `add` prints an `AddReport`:

```json
{
  "harness": "pi",
  "provider": "groq",
  "modelCount": 6,
  "applied": true,
  "filesWritten": ["/home/you/.pi/agent/models.json"],
  "backupDir": "/home/you/.pi/agent/.hakonfig/backups/20261108-101500",
  "warnings": []
}
```

For a dry run, `applied` is `false` and `filesWritten` is empty; the preview
warnings are included in `warnings`.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, including a user cancellation. |
| `1` | Error: unknown harness/provider, validation failure, failed explicit `--test`, unreadable config. |

## Environment variables

| Variable | Effect |
|---|---|
| `HAKONFIG_ROOT` | Same as `--root`: redirect all harness directories. |
| `HAKONFIG_LANG` | Same as `--lang`: output language (`en`, `id`). |
