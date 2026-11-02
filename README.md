# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages are designed for **Mastodon RSS** (no Twitter).

This MVP shows:

- Today / last-week git commit panels + bar chart (real `git log` scan)
- Care / say panels fed from **Mastodon RSS** (`@user@instance` → `https://{host}/@{user}.rss`)
- Weather stub
- Keys: `q` / `Esc` quit, `r` refresh

## Requirements

- Rust 1.99.1+ (edition 2024; pinned in `rust-toolchain.toml`)
- `git` on `PATH` for commit scanning

## Build & run

```bash
cargo build
cargo run

# with an explicit config
cargo run -- --config ./config.toml

# offline care fixtures (no network)
cargo run -- --mock
```


## Configure repos

Repo paths come from TOML and/or env. Env wins for the same key.

### Env (parity with original `TTC_*`)

```bash
export RCT_REPOS="$HOME/Code,$HOME/src/project"
export RCT_REPOS_DEPTH=2
export RCT_UPDATE_INTERVAL=20

# Care sources → intended RSS: https://{host}/@{user}.rss
export RCT_BOTS='selfcare@hachyderm.io,selfcaregentle@hachyderm.io,3goodthings@fedigroups.social'
# Or explicit feeds:
# export RCT_BOT_FEEDS='https://hachyderm.io/@selfcare.rss,...'

# Legacy aliases still accepted: TTC_REPOS, TTC_REPOS_DEPTH, TTC_BOTS, …
cargo run
```

### TOML

Copy [`config/config.example.toml`](config/config.example.toml) to `./config.toml` or `~/.config/rusty-care-terminal/config.toml`.

```toml
repos = ["~/Code"]
repos_depth = 2
bots = ["selfcare@hachyderm.io", "selfcaregentle@hachyderm.io", "3goodthings@fedigroups.social"]
mock = true
```

Without configured repos, commit panels stay empty (friendly placeholder copy) and the status line reminds you to set `RCT_REPOS`.

## Layout

```
┌──────────── last 24h ────────────┬ commits ┬ weather ─┐
│ git subjects                     │  bar    ├ care #2 ─┤
│                                  │         ├ care #3 ─┤
├──────────── last week ───────────┴─────────┴──────────┤
│ week commits                     │ say-box / parrot   │
└───────────────────────────────────────────────────────┘
```

## Roadmap

See the project plan: Mastodon **RSS-first** care feeds, then weather, art polish, and pomodoro.

## Credit

Inspired by Monica Dinculescu’s [tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal) and the selfcare.tech community.
