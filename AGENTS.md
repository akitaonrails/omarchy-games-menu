# AGENTS.md

## Repo shape

- Rust workspace at the root; binary is `ogm` (crate `ogm-cli`).
- `crates/ogm-core`: pure domain logic, no I/O/async — desktop-entry parsing,
  catalog, sort/filter, frecency, version compare, reconcile. Keep it I/O-free.
- `crates/ogm-store`: XDG persistence (config.toml, games.json, state.json,
  catalog.d fragments), atomic writes.
- `crates/ogm-net`: GitHub releases + SteamGridDB clients behind async traits;
  parse logic in standalone `parse_*_response` fns with inline fixtures.
- `crates/ogm-cli`: clap subcommands, the only crate that touches the network
  and the process table.
- `catalog.toml` (root): curated game metadata, bundled via `include_str!` —
  editing it requires a rebuild.
- `qml/`: QuickShell 0.3.1 frontend, run via `./games-menu` (dev) or the
  packaged `/usr/share/omarchy-games-menu/qml`.
- `packaging/PKGBUILD`: AUR source package template; the AUR Publish workflow
  sed-fills `pkgver`/`sha256sums` from the release tag tarball.

## Commands

- `cargo test --workspace` — 146+ tests, no network required.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and
  `cargo fmt --all --check` — both enforced in CI.
- QML lint: `/usr/lib/qt6/bin/qmllint -I /usr/lib/qt6/qml qml/*.qml` — the six
  remaining `id-shadows-member` warnings (`Theme { id: theme }` vs the `theme`
  property on GameGrid/GameCard) are known and shared with the production
  omarchy shell.
- Local install: `./install.sh` (symlinks into `~/.local/bin`).
- Preview QML against a fixture: see the header comment in `qml/shell.qml`.

## Hard contracts (do not break casually)

- `state.json` schema is the API between backend and QML; a serialization
  contract test locks it. Adding fields is fine (QML is null-safe); renaming
  or changing types is not.
- Frecency decay steps (≤7d: 1.0, ≤30d: 0.6, ≤90d: 0.3, older: 0.1) are
  mirrored in `qml/shell.qml` `frecencyScore` — change both together.
- UI prefs (`prefs.json`) are owned by the QML side; the backend must never
  write that file. `state.json` is the reverse: backend-owned.
- Controller input: `ogm gamepad` streams one JSON edge per line
  (`{"type":"dir",...}` / `{"type":"button",...}`) and the overlay parses it
  with a SplitParser. Devices are found by capability in
  `/proc/bus/input/devices` (EV_ABS + BTN_SOUTH), never by udev symlink — a
  ZSA Moonlander exposes `-event-joystick` without being a pad. Buttons are
  positional per the kernel gamepad spec, so on an Xbox-layout pad BTN_NORTH
  is Y. Direction *edges* only: the overlay owns key-repeat so the stick,
  D-pad and arrow keys share one acceleration ramp. A pad that vanishes
  mid-hold gets an explicit release, or the cursor repeats forever.
  Bindings: A/Start play · B quit · LB/RB category · X sort direction ·
  Y sort key · LT/RT cover size · Select refresh (re-fetches any missing
  cover, same as the `ogm refresh` the overlay runs on open).
- Processes in `shell.qml` set `command` then `running` imperatively. A
  declarative `running: true` races the `command` binding and can relaunch
  the child mid-stream, which silently drops input edges.
- `ogm refresh` is spawned detached (`setsid -f`) by the QML side: a plain
  child Process is killed when the overlay closes, which truncated SGDB cover
  fetches for newly added games. The backend takes `cache/refresh.lock` so
  overlapping overlay launches can't race, and saves `state.json` after each
  cover so a killed refresh keeps its progress instead of orphaning files.
- External tools integrate via `~/.config/ogm/catalog.d/*.toml` fragments
  (same `[[game]]` schema as `catalog.toml`) + `ogm scan`. distrobox-gaming's
  `ogm_catalog` role is the reference consumer.
- Discovery: `.desktop` files with `X-OGM-Managed=true` are primary
  (`X-OGM-Category`/`X-OGM-GitHub`/`X-OGM-SGDBQuery` override catalog data for
  that file); the `desktop_globs` stem matching in config.toml is the legacy
  fallback and must keep working until distrobox-gaming fully migrates.
- Web-page update checks: `X-OGM-WebURL`/`X-OGM-UpdateURL`/`X-OGM-UpdateRegex`
  (also on catalog/custom entries) produce `state.json`'s `web` block. First
  poll is a baseline; `has_update` means "page changed since last launch" and
  only `ogm launch` clears it. No regex → fingerprint = ETag > Last-Modified >
  sha256(body); with regex, capture group 1 (else whole match) is the version.

## Conventions

- No network or filesystem I/O in `ogm-core` — that's what makes the rest
  testable. New logic goes there first.
- Comments only for non-obvious logic; no doc churn.
- Never commit state.json, prefs.json, covers, or API keys. The SteamGridDB
  key lives in `~/.config/ogm/config.toml` on user machines only.

## Releasing

Push a `vX.Y.Z` tag: the Release workflow runs tests, then creates the GitHub
release (notes from `releases/<tag>.md` when present); the AUR Publish
workflow then renders `packaging/PKGBUILD` with the tag's tarball sha256 and
pushes to AUR. AUR credentials live in repo secrets (`AUR_USERNAME`,
`AUR_EMAIL`, `AUR_SSH_PRIVATE_KEY`).
