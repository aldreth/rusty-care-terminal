# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages come from **Mastodon RSS** (no Twitter).

It ships as a single binary and is configured with **environment variables** and/or **CLI flags** (no config file).

## Features

- Today (last 24 hours) and last-week git panels plus a bar chart, showing commits where **you** are the author, committer or a `Co-authored-by` co-author
- Three care messages from Mastodon (`@user@instance` → `https://{host}/@{user}.rss`), with a public REST fallback, an offline cache and rate-limit backoff
- A say-box with four pixel-art animals (parrot, cat, bunny, llama) that resize cleanly; `a` cycles them
- Weather from [Open-Meteo](https://open-meteo.com/) (city name or `lat,lon`, no API key)
- A pomodoro timer in the say-box
- Auto, dark and light themes that never paint over your terminal's background
- Refreshes run in the background, so the UI stays responsive on slow or flaky networks

## Install

Prebuilt binaries for macOS (Apple silicon and Intel), Linux (x86_64 and arm64) and Windows are attached to each [GitHub Release](https://github.com/aldreth/rusty-care-terminal/releases):

```bash
# macOS / Linux
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/aldreth/rusty-care-terminal/releases/latest/download/rusty-care-terminal-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/aldreth/rusty-care-terminal/releases/latest/download/rusty-care-terminal-installer.ps1 | iex"
```

From crates.io (needs Rust 1.89 or newer):

```bash
cargo install rusty-care-terminal --locked
```

`git` must be on your `PATH` for the commit panels.

## Run

```bash
rusty-care-terminal --repos "$HOME/Code" --weather London

rusty-care-terminal --mock   # offline care messages, no Mastodon requests
```

It needs an interactive terminal; piping its output exits with an error.

## Configuration

Every option is a flag and an `RCT_*` environment variable; a flag beats its variable. `rusty-care-terminal --help` lists them all.

| Env | Flag | Default | Purpose |
| --- | --- | --- | --- |
| `RCT_REPOS` | `--repos` | none | Comma-separated folders to scan for git repos (`~/` works) |
| `RCT_REPOS_DEPTH` | `--repos-depth` | `1` | How deep to look for repos under each folder (1–5) |
| `RCT_GIT_AUTHOR` | `--git-author` | each repo's `user.email` / `user.name` | Your identity: comma-separated emails and/or names |
| `RCT_BOTS` | `--bots` | see below | Comma-separated `user@instance` care accounts; the first feeds the say-box |
| `RCT_BOT_FEEDS` | `--bot-feeds` | none | Comma-separated RSS/Atom URLs; these take the first care slots |
| `RCT_SAY_BOX` | `--say-box` | `parrot` | Starting animal: `parrot`, `cat`, `bunny`, `llama` or `random` |
| `RCT_WEATHER` | `--weather` | none | City name or `lat,lon` |
| `RCT_CELSIUS` | `--celsius` | `true` | `false` for °F and mph (`--celsius false`) |
| `RCT_UPDATE_INTERVAL` | `--update-interval` | `20` | Minutes between automatic refreshes (1–1440) |
| `RCT_POMODORO` | `--pomodoro` | `25` | Focus minutes (1–120) |
| `RCT_BREAK` | `--break-minutes` | `5` | Break minutes (1–60) |
| `RCT_THEME` | `--theme` | `auto` | `auto` (terminal `COLORFGBG`, then OS setting), `dark` or `light` |
| `RCT_TERMINAL_TITLE` | `--terminal-title` | `Rusty care terminal` | Window title; empty or `false` leaves the title alone |
| `RCT_MOCK` | `--mock` | `false` | Offline care messages instead of live feeds |

Booleans accept `true`/`false`, `yes`/`no`, `on`/`off` and `1`/`0`. Invalid values stop the app at startup with an error saying what's wrong.

```bash
export RCT_REPOS="$HOME/Developer/play,$HOME/Code"
export RCT_REPOS_DEPTH=2
export RCT_WEATHER=London
export RCT_BOTS='selfcare@hachyderm.io,selfcaregentle@hachyderm.io,GoatsLive@mastodon.social'
rusty-care-terminal
```

### Your commits

A commit is shown if one of your identities matches its author, committer or a `Co-authored-by` trailer. Emails (anything containing `@`) match emails and other values match names. Matching is exact and case-insensitive, so `ed@example.com` does not match `fred@example.com`. A repo with no identity (no `RCT_GIT_AUTHOR` and no `user.email`/`user.name`) shows nothing rather than everyone's commits. Commits from all branches count; stashes don't.

### Legacy `TTC_*` variables

The original tiny-care-terminal variables still work, with lower priority than `RCT_*`: `TTC_REPOS`, `TTC_REPOS_DEPTH`, `TTC_WEATHER`, `TTC_CELSIUS`, `TTC_UPDATE_INTERVAL`, `TTC_POMODORO`, `TTC_BREAK`, `TTC_TERMINAL_TITLE`, `TTC_BOTS` and `TTC_SAY_BOX`. Values that only made sense in the original app are ignored instead of failing: Twitter handles in `TTC_BOTS` (the default Mastodon accounts are used), and cowsay/ANSI art in `TTC_SAY_BOX` (the parrot is used). Twitter API keys are not used.

### Network, cache and offline use

- The app contacts the Mastodon instances in your care accounts (or your feed URLs), `api.open-meteo.com` and `geocoding-api.open-meteo.com` (only when `RCT_WEATHER` is set). Nothing else, and no telemetry.
- Requests time out after 10 seconds and responses over 2 MB are rejected.
- The last good message per account is cached in `$XDG_CACHE_HOME/rusty-care-terminal/` (default `~/.cache/rusty-care-terminal/`). It's used when a feed fails; without a cache entry, a built-in kind message is shown.
- On HTTP 429 the app stops calling feeds for the server's `Retry-After` (1–60 minutes, default 5) and shows cached messages.
- Problems never interrupt the dashboard: the first one is shown in the footer.

## Keys

| Key | Action |
| --- | --- |
| `h` / `?` | Help |
| `q` / `Esc` | Quit (closes help or a fullscreen list first) |
| `Ctrl+C` | Quit straight away |
| `r` | Refresh git, care messages and weather now |
| `t` / `w` | Fullscreen Today / Week commits (easy to select and copy) |
| `Tab` | Choose which commit list `j`/`k` scrolls (highlighted border) |
| `j` / `k` or `↓` / `↑` | Scroll the selected commit list |
| `a` | Next say-box animal (parrot → cat → bunny → llama) |
| `p` | Toggle the pomodoro timer in the say-box |
| `s` | Start / pause the timer |
| `e` | Reset the current phase |
| `u` / `U` | Focus minutes +1 / −1 |
| `b` / `B` | Break minutes +1 / −1 |

The theme is set only by `RCT_THEME` / `--theme`. The say-box animals are pixel sprites with hand-drawn sizes; the box uses the largest that fits (scaling only by whole pixels) and hides the art when the panel is too short.

The window title is saved and restored on exit in terminals that support the xterm title stack (most modern ones).

## Default care accounts

| Account | Slot |
| --- | --- |
| `selfcare@hachyderm.io` | Say-box — selfcare.tech tips |
| `selfcaregentle@hachyderm.io` | Care panel — gentler cadence |
| `GoatsLive@mastodon.social` | Care panel — wholesome goats |

Alternatives: `3goodthings@fedigroups.social` (often an empty feed), `scream@bots.robots.rodeo`, `meditation@fedigroups.social`.

## Layout

```
┌──────────── last 24h ────────────┬ commits ┬ weather ─┐
│ git subjects                     │  bar    ├ care #2 ─┤
│                                  │         ├ care #3 ─┤
├──────────── last week ───────────┴─────────┴──────────┤
│ week commits                     │ say-box / pomodoro │
└───────────────────────────────────────────────────────┘
```

## Troubleshooting

- **No commits:** check `RCT_REPOS` points at your repos (or a folder above them with a larger `RCT_REPOS_DEPTH`), and that `git config user.email` in those repos (or `RCT_GIT_AUTHOR`) matches the email on your commits.
- **"weather unavailable":** try a bigger nearby city, or `lat,lon` such as `51.51,-0.13`.
- **Care messages don't change:** the feed may be rate limited or offline; the footer says which. Cached messages are shown until it recovers.
- **Wrong colours:** set `RCT_THEME=dark` or `RCT_THEME=light`.

## Compared to tiny-care-terminal

| | tiny-care-terminal | rusty-care-terminal |
| --- | --- | --- |
| Language / UI | Node + blessed | Rust + ratatui |
| Care source | Twitter bots | Mastodon RSS (+ REST fallback) |
| Config | `TTC_*` env | `RCT_*` env + CLI flags (no file), `TTC_*` still read |
| Weather | MSN / weather-js | Open-Meteo |
| Say-box art | cowsay / ANSI files | Four built-in pixel animals |
| Pomodoro | Yes | Yes (no desktop notifications) |
| Git | Your commits | Sole or joint authorship (incl. `Co-authored-by`) |

## Development

The toolchain is pinned in `rust-toolchain.toml` (rustup installs it on first use). The minimum Rust for `cargo install` is `rust-version` in `Cargo.toml`.

```bash
cargo run -- --mock --repos .
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test -- --ignored   # live Mastodon fetch (needs network)
```

Profiles (via [cargo-wizard](https://github.com/kobzol/cargo-wizard)):

- **dev**: `debug = 0` for quicker rebuilds
- **release**: `lto`, `codegen-units = 1`, `panic = "abort"` for speed (the terminal is still restored on a panic)
- **dist**: inherits release with `lto = "thin"` for CI binary packaging

`target-cpu=native` is deliberately not committed because it breaks portable release binaries. For a machine-tuned local build:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Releases

Releases are automated with git-cliff, release-plz and dist, following Orhun's [automated Rust releases](https://blog.orhun.dev/automated-rust-releases/) recipe. See [RELEASES.md](./RELEASES.md) for the flow, one-time GitHub setup and local checks.

## License

MIT. Inspired by Monica Dinculescu's [tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal) and the selfcare.tech community.
