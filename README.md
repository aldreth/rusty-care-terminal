# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages come from **Mastodon RSS** (no Twitter).

Meant to ship as a single binary — configure with **env vars** and/or **CLI flags** (no config file).

## Features

- Today / last-week git commit panels + bar chart (commits where **you** are sole or joint author)
- Care / say panels from Mastodon RSS (`@user@instance` → `https://{host}/@{user}.rss`)
- Weather via Open-Meteo (`RCT_WEATHER` city or `lat,lon`)
- Pomodoro mode in the say-box (`p` / `s` / `e` / `u` / `b`)
- Keys: `q` / `Esc` quit, `r` refresh, `p` pomodoro

## Requirements

- Rust toolchain from `rust-toolchain.toml` (edition 2024)
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
| `RCT_WEATHER` | `--weather` | City name or `lat,lon` (Open-Meteo) |
| `RCT_CELSIUS` | `--celsius` | `true` (default) / `false` for °F |
| `RCT_TERMINAL_TITLE` | — | Window title (default `tiny care`) |
| `RCT_POMODORO` | `--pomodoro` | Focus minutes (default 25) |
| `RCT_BREAK` | `--break-minutes` | Break minutes (default 5) |

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
│ week commits                     │ say-box / pomodoro │
└───────────────────────────────────────────────────────┘
```

## Keys

| Key | Action |
| --- | --- |
| `q` / `Esc` | Quit |
| `r` | Refresh git / RSS / weather |
| `p` | Toggle pomodoro in the say-box |
| `s` | Start / pause pomodoro |
| `e` | Reset current phase |
| `u` / `U` | Work minutes +1 / −1 |
| `b` / `B` | Break minutes +1 / −1 |

Desktop notifications are intentionally off (quiet by default for headless/remote use).

## Credit

Inspired by Monica Dinculescu’s [tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal) and the selfcare.tech community.
