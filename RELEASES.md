# Releases

Release automation is based on [Orhun’s 2023 guide](https://blog.orhun.dev/automated-rust-releases/), refreshed for 2026 tooling.

## Stack

| Tool | Role | Notes vs 2023 blog |
| --- | --- | --- |
| [git-cliff](https://git-cliff.org/) | Changelog from conventional commits (`cliff.toml`) | Still the default changelog engine for release-plz |
| [release-plz](https://release-plz.dev/) | Release PR (version + CHANGELOG), tag, optional crates.io | Action is now `release-plz/action@v0.5` (was `MarcoIeni/release-plz-action@v0.5`) |
| [dist](https://axodotdev.github.io/cargo-dist/) (formerly cargo-dist) | Cross-platform binaries, installers, GitHub Release | Config lives in `dist-workspace.toml`; invoke as `dist` or `cargo dist` |
| Dependabot | Cargo + Actions dependency PRs | Weekly cadence |
| Mergify | — | **Skipped** — merge Dependabot PRs yourself (or enable GitHub auto-merge later) |

## Flow

1. Land conventional commits on `main` (`feat:`, `fix:`, `docs:`, …).
2. `release-plz` opens/updates a **Release PR** (version bump + `CHANGELOG.md`).
3. Merge the Release PR when ready.
4. `release-plz` publishes to crates.io (if `CARGO_REGISTRY_TOKEN` is set) and pushes tag `vX.Y.Z`.
5. `dist`’s `release.yml` builds archives + shell/PowerShell installers and attaches them to the GitHub Release.

`git_release_enable = false` in `release-plz.toml` so **dist** owns the GitHub Release page.

## One-time GitHub setup

1. **Actions → General → Workflow permissions**: allow GitHub Actions to create and approve pull requests.
2. Secret **`RELEASE_PLZ_TOKEN`**: fine-grained PAT with **Contents** + **Pull requests** read/write on this repo. Required so the tag from release-plz can trigger `release.yml` (the default `GITHUB_TOKEN` cannot start other workflows).
3. Secret **`CARGO_REGISTRY_TOKEN`**: crates.io token with `publish-new` + `publish-update`.  
   Alternative: crates.io [trusted publishing](https://crates.io/docs/trusted-publishing) (OIDC) — first publish of a brand-new crate still must be done manually once.

Without (2), release PRs still work with `GITHUB_TOKEN`, but binary releases will not auto-start after tagging.

## Local checks

```bash
# Preview what dist would ship for the current version
dist plan

# Preview unreleased changelog
git cliff --unreleased

# Apply cargo-wizard profile templates again (already applied)
cargo wizard apply fast-compile dev --nightly=off
cargo wizard apply fast-runtime release --nightly=off
# Then remove any `.cargo/config.toml` rustflags with target-cpu=native
# before committing — that flag is not portable for distributed binaries.
```

## Profiles

See the README “Build & run” section. Summary:

- `dev` → fast rebuilds (`debug = 0`)
- `release` → fast runtime (`lto`, `codegen-units = 1`, `panic = "abort"`)
- `dist` → CI packaging profile (`inherits = "release"`, `lto = "thin"`)
