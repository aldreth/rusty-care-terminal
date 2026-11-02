# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages come from **Mastodon RSS** (no Twitter).

Meant to ship as a single binary — configure with **env vars** and/or **CLI flags** (no config file).

## Features

- Today / last-week git commit panels + bar chart (commits where **you** are sole or joint author)
- Care / say panels from Mastodon RSS (`@user@instance` → `https://{host}/@{user}.rss`)
- Weather stub (later milestone)
- Keys: `q` / `Esc` quit, `r` refresh

## Requirements

- Rust 1.99.1+ to build (edition 2024; pinned in `rust-toolchain.toml`)
- `git` on `PATH` for commit scanning

## Build & run

```bash
cargo build --release
./target/release/rusty-care-terminal

# or while developing
cargo run -- --repos "$HOME/Code" --repos-depth 2
cargo run -- --mock   # offline care text
```

## Configuration

Env vars (primary) and matching CLI flags. Legacy `TTC_*` env aliases still work for the original tiny-care names.

| Env | CLI | Purpose |
| --- | --- | --- |
| `RCT_REPOS` | `--repos` | Comma-separated repo roots (`~/` ok) |
| `RCT_REPOS_DEPTH` | `--repos-depth` | Nested walk depth (default 1) |
| `RCT_GIT_AUTHOR` | `--git-author` | Your name/email(s) for authorship filter; default = repo `user.email` / `user.name` |
| `RCT_BOTS` | `--bots` | Care accounts `@user@instance` (first → say-box) |
| `RCT_BOT_FEEDS` | `--bot-feeds` | Explicit RSS/Atom URLs |
| `RCT_SAY_BOX` | `--say-box` | `parrot` \| `bunny` \| `llama` \| `cat` |
| `RCT_UPDATE_INTERVAL` | `--update-interval` | Minutes (default 20) |
| `RCT_MOCK` | `--mock` | Skip live RSS; use bundled kindness text |
| `RCT_WEATHER` | — | Reserved for weather milestone |
| `RCT_CELSIUS` | — | Reserved |
| `RCT_TERMINAL_TITLE` | — | Window title (default `tiny care`) |
| `RCT_POMODORO` / `RCT_BREAK` | — | Reserved for pomodoro |

```bash
export RCT_REPOS="$HOME/Developer/play,$HOME/Code"
export RCT_REPOS_DEPTH=2
export RCT_BOTS='selfcare@hachyderm.io,selfcaregentle@hachyderm.io,GoatsLive@mastodon.social'
rusty-care-terminal
```

Without `RCT_REPOS` / `--repos`, commit panels stay empty with a friendly placeholder.

Git panels include a commit if you are the **author**, the **committer**, or listed in a **`Co-authored-by`** trailer (so Cursor/agent joint commits count). Unrelated authors in the same folder tree are skipped. Set `RCT_GIT_AUTHOR` if your identity isn’t in the repo’s git config.

## Layout

```
┌──────────── last 24h ────────────┬ commits ┬ weather ─┐
│ git subjects                     │  bar    ├ care #2 ─┤
│                                  │         ├ care #3 ─┤
├──────────── last week ───────────┴─────────┴──────────┤
│ week commits                     │ say-box / parrot   │
└───────────────────────────────────────────────────────┘
```

## Credit

Inspired by Monica Dinculescu’s [tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal) and the selfcare.tech community.
