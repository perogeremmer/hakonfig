# hakonfig-cli

The command-line companion to the Hakonfig desktop app. It calls the same
`hakonfig_core::api` entry point, so writes, backups, and secret redaction
behave identically to the GUI.

## Install

```bash
cargo install --path src-tauri/crates/cli   # from a source checkout
```

Prebuilt binaries (`hakonfig-cli-<target>`) are attached to every release.

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
- Read-only commands (`list`, `show`, `providers`, `paths`) never touch files.
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

Preset model lists are static snapshots; `add` can sync the live list from
the endpoint when a key is available.

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
| `--test` | Test the endpoint even when non-interactive; a failure aborts. |
| `--no-test` | Skip the connection test. |
| `--dry-run` | Show the preview only, write nothing. |
| `--yes`, `-y` | Do not ask for confirmation (only relevant in a terminal). |

Idempotent: running the same `add` twice writes nothing the second time.
A backup is created automatically before any overwrite.

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
