# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages come from **Mastodon RSS** (no Twitter).

Meant to ship as a single binary — configure with **env vars** and/or **CLI flags** (no config file).

## Features

- Today / last-week git commit panels + bar chart (commits where **you** are sole or joint author)
- Care / say panels from Mastodon RSS (`@user@instance` → `https://{host}/@{user}.rss`), with REST fallback + disk cache
- Weather via Open-Meteo (`RCT_WEATHER` city or `lat,lon`)
- Pomodoro mode in the say-box (`p` / `s` / `e` / `u` / `b`)

## Requirements

- Rust toolchain from `rust-toolchain.toml` (edition 2024)
- `git` on `PATH` for commit scanning

## Build & run

```bash
cargo build --release
./target/release/rusty-care-terminal --repos "$HOME/Code" --weather "London"

cargo run -- --mock   # offline care text (still uses cache/fixtures)
```

Profiles (via [cargo-wizard](https://github.com/kobzol/cargo-wizard)):

- **dev** — `fast-compile` (`debug = 0`) for quicker rebuilds
- **release** — `fast-runtime` (`lto`, `codegen-units = 1`, `panic = "abort"`) for speed
- **dist** — inherits release with `lto = "thin"` for CI binary packaging

`target-cpu=native` from the wizard template is **not** committed (it breaks portable release binaries). For a local machine-tuned build:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Configuration

Env vars (primary) and matching CLI flags. Legacy `TTC_*` env aliases still work for original tiny-care names.

| Env | CLI | Purpose |
| --- | --- | --- |
| `RCT_REPOS` | `--repos` | Comma-separated repo roots (`~/` ok) |
| `RCT_REPOS_DEPTH` | `--repos-depth` | Nested walk depth 1–5 (default 1) |
| `RCT_GIT_AUTHOR` | `--git-author` | Name/email(s) for sole/joint authorship filter |
| `RCT_BOTS` | `--bots` | Care accounts `@user@instance` (first → say-box) |
| `RCT_BOT_FEEDS` | `--bot-feeds` | Explicit RSS/Atom URLs |
| `RCT_SAY_BOX` | `--say-box` | `parrot` \| `bunny` \| `llama` \| `cat` \| `RANDOM` \| art file path |
| `RCT_UPDATE_INTERVAL` | `--update-interval` | Minutes (default 20) |
| `RCT_MOCK` | `--mock` | Skip live RSS; use bundled kindness text |
| `RCT_WEATHER` | `--weather` | City name or `lat,lon` (Open-Meteo) |
| `RCT_CELSIUS` | `--celsius` | `true` (default) / `false` for °F |
| `RCT_TERMINAL_TITLE` | — | Window title (default `tiny care`) |
| `RCT_POMODORO` | `--pomodoro` | Focus minutes (default 25) |
| `RCT_BREAK` | `--break-minutes` | Break minutes (default 5) |
| `RCT_THEME` | `--theme` | `auto` (default: terminal/`COLORFGBG`, then OS) \| `dark` \| `light` |

```bash
export RCT_REPOS="$HOME/Developer/play,$HOME/Code"
export RCT_REPOS_DEPTH=2
export RCT_WEATHER=London
export RCT_BOTS='selfcare@hachyderm.io,selfcaregentle@hachyderm.io,GoatsLive@mastodon.social'
rusty-care-terminal
```

Git panels include a commit if you are the **author**, the **committer**, or listed in a **`Co-authored-by`** trailer.

### Offline / cache

- `--mock` / `RCT_MOCK=true`: bundled kindness strings, no network for care.
- On feed errors or HTTP 429: last good tip from `~/.cache/rusty-care-terminal/care/` (or `$XDG_CACHE_HOME/...`), then static fallback.
- Rate-limit backoff persists briefly across refreshes.

## Keys

| Key | Action |
| --- | --- |
| `q` / `Esc` | Quit |
| `r` | Refresh git / RSS / weather |
| `t` | Toggle dark / light theme |
| `Tab` | Focus today vs week list for scrolling |
| `j` / `k` or arrows | Scroll focused commit list |
| `p` | Toggle pomodoro in the say-box |
| `s` | Start / pause pomodoro |
| `e` | Reset current phase |
| `u` / `U` | Work minutes +1 / −1 |
| `b` / `B` | Break minutes +1 / −1 |

Desktop notifications stay off (quiet for headless/remote use).

## Default care accounts

| Account | Role |
| --- | --- |
| `selfcare@hachyderm.io` | Say-box — selfcare.tech tips |
| `selfcaregentle@hachyderm.io` | Care panel — gentler cadence |
| `GoatsLive@mastodon.social` | Care panel — wholesome goats |

Alternatives: `3goodthings@fedigroups.social` (often empty RSS), `scream@bots.robots.rodeo`, `meditation@fedigroups.social`.

## Compared to tiny-care-terminal

| | tiny-care-terminal | rusty-care-terminal |
| --- | --- | --- |
| Language / UI | Node + blessed | Rust + ratatui |
| Care source | Twitter bots | Mastodon RSS (+ REST fallback) |
| Config | `TTC_*` env | `RCT_*` env + CLI (no file) |
| Weather | MSN / weather-js | Open-Meteo |
| Pomodoro | Yes | Yes (no desktop notify by default) |
| Git | Your commits | Sole or joint authorship (incl. Co-authored-by) |

## Layout

```
┌──────────── last 24h ────────────┬ commits ┬ weather ─┐
│ git subjects                     │  bar    ├ care #2 ─┤
│                                  │         ├ care #3 ─┤
├──────────── last week ───────────┴─────────┴──────────┤
│ week commits                     │ say-box / pomodoro │
└───────────────────────────────────────────────────────┘
```

## Releases

Automated releases follow Orhun’s [git-cliff + release-plz + dist](https://blog.orhun.dev/automated-rust-releases/) recipe (2023), updated for current tooling (`cargo-dist` → **`dist` 0.34**, `release-plz/action@v0.5`). Mergify is skipped — Dependabot PRs are reviewed manually.

See [RELEASES.md](./RELEASES.md) for the flow and required GitHub secrets.

Install from a GitHub Release (after the first tagged release):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/aldreth/rusty-care-terminal/releases/latest/download/rusty-care-terminal-installer.sh | sh
```

Or from crates.io once published: `cargo install rusty-care-terminal --locked`

## License

MIT. Inspired by Monica Dinculescu’s [tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal) and the selfcare.tech community.
