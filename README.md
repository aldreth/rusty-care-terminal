# rusty-care-terminal

A gentle terminal dashboard inspired by [notwaldorf/tiny-care-terminal](https://github.com/notwaldorf/tiny-care-terminal), rebuilt in **Rust** with **ratatui**.

It shows:

- the last posts from [tinycarebot](https://bsky.app/profile/did:plc:neqftbqcrgcypgrexcenyiqx) [@selfcare@hachyderm.io](https://hachyderm.io/@selfcare), & [@obliquestions@mastodon.matthewmcvickar.com](https://mastodon.matthewmcvickar.com/@obliquestions). The first tow posts reminders about taking breaks, drinking water and looking outside, the latter asks questions lifted out of context from public-domain books. If you don't like these bots, they're configurable!
- your git commits, including co-authored commits, from today and the last 7 days. When I get stressed out because I think I haven't done anything, it turns out that I only think about big and serious commits, and forget about all the tiny amounts of work I've actually done throughout. Hopefully this will help you too <3
- the weather, because you might get rained on.

You can also use it as a pomodoro timer.

It looks like this, and updates every 20 minutes.

## Make it work

### Install

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

### Configure

Settings are `RCT_*` environment variables or CLI flags (a flag overrides an environment variable). [CONFIGURATION.md](./CONFIGURATION.md) lists every option and default, how your commits are matched, bots to try (including some delightfully strange ones) and legacy `TTC_*` support.

```bash
export RCT_REPOS="$HOME/Developer/code,$HOME/projects"
export RCT_WEATHER='York, United Kingdom'
```

### Run

```bash
rusty-care-terminal

# or with cli flags
rusty-care-terminal l --repos "$HOME/Code"
```

It needs an interactive terminal; piping its output exits with an error.

## Keys

| Key                    | Action                                        |
| ---------------------- | --------------------------------------------- |
| `h` / `?`              | Help                                          |
| `q` / `Esc`            | Quit (closes help or a fullscreen list first) |
| `Ctrl+C`               | Quit straight away                            |
| `r`                    | Refresh git, bot messages and weather now     |
| `t` / `w`              | Fullscreen Today / Week commits               |
| `Tab`                  | Choose which commit list is scrollable        |
| `j` / `k` or `↓` / `↑` | Scroll the selected commit list               |
| `a`                    | Change animal (parrot → cat → bunny → llama)  |
| `p`                    | Toggle the pomodoro timer                     |
| `s`                    | Start / pause the pomodoro timer              |
| `e`                    | Reset the pomodoro timer                      |
| `u` / `U`              | Focus minutes +1 / −1                         |
| `b` / `B`              | Break minutes +1 / −1                         |

## Troubleshooting

- **No commits:** check `RCT_REPOS` points at your repos (or a folder above them with a larger `RCT_REPOS_DEPTH`), and that `git config user.email` in those repos (or `RCT_GIT_AUTHOR_EMAILS`) matches the email on your commits.
- **"weather unavailable":** try a bigger nearby city, or `lat,lon` such as `51.51,-0.13`.
- **Bot messages don't change:** the feed may be rate limited or offline; the footer says which. Cached messages are shown until it recovers.
- **Wrong colours:** set `RCT_THEME=dark` or `RCT_THEME=light`.

## Development

The toolchain is pinned in `rust-toolchain.toml` (rustup installs it on first use). The minimum Rust for `cargo install` is `rust-version` in `Cargo.toml`.

```bash
cargo run -- --mock --repos .
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test -- --ignored   # live Mastodon fetch (needs network)
```

Coverage uses [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov). CI fails if line coverage drops below 85%:

```bash
rustup component add llvm-tools
cargo install cargo-llvm-cov --locked
cargo llvm-cov --summary-only        # per-file table
cargo llvm-cov --open                # browsable HTML report
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
