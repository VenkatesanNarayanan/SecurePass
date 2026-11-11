# Remaining work

Open items as of 2026-11-11 (after the v1.1.21 release). Check an item off or delete it when it's done.

## State at hand-off

- `main` is at the release-script fix and its docs update; CI (frontend, Rust test + clippy, dependency audit) passes.
- **v1.1.21 is live** on the website (macOS, Windows, Linux). It contains the 10 RustSec dependency fixes (rustls, rustls-webpki, h2, bytes, time, quick-xml) and the entry-list spacing fix. `version.json` says 1.1.21.
- GitHub settings now on: branch protection for `main` (Frontend and Rust checks required for PRs; no force-push or deletion; admins can still push directly), Dependabot alerts and security-update PRs, secret scanning with push protection, private vulnerability reporting.
- The two secret-scanning alerts for the Google OAuth client ID and secret in `sync.rs` were closed as "won't fix": the secret is public by design for desktop apps (see README, *Security design → Google Drive*).
- `scripts/release.sh` now bumps the version in four places, including the `securepass` entry in `src-tauri/Cargo.lock`. CI builds with `--locked`, so a stale lock fails the build.

## 1. Android app (highest priority)

The Android build was made on another laptop. Its project (`src-tauri/gen/android`) is not in this repo, and the release workflow builds no APK. Everything since the Android build (redesigned UI, dark mode, recently-used sort, dependency security fixes) reaches the phone only after you rebuild from the current `main`.

- [ ] **Make the start-up window code desktop-only.** `fit_to_work_area` in `src-tauri/src/lib.rs` sizes and centres a desktop window using the monitor's work area, and `tauri.conf.json` now creates the window hidden (`"visible": false`) and shows it after placing it. On Android, wrap the `fit_to_work_area` function and its call in `#[cfg(desktop)]`, and check that the window still appears (`window.show()` after the hidden start). If not, use a mobile-specific config (`tauri.android.conf.json`) with `"visible": true`.
- [ ] **Phone layout.** The redesign is a three-pane desktop layout: `Sidebar`, `EntryList` and the detail pane side by side in `Dashboard.svelte`, with `minWidth` 800 in `tauri.conf.json` and only a few breakpoints (1100/1000/860/700 px). Phones need one pane at a time: entry list → entry view/form with a back button, and the sidebar as a slide-out menu. Keep using the CSS variables in `src/app.css`.
- [ ] **Check the desktop-only pieces on Android:**
  - Google sign-in uses a local redirect listener (`sync.rs`). It may not work on Android and may need a custom-scheme / app-link redirect or a different OAuth client type.
  - Clipboard copy/clear (`commands::copy_to_clipboard`, `clear_clipboard`).
  - File dialogs for export/import (`tauri-plugin-dialog`, `tauri-plugin-fs`).
  - Keyboard shortcuts and hover-only controls.
- [ ] **Commit the Android project.** Run `npm run tauri android init` (or copy `src-tauri/gen/android` from the laptop) and commit it. Keep signing keys and `keystore.properties` out of git.
- [ ] **Add an Android job to `.github/workflows/release.yml`** that builds a signed APK/AAB, with the keystore in GitHub Actions secrets. Add an Android download to the website.
- [ ] **Update check on Android.** `check_for_update` reads the same `version.json` and links to `#download`, which has no APK yet. Either publish the APK there or hide the banner on mobile.

## 2. App security improvements

Bundle these into one release, 1.1.22. All are documented as known limitations in the README and `SECURITY.md`; update those sections when you fix them.

- [ ] **Clipboard history (Windows):** mark copied secrets so Win+V history and cloud clipboard skip them (the `ExcludeClipboardContentFromMonitorProcessing` / `CanIncludeInClipboardHistory` clipboard formats).
- [ ] **Clear memory on lock:** zeroize decrypted entries and the temporary key copies made during sync (`get_key_bytes()` in `commands.rs` / `sync.rs`). The `zeroize` crate is the usual tool.
- [ ] **Drive tokens in the OS keychain** instead of the unencrypted `~/.securepass/cloud_tokens.json` (the `keyring` crate). Migrate existing token files and delete them.

## 3. Release pipeline

- [ ] **SHA-256 checksums** for every installer, published with the release and on the website (the builds aren't code-signed, so a checksum is the only way users can check a download).
- [ ] **Code signing** (Windows Authenticode, Apple notarization) once there's a budget for certificates.

## 4. Repo housekeeping

- [ ] Bump `actions/checkout` and `actions/setup-node` from `@v4` to versions that run on a supported Node (GitHub warns that Node 20 is deprecated).
- [ ] Optional: `.github/dependabot.yml` with weekly version updates for `github-actions` (Dependabot currently opens PRs only for security fixes).
- [ ] `CONTRIBUTING.md` and issue templates (bug report, feature request).
- [ ] `cargo audit` reports 17 "unmaintained" warnings (`fxhash`, the GTK3 bindings Tauri uses on Linux). They aren't vulnerabilities and go away when Tauri updates them; recheck after Tauri upgrades.

## 5. Website (`securepass-web` repo)

- [ ] A "What's new" section for the latest release.
- [ ] A 1200×630 Open Graph image, so shared links show a proper preview card.
- [ ] An Android download once the APK is built in CI (see 1).

## Notes for the other laptop

- Pull `main` first. If you have unpushed Android work there, rebase it onto `main`: the frontend was largely rewritten (see `CLAUDE.md` for the new component layout and theming rules).
- Pushing changes under `.github/workflows/` needs a token with the `workflow` scope (`gh auth refresh -h github.com -s workflow`).
- To release: `./scripts/release.sh <version> "<message>"`.
