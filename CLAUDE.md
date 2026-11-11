# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

SecurePass is a desktop password manager: Tauri 2 (Rust backend in `src-tauri/`) + SvelteKit / Svelte 5 runes frontend (`src/`), with Google Drive sync. `AGENTS.md` holds the original project plan; where it disagrees with this file (e.g. `npm test`, integration tests, per-entry nonces), the code matches this file.

## Commands

```bash
npm install
npm run tauri dev        # full app: Vite on fixed port 1420 (strictPort) + Rust hot-rebuild
npm run dev              # frontend only — every invoke() call fails without the Rust side
npm run tauri build      # production bundles (dmg/msi/nsis/appimage/deb)
npm run check            # svelte-kit sync + svelte-check (this is also what `npm run lint` runs)
npm run build            # static frontend build into /build

# Rust — run from src-tauri/
cargo test               # unit tests live in #[cfg(test)] modules in crypto.rs and vault.rs
cargo test test_encrypt_decrypt   # single test
cargo clippy
cargo fmt
```

There is no frontend test runner and no Rust integration-test directory. CI (`.github/workflows/test.yml`) runs on pushes and PRs to `main`: `npm run check` + `npm run build`, `cargo test --locked` + `cargo clippy --all-targets -- -D warnings`, and `cargo audit` + `npm audit --omit=dev --audit-level=high`. Keep clippy warning-free.

## Releasing

`./scripts/release.sh <version> "<message>"` bumps the version in **four** places (`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and the `securepass` entry in `src-tauri/Cargo.lock`), commits, tags `v<version>` and pushes. Keep all four in sync if you bump by hand (CI builds with `--locked`): `check_for_update` compares `CARGO_PKG_VERSION` against `version.json` on the website. The tag triggers `release.yml`, which builds on macOS, Windows and Linux, then pushes the binaries plus `release-data.js` / `version.json` into the separate `securepass-web` repo and keeps only the two newest releases there.

## Architecture

**Frontend → backend boundary.** Every operation is a `#[tauri::command]` in `src-tauri/src/commands.rs`, registered in `lib.rs`'s `generate_handler!`. A new command must be added in both places. Most commands take one `request` struct with `#[serde(rename_all = "camelCase")]`, so the frontend calls `invoke("cmd", { request: { entryId, masterPassword, ... } })`. Errors come back as `Result<_, String>`.

**State.** A single `Mutex<AppState>` (std mutex) holds the unlocked `Vault`, the `CryptoManager`, and pending sync-conflict state. Async commands that do network I/O copy what they need (`get_key_bytes()`, a vault clone) out of the lock, drop it, `await`, then lock again to write back. Never hold the guard across an `.await`.

**Encryption model (`crypto.rs`, `commands.rs` `save_vault` / `load_vault`).** The whole vault is serialized to JSON and encrypted as one ChaCha20-Poly1305 blob with AAD `b"vault"`. Key derivation is Argon2id. File layout: `[32-byte salt][12-byte nonce][ciphertext]`, and the salt also lives in `Vault.salt`. Every save writes a fresh nonce. Despite its name, `Field.value_encrypted` holds **plaintext** bytes in memory (`Field::encrypt_value` is dead code); read values with `get_plaintext_value()`. The same byte format is used for `~/.securepass/vault.dat`, `.spvault` exports, and the Drive file (`sync::encrypt_vault_bytes` / `decrypt_vault_bytes`).

**Sync (`sync.rs`).** Sync uses an OAuth installed-app flow with a local redirect listener (`drive.file` scope). Tokens are stored in `~/.securepass/cloud_tokens.json` and `last_sync` / `drive_file_id` in `sync_state.json`. `merge_vaults` does an entry-level 3-way merge using `last_sync` as the base:
- An entry changed on both sides since `last_sync` becomes a conflict.
- Deletions propagate through tombstones (`Vault.deleted_entries`, a union of both sides). `delete_entry` must leave a tombstone, not just remove the entry.
- `local_contributed` decides whether to upload. Don't upload when it is false: that bumps Drive `modifiedTime` and makes other devices show a spurious "sync needed".

When there are conflicts, `sync_vault` stashes `pending_sync_merge`, `pending_conflicts` and `pending_drive_file_id` in `AppState`. `ConflictResolutionModal.svelte` then calls `resolve_conflicts` to finish and upload. If decryption of the Drive file fails with the current key, the master password was changed on another device: the UI prompts, and `reauth_and_sync` derives the new key from the new password plus the **salt in the Drive file**, so both devices end up with the same key.

**Frontend.** `src/routes/+page.svelte` owns locked/unlocked state and the auto-lock timer. `Dashboard.svelte` is the main shell and owns all sync, import/export and re-auth logic. Its layout is three panes:
- `Sidebar.svelte`: search, categories, the sync card, and the Import/Export/Settings/Lock buttons.
- `EntryList.svelte`: the list of entries.
- The detail pane: shows `EntryView` or `EntryForm` inline (not as modals). `PasswordGenerator` is inline under password fields.

SSR is off (`+layout.ts`), and adapter-static builds an SPA that Tauri loads from `../build`.

**Theming.** All colours are CSS variables in `src/app.css`, imported by `+layout.svelte`; components must use the variables, not hex values. Shared classes (`.btn`, `.input`, `.modal`, `.msg`, …) also live there. Light/dark switching:
- The preference (system/light/dark, saved as localStorage `sp_theme`) is handled by `src/lib/theme.ts` and becomes `<html data-theme>`.
- An inline script in `src/app.html` applies it before first paint.
- The dark tokens appear twice in `app.css`, once for `[data-theme="dark"]` and once for `prefers-color-scheme`. Keep both blocks in sync.

**App icon.** The source is `src-tauri/icons/app-icon.svg`; its header comment has the regenerate steps. Cargo doesn't rebuild when only icon files change, so run `cargo clean -p securepass` (from `src-tauri/`) before `tauri dev`/`build`, or the program keeps the old icon.

Category labels and colours come from `src/lib/categories.ts`. Icons are `Icon.svelte` (inline SVG, no emoji). The CSP blocks external fonts and styles, so don't add CDN links.

## Conventions

- Never log field values, master passwords or derived keys. Changes to `crypto.rs` or the on-disk format affect existing vaults on users' disks and on Drive, so keep them backward compatible (for example, new `Vault` fields need `#[serde(default)]`).
- Commit messages use conventional prefixes (`feat:`, `fix:`).
