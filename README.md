# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**. Care messages come from **Mastodon RSS** (no Twitter).

It ships as a single binary and is configured with **environment variables** and/or **CLI flags** (no config file).

## Features

- Today (last 24 hours) and last-week git panels plus a bar chart, showing commits where **you** are the author, committer or a `Co-authored-by` co-author
- Three care messages from Mastodon (`@user@instance` → `https://{host}/@{user}.rss`), with a public REST fallback, an offline cache and rate-limit backoff
- A say-box with four pixel-art animals (parrot, cat, bunny, llama) that resize cleanly; `a` cycles them
- Weather from [Open-Meteo](https://open-meteo.com/) (city name or `lat,lon`, no API key)
- A pomodoro timer in the say-box
- Auto, dark and light themes that never paint over your terminal's background; auto follows light/dark switches live (instantly in Ghostty, kitty and other terminals that report them)
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

Settings are `RCT_*` environment variables or CLI flags (a flag wins); there is no config file. **[CONFIGURATION.md](./CONFIGURATION.md)** lists every option and default, how your commits are matched, care accounts to try (including some delightfully strange bots), legacy `TTC_*` support and what the app sends over the network.

```bash
export RCT_REPOS="$HOME/Code"
export RCT_WEATHER=London
export RCT_BOTS='selfcare@hachyderm.io,petrock@stefanbohacek.online,GoatsLive@mastodon.social'
rusty-care-terminal
```

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
