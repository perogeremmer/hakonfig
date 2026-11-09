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
| `--root <DIR>` | Redirect every harness directory under `DIR` (same as `HAKONFIG_ROOT`). For tests and sandboxes. |
| `--json` | Print machine-readable JSON instead of tables. Also forces non-interactive mode. |
| `--lang <en\|id>` | UI language. Default `en`; `HAKONFIG_LANG` env var is also honoured. |

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

### `providers`

Search the built-in preset catalog.

| Flag | Effect |
|---|---|
| `--search`, `-s <TEXT>` | Filter by id or name. |
| `--free` | Only providers badged `free` or `local`. |

With `--json` it prints an array of presets:

```json
{
  "id": "groq",
  "name": "Groq",
  "baseUrl": "https://api.groq.com/openai/v1",
  "api": "openai-completions",
  "envVar": "GROQ_API_KEY",
  "docsUrl": null,
  "keyRequired": true,
  "badge": "free",
  "notes": "",
  "models": [{ "id": "llama-3.3-70b-versatile", "name": null, "contextWindow": 128000, "maxTokens": null, "reasoning": false }]
}
```

Preset model lists are static snapshots. In a terminal, `add` offers to sync
the live list from the endpoint (default yes); with `--sync` it always does,
and with `--models` it never does.

### `doctor [--harness <id>] [--fix]`

`add` already reconciles the obvious cases while it writes — it widens a saved
pi scope and lifts a disable flag for the provider it just wrote — so the new
models show up without extra steps. `doctor` is the safety net for state that
drifted afterwards (a scope saved later with `Ctrl+S`, a config written by an
older Hakonfig, an id that has to be renamed):

| Notice | Meaning | Fix |
|---|---|---|
| `pi.scope` | pi limits the model picker to `enabledModels` in `settings.json`; models of a provider are outside it (a saved scope from `Ctrl+S`, or a pattern like `provider/*` that does not match ids containing `/`). | Adds `<provider>/**` and drops the narrower `<provider>/*`. |
| `omp.disabled` / `opencode.disabled` | The provider is listed in `disabledProviders` / `disabled_providers`, so its models stay hidden even though they are in `models.yml` / `opencode.json`. | Removes the provider from the list (the entry and its models are kept). |
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
manual-only never fail the run. `--json` prints the notices as they are
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

Add or change a provider. Non-interactive contract:

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
