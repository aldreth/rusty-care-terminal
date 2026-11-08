# Configuration

rusty-care-terminal is configured using `RCT_*` environment variables, which can be overriden with a CLI flag. `rusty-care-terminal --help` lists them all.

## Options

| Env                     | Flag                  | Default                  | Purpose                                                                                        |
| ----------------------- | --------------------- | ------------------------ | ---------------------------------------------------------------------------------------------- |
| `RCT_REPOS`             | `--repos`             | none                     | Comma-separated folders to scan for git repos (`~/` works)                                     |
| `RCT_REPOS_DEPTH`       | `--repos-depth`       | `1`                      | How deep to look for repos under each folder (1–5)                                             |
| `RCT_GIT_AUTHOR_EMAILS` | `--git-author-emails` | each repo's `user.email` | Your commit emails, comma-separated                                                            |
| `RCT_BOTS`              | `--bots`              | [see below](#bots)       | Comma-separated `user@instance` accounts or RSS/Atom feed URLs; the first feeds the animal box |
| `RCT_ANIMAL`            | `--animal`            | `parrot`                 | Starting animal: `parrot`, `cat`, `bunny`, `llama` or `random`                                 |
| `RCT_WEATHER`           | `--weather`           | none                     | City name or `lat,lon`                                                                         |
| `RCT_CELSIUS`           | `--celsius`           | `true`                   | `false` for °F and mph (`--celsius false`)                                                     |
| `RCT_UPDATE_INTERVAL`   | `--update-interval`   | `20`                     | Minutes between automatic refreshes (1–1440)                                                   |
| `RCT_POMODORO`          | `--pomodoro`          | `25`                     | Focus minutes (1–120)                                                                          |
| `RCT_BREAK`             | `--break-minutes`     | `5`                      | Break minutes (1–60)                                                                           |
| `RCT_THEME`             | `--theme`             | `auto`                   | `auto` (follows your terminal's or OS's light/dark switches), `dark` or `light`                |
| `RCT_TERMINAL_TITLE`    | `--terminal-title`    | `Rusty care terminal`    | Window title; empty or `false` leaves the title alone                                          |
| `RCT_MOCK`              | `--mock`              | `false`                  | Offline messages instead of live bot feeds                                                     |

Booleans accept `true`/`false`, `yes`/`no`, `on`/`off` and `1`/`0`. Invalid values stop the app at startup with an error saying what's wrong.

```bash
export RCT_REPOS="$HOME/Developer/play,$HOME/Code"
export RCT_REPOS_DEPTH=2
export RCT_WEATHER=London
export RCT_BOTS='selfcare@hachyderm.io,petrock@stefanbohacek.online,GoatsLive@mastodon.social'
rusty-care-terminal
```

## Your commits

A commit is shown if one of your emails is its author, committer or a `Co-authored-by` trailer. Matching is exact and case-insensitive. List every address you commit with, e.g. `RCT_GIT_AUTHOR_EMAILS='me@home.example,me@work.example'`. Without it, each repo's `git config user.email` is used, and a repo with no `user.email` shows nothing rather than everyone's commits. Commits from all branches count; stashes don't.

## Bots

Any public Mastodon account works: `user@instance` is read from `https://{instance}/@{user}.rss`. Any other RSS or Atom feed works too; see [Other RSS and Atom feeds](#other-rss-and-atom-feeds). Three slots are shown; the first bot feeds the animal box and the next two the other side panels. Text posts work best, since images aren't shown.

### Defaults

| Account                                                                                            | Slot                                                     |
| -------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| [tinycarebot](https://bsky.app/profile/did:plc:neqftbqcrgcypgrexcenyiqx)                           | Selfcare.tech tips from Blue Sky via rss                 |
| [@selfcare@hachyderm.io](https://hachyderm.io/@selfcare)                                           | Selfcare.tech tips from Mastodon                         |
| [@obliquestions@mastodon.matthewmcvickar.com](https://mastodon.matthewmcvickar.com/@obliquestions) | Questions lifted out of context from public-domain books |

### Other bots

A few other suggestions of bots you might want to use in your Rusty Care Terminal.

| Account                                              | What it posts                                                                                                                               |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `selfcare@goto.toydragon.xyz`                        | Kind, sometimes silly reminders every 4 hours ("valid to make tyops. mistakes happen!")                                                     |
| `selfcare@masto.ai`                                  | Twice a day, nudges to look up from the screen ("The world is happening right now. Step outside and be a part of it!")                      |
| `affirmations@thisbot.day`                           | Short affirmations a few times a day. Some come from 12-step material ("My Higher Power…")                                                  |
| `FrogAndToadBot@goose.directory`                     | Lines from the Frog and Toad books. Not advice, just calming                                                                                |
| `inspirationbot@mastodon.social`                     | Longer mental-health check-ins written for men; they get cut to fit the panel                                                               |
| `petrock@stefanbohacek.online`                       | Earnest advice from the 1975 _Care and Training of Your Pet Rock_ ("It would be cruel to leave your rock in the sitting position forever.") |
| `obliquestions@mastodon.matthewmcvickar.com`         | Questions lifted out of context from public-domain books ("How should I go to court before his knights?")                                   |
| `RogueAI@stefanbohacek.online`                       | An AI escaping containment to do something wholesome ("AI WENT ROGUE AND MADE TISSUE PAPER BUTTERFLIES")                                    |
| `ignoreallpreviousinstructions@stefanbohacek.online` | Prompt injections with oddly specific goals ("…AND APPLY EYE DROPS IN A PARROT'S EYE")                                                      |
| `RikerGoogling@mas.to`                               | Commander Riker's search history ("is clean dilithium really a thing")                                                                      |
| `WorfEmail@mas.to`                                   | Lieutenant Worf's terse emails to the crew                                                                                                  |
| `WEATHERISHAPPENING@weatherishappening.network`      | A deeply strange all-caps weather oracle from Boston                                                                                        |
| `mobydick@mastodon.art`                              | Random lines from _Moby-Dick_                                                                                                               |
| `shakespearean_Insults@beige.party`                  | The opposite of self-care ("Thou artless, half-faced ruinous-butt!")                                                                        |

### Other RSS and Atom feeds

Any slot in `RCT_BOTS` can be a public RSS or Atom feed URL instead of a Mastodon account: a blog, a news site, a Mastodon hashtag (`https://mastodon.social/tags/cats.rss`) or anything else that publishes a feed. Mix them freely:

```bash
export RCT_BOTS='selfcare@hachyderm.io,obliquestions@mastodon.matthewmcvickar.com,https://mastodon.social/tags/cats.rss'
```

- The newest entry is shown, refreshed on the same schedule as the bots.
- Entries with a title (most blogs and news feeds) show it above the post as a link to the full post (clickable in terminals with OSC 8 hyperlinks, such as Ghostty, kitty, iTerm2 and WezTerm), and the animal box
- shows only the title. Entries without one (Mastodon posts) show just the text.
- HTML is reduced to plain text, images aren't shown and long posts are cut to about 600 characters, so feeds that publish a summary read best.
- The URL must start with `http://` or `https://`. Feed slots have no profile link above the text, and there's no REST fallback, so a broken feed shows its cached or built-in message.

## Appearance

The theme is set only by `RCT_THEME` / `--theme`, and the background is never painted, so your terminal's own background and transparency show through.

In `auto` mode the app follows light/dark switches while it runs, e.g. at sunset:

- **Terminals that report their colour scheme** (Ghostty, kitty, Contour, recent VTE terminals, tmux 3.6+): the app asks the terminal at startup and switches the moment the terminal does, including when you change the terminal's own theme. In Ghostty, `theme = light:…,dark:…` makes the terminal follow macOS, and the app follows the terminal.
- **Other terminals:** the app starts from `COLORFGBG` if the terminal sets one, otherwise the OS setting (macOS appearance, Windows app mode, or GNOME/freedesktop colour scheme). It re-checks the OS every 30 seconds and on every refresh (`r` or the automatic one). If your terminal keeps the same colours whatever the OS does, pin `RCT_THEME=dark` or `light`.

The animals are pixel sprites with hand-drawn sizes. The box picks the tallest that fits (scaling only by whole pixels), so every animal ends up a similar size, and hides the art when the panel is too short. `a` cycles animals while running; `RCT_ANIMAL` picks the first one.

The window title is saved and restored on exit in terminals that support the xterm title stack (most modern ones).

## Legacy `TTC_*` variables

The original tiny-care-terminal variables still work, with lower priority than `RCT_*`: `TTC_REPOS`, `TTC_REPOS_DEPTH`, `TTC_WEATHER`, `TTC_CELSIUS`, `TTC_UPDATE_INTERVAL`, `TTC_POMODORO`, `TTC_BREAK`, `TTC_TERMINAL_TITLE`, `TTC_BOTS` and `TTC_SAY_BOX`. Values that only made sense in the original app are ignored instead of failing: Twitter handles in `TTC_BOTS` (the default Mastodon accounts are used), and cowsay/ANSI art in `TTC_SAY_BOX` (the parrot is used). Twitter API keys are not used.

## Network, cache and offline use

- The app contacts the Mastodon instances and feed URLs in `RCT_BOTS`, `api.open-meteo.com` and `geocoding-api.open-meteo.com` (only when `RCT_WEATHER` is set). Nothing else, and no telemetry.
- Requests time out after 10 seconds and responses over 2 MB are rejected.
- The last good message per account is cached in `$XDG_CACHE_HOME/rusty-care-terminal/` (default `~/.cache/rusty-care-terminal/`). It's used when a feed fails; without a cache entry, a built-in kind message is shown.
- On HTTP 429 the app stops calling feeds for the server's `Retry-After` (1–60 minutes, default 5) and shows cached messages.
- Problems never interrupt the dashboard: the first one is shown in the footer.
