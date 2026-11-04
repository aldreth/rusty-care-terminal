# Changelog

All notable changes to this project will be documented in this file.

<!-- git-cliff: end of header -->

## [0.1.0]

First release.

### 🚀 Features

- Today / last-week git panels and bar chart for commits you authored, committed or co-authored (`Co-authored-by`), across every repo under `RCT_REPOS`
- Messages from Mastodon bots over RSS (`@user@instance`, or any feed URL), with a public REST fallback, an offline cache and rate-limit backoff
- Say-box with four pixel-art animals (parrot, cat, bunny, llama) that resize cleanly; `a` cycles them
- Weather from Open-Meteo (city name or `lat,lon`), no API key needed
- Pomodoro timer in the say-box (`p`, `s`, `e`, `u`/`U`, `b`/`B`)
- Fullscreen, copy-friendly commit lists (`t` / `w`) and a help screen (`h`)
- Auto, dark and light themes that never paint over the terminal background
- Configuration through `RCT_*` environment variables or matching flags; original `TTC_*` names still work where they make sense
- Refreshes run in the background, so the UI stays responsive on slow networks
- Remote text is stripped of terminal control sequences before display
