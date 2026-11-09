# hakonfig

Command-line companion to [Hakonfig](https://github.com/perogeremmer/hakonfig):
read, change, and write AI provider configs for coding harnesses (Pi, oh-my-pi,
OpenCode, Hermes) without editing JSON/YAML by hand.

It calls the same `hakonfig_core::api` entry point as the desktop app, so writes,
backups, and secret masking behave identically.

## Install

```bash
npm i -g hakonfig        # installs the `hakonfig-cli` binary
bunx hakonfig-cli list   # or run it once without installing
```

One package carries the binaries for macOS (universal), Linux x64, and Windows
x64 — the shim picks the right one, so the same command works everywhere.

Other channels:

```bash
brew install perogeremmer/tap/hakonfig-cli
cargo install hakonfig-cli
curl -fsSL https://raw.githubusercontent.com/perogeremmer/hakonfig/master/scripts/install.sh | sh
```

## Usage

```bash
hakonfig-cli list                            # installed harnesses, with versions
hakonfig-cli show pi                         # current providers of one tool
hakonfig-cli providers --search groq         # browse the preset catalog
hakonfig-cli add --harness pi --provider groq --key-stdin
hakonfig-cli add --harness opencode --provider openrouter --key-stdin --model sonnet
```

Passing any flag — or piping stdout — makes it fully non-interactive, and
`--json` prints machine-readable output, so it fits straight into server setup
scripts and cron jobs. Every write shows a diff, keeps a rolling backup, and
never takes the API key from an argument (use `--key-stdin`).

## Docs

- CLI reference: <https://github.com/perogeremmer/hakonfig/blob/master/docs/CLI.md>
- Desktop app: <https://github.com/perogeremmer/hakonfig>

MIT licensed.
