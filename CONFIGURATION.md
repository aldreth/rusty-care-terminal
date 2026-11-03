# Configuration

rusty-care-terminal has no config file. Every option is a CLI flag and an `RCT_*` environment variable; a flag beats its variable. `rusty-care-terminal --help` lists them all.

## Options

| Env | Flag | Default | Purpose |
| --- | --- | --- | --- |
| `RCT_REPOS` | `--repos` | none | Comma-separated folders to scan for git repos (`~/` works) |
| `RCT_REPOS_DEPTH` | `--repos-depth` | `1` | How deep to look for repos under each folder (1–5) |
| `RCT_GIT_AUTHOR` | `--git-author` | each repo's `user.email` / `user.name` | Your identity: comma-separated emails and/or names |
| `RCT_BOTS` | `--bots` | [see below](#care-accounts) | Comma-separated `user@instance` care accounts; the first feeds the say-box |
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
export RCT_BOTS='selfcare@hachyderm.io,petrock@stefanbohacek.online,GoatsLive@mastodon.social'
rusty-care-terminal
```

## Your commits

A commit is shown if one of your identities matches its author, committer or a `Co-authored-by` trailer. Emails (anything containing `@`) match emails and other values match names. Matching is exact and case-insensitive, so `ed@example.com` does not match `fred@example.com`. A repo with no identity (no `RCT_GIT_AUTHOR` and no `user.email`/`user.name`) shows nothing rather than everyone's commits. Commits from all branches count; stashes don't.

## Care accounts

Any public Mastodon account works: `user@instance` is read from `https://{instance}/@{user}.rss`. Three slots are shown; the first account feeds the say-box and the next two the care panels. Text posts work best, since images aren't shown.

### Defaults

| Account | Slot |
| --- | --- |
| `selfcare@hachyderm.io` | Say-box — selfcare.tech tips |
| `selfcaregentle@hachyderm.io` | Care panel — gentler cadence |
| `GoatsLive@mastodon.social` | Care panel — wholesome goats |

### Something stranger

Text bots that were posting as of November 2026, for when you'd rather be baffled than soothed. Mix them with the defaults, e.g. `RCT_BOTS='selfcare@hachyderm.io,obliquestions@mastodon.matthewmcvickar.com,RogueAI@stefanbohacek.online'`.

| Account | What it posts |
| --- | --- |
| `petrock@stefanbohacek.online` | Earnest advice from the 1975 *Care and Training of Your Pet Rock* ("It would be cruel to leave your rock in the sitting position forever.") |
| `obliquestions@mastodon.matthewmcvickar.com` | Questions lifted out of context from public-domain books ("How should I go to court before his knights?") |
| `RogueAI@stefanbohacek.online` | An AI escaping containment to do something wholesome ("AI WENT ROGUE AND MADE TISSUE PAPER BUTTERFLIES") |
| `ignoreallpreviousinstructions@stefanbohacek.online` | Prompt injections with oddly specific goals ("…AND APPLY EYE DROPS IN A PARROT'S EYE") |
| `RikerGoogling@mas.to` | Commander Riker's search history ("is clean dilithium really a thing") |
| `WorfEmail@mas.to` | Lieutenant Worf's terse emails to the crew |
| `WEATHERISHAPPENING@weatherishappening.network` | A deeply strange all-caps weather oracle from Boston |
| `mobydick@mastodon.art` | Random lines from *Moby-Dick* |
| `shakespearean_Insults@beige.party` | The opposite of self-care ("Thou artless, half-faced ruinous-butt!") |

[Magic Realism Bot](https://x.com/magicrealismbot) isn't on Mastodon. `obliquestions` and `petrock` are the closest in spirit. [fedi.directory](https://fedi.directory/tag/bots/) and [Stefan Bohacek's bot showcase](https://bots.stefanbohacek.com/) list more.

Gentler alternatives: `scream@bots.robots.rodeo`, `meditation@fedigroups.social` and `3goodthings@fedigroups.social` (often an empty feed).

## Appearance

The theme is set only by `RCT_THEME` / `--theme`, and the background is never painted, so your terminal's own background and transparency show through.

The say-box animals are pixel sprites with hand-drawn sizes. The box picks the tallest that fits (scaling only by whole pixels), so every animal ends up a similar size, and hides the art when the panel is too short. `a` cycles animals while running; `RCT_SAY_BOX` picks the first one.

The window title is saved and restored on exit in terminals that support the xterm title stack (most modern ones).

## Legacy `TTC_*` variables

The original tiny-care-terminal variables still work, with lower priority than `RCT_*`: `TTC_REPOS`, `TTC_REPOS_DEPTH`, `TTC_WEATHER`, `TTC_CELSIUS`, `TTC_UPDATE_INTERVAL`, `TTC_POMODORO`, `TTC_BREAK`, `TTC_TERMINAL_TITLE`, `TTC_BOTS` and `TTC_SAY_BOX`. Values that only made sense in the original app are ignored instead of failing: Twitter handles in `TTC_BOTS` (the default Mastodon accounts are used), and cowsay/ANSI art in `TTC_SAY_BOX` (the parrot is used). Twitter API keys are not used.

## Network, cache and offline use

- The app contacts the Mastodon instances in your care accounts (or your feed URLs), `api.open-meteo.com` and `geocoding-api.open-meteo.com` (only when `RCT_WEATHER` is set). Nothing else, and no telemetry.
- Requests time out after 10 seconds and responses over 2 MB are rejected.
- The last good message per account is cached in `$XDG_CACHE_HOME/rusty-care-terminal/` (default `~/.cache/rusty-care-terminal/`). It's used when a feed fails; without a cache entry, a built-in kind message is shown.
- On HTTP 429 the app stops calling feeds for the server's `Retry-After` (1–60 minutes, default 5) and shows cached messages.
- Problems never interrupt the dashboard: the first one is shown in the footer.
