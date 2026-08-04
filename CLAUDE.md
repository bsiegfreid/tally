# Project Rules for Claude

No git add, commit, or push.

## What this is

Tally receives build/test stats over JSON POST, stores them in
SQLite, and serves a server-rendered trends page. It is thin on
purpose, and doubles as a worked example of a minimal service —
weigh every line and every dependency against that. It serves
without TLS or auth by design; deployment puts it behind a trusted
boundary.

## Shape (do not redesign)

Single binary, singular module names, per the structure/canopy house
style:

- `config` — all environment reads; nothing else touches env vars
- `model` — plain types, no handles, no format opinions. `NewRun` is
  the pre-insert payload; `RunRow` is a stored run read back
- `format` — content negotiation, pure (canopy's `taproot::format`
  cut down). Unified routes: one `/run` resource, no `/api` prefix;
  `.json` extension canonical, `Accept` fallback, HTML default
- `mapper` — the only door to SQLite. One thread owns the sole
  connection; handlers send commands over a channel. Writes are
  fire-and-forget (202), reads reply on a oneshot
- `route` — handlers and row rendering. The page shell and CSS are
  plain files in `assets/`, baked into the binary via `include_str!`
  — edit them there, never as string literals in Rust

One `run` table, STRICT, generic: `kind` + free-form JSON `detail`,
so a new stat kind needs no schema change. No global state. No
JavaScript — the page refreshes via meta tag, and stays fully
self-contained. Never reference a CDN; if the page ever needs htmx,
vendor the file.

## Before code is ready

```
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
