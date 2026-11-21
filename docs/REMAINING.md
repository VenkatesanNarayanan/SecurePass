# Remaining work

Open items as of 2026-11-11 (after the v1.1.21 release). Check an item off or delete it when it's done.

## State at hand-off

- `main` is at the release-script fix and its docs update; CI (frontend, Rust test + clippy, dependency audit) passes.
- **v1.1.21 is live** on the website (macOS, Windows, Linux). It contains the 10 RustSec dependency fixes (rustls, rustls-webpki, h2, bytes, time, quick-xml) and the entry-list spacing fix. `version.json` says 1.1.21.
- GitHub settings now on: branch protection for `main` (Frontend and Rust checks required for PRs; no force-push or deletion; admins can still push directly), Dependabot alerts and security-update PRs, secret scanning with push protection, private vulnerability reporting.
- The two secret-scanning alerts for the Google OAuth client ID and secret in `sync.rs` were closed as "won't fix": the secret is public by design for desktop apps (see README, *Security design → Google Drive*).
- `scripts/release.sh` now bumps the version in four places, including the `securepass` entry in `src-tauri/Cargo.lock`. CI builds with `--locked`, so a stale lock fails the build.

## 1. Android app (highest priority)

The Android project is in the repo and builds from `main` (`scripts/build-apk.sh`, or `npm run tauri android build -- --debug --target aarch64 --apk true` for a quick emulator build). The release workflow builds no APK yet.

- [x] **Make the start-up window code desktop-only.** `fit_to_work_area` and `set_focus` are `#[cfg(desktop)]`; `window.show()` runs on every platform, and the window appears on Android.
- [x] **Phone layout.** Below 700 px `Dashboard.svelte` shows one pane at a time (list → entry view/form with a back button in a top bar), the search box moves to the top bar, and the sidebar is a slide-out drawer. Each open layer gets a shallow history entry (SvelteKit `pushState`), so Android's back gesture closes the entry or drawer instead of leaving the app. Dialogs and bars respect the safe-area insets (the activity is edge-to-edge).
- [ ] **Check the desktop-only pieces on Android:**
  - [ ] Google sign-in: a mobile flow exists (PKCE + custom-scheme deep link, `sync::start_oauth_flow_mobile`, Chrome forced via `chrome_opener.rs`), but it hasn't been tested since the rebase.
  - [x] Clipboard copy/clear: on Android these go through `android_native.rs`, which marks copies as sensitive (Android 13+ shows dots in the clipboard preview) and clears with `clearPrimaryClip()`.
  - [ ] File dialogs for export/import (`tauri-plugin-dialog`): untested on Android.
  - [x] Keyboard shortcuts and hover-only controls: there are no hover-only controls; the shortcuts are harmless without a keyboard.
- [x] **Autofill.** Android's autofill service offered to save an entry's title and password to Google Password Manager. `MainActivity.kt` now excludes the whole app from autofill.
- [x] **Status bar icons follow the theme.** `theme.ts` calls `set_system_bars_theme` whenever the effective theme changes, so the icons stay visible in light and dark (including a Light/Dark override).
- [x] **Android launcher icon** regenerated from the redesigned icon as an adaptive icon (`src-tauri/icons/android-bg.svg` + `android-fg.svg`; steps in the `app-icon.svg` header).
- [ ] **iOS icons** (`src-tauri/icons/ios/`) still show the old logo; regenerate them when the iOS project is set up (they need a full-bleed, non-transparent square).
- [x] **Commit the Android project.** `src-tauri/gen/android` is in the repo; build outputs, `*.apk` and `.kotlin/` are ignored. No release keystore exists yet (`scripts/build-apk.sh` signs with the debug key).
- [x] **Android job in CI.** `.github/workflows/android.yml` builds an arm64 release APK (~17 MB, under the website's 25 MiB limit) on every tag via `release.yml`, or by hand from the Actions tab. A failed Android build doesn't block the desktop release.
- [ ] **Create the release keystore and add the four `ANDROID_*` secrets** (steps in `docs/ANDROID_RELEASE.md`). Until then the job only uploads an unsigned APK and the website lists no Android download.
- [ ] **Show Android on the website.** `release-data.js` gets an `android` entry once a signed APK exists, but `scripts/downloads.js` in `securepass-web` only renders keys starting with `macos`/`windows`/`linux`, and detects Android phones as Linux (their user agent contains "Linux").
- [x] **Update check on Android.** `version.json` has an `android` section (version + APK link) only once a signed APK is published, and the Android app compares against that section alone, so desktop-only releases don't trigger its banner. iOS never shows the banner (no iOS releases). Android builds from before this change still compare against the desktop version and show the banner until they're updated.

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
- [ ] An Android download: handle the `android` key in `scripts/downloads.js` and detect Android before Linux (see 1).

## Notes for the other laptop

- Pull `main` first. If you have unpushed Android work there, rebase it onto `main`: the frontend was largely rewritten (see `CLAUDE.md` for the new component layout and theming rules).
- Pushing changes under `.github/workflows/` needs a token with the `workflow` scope (`gh auth refresh -h github.com -s workflow`).
- To release: `./scripts/release.sh <version> "<message>"`.
