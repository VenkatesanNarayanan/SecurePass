# SecurePass

> Your passwords. Your device. Your cloud.

A cross-platform desktop password manager built with Rust, Svelte and Tauri. Your vault is encrypted on your device and, if you want, synced through your own Google Drive — no account, no subscription, no SecurePass server.

[![Latest release](https://img.shields.io/github/v/release/VenkatesanNarayanan/SecurePass?label=release)](https://github.com/VenkatesanNarayanan/SecurePass/releases/latest)
[![Test](https://github.com/VenkatesanNarayanan/SecurePass/actions/workflows/test.yml/badge.svg)](https://github.com/VenkatesanNarayanan/SecurePass/actions/workflows/test.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

**Download:** [securepass.wonderstek.com](https://www.securepass.wonderstek.com/#download) — macOS, Windows and Linux.

---

## Features

### Security
- **Authenticated encryption** — the whole vault is encrypted with ChaCha20-Poly1305
- **Argon2id key derivation** (64 MiB memory, 3 iterations) from your master password
- **Nothing stored unencrypted** — entries are never written to disk in plain text; the master password is never stored
- **Key wiped on lock** — locking overwrites the encryption key and drops the decrypted vault (see [Memory](#memory) for the limits)
- **Auto-lock** after inactivity (1–60 minutes) and **clipboard auto-clear** after copying (5–300 seconds)
- **Change master password** — derives a new key with a fresh salt and re-encrypts the vault

### Google Drive sync
- **Your own Drive** — the encrypted vault file lives in your Google Drive, using the `drive.file` scope (SecurePass can only see files it created)
- **Entry-level 3-way merge** — edits made on different devices merge automatically; only real conflicts are shown to you, side by side
- **Deletions sync** — entries deleted on one device are removed on the others
- **Restore on a new device** — pull your vault straight from Drive on first launch
- **Cross-device password change** — after changing the master password on one device, the others ask for the new password on their next sync and re-key themselves

### Everyday use
- Three-pane layout: categories and sync status, entry list, and entry details side by side
- **Category templates** (Web & Apps, SSH & Servers, Banking, Email, Wi-Fi, Generic) plus custom text, secret and long-text fields
- **Search** and sorting by **recently used**, recently updated or A–Z
- **Password generator** built into every password field, with a strength meter and copy button
- Per-field copy and reveal; links open in your browser
- **Light and dark themes** (follows your system, or choose one)
- **Export / import** encrypted `.spvault` backups, and import from PasswordLocker JSON exports

### Keyboard shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl/Cmd + K or F | Focus search |
| Ctrl/Cmd + N | New entry |
| Ctrl/Cmd + L | Lock vault |
| ↑ / ↓ | Move through the entry list |
| Escape | Cancel editing, clear search, or close a dialog |

---

## Getting started (users)

1. Download the installer for your platform from the [website](https://www.securepass.wonderstek.com/#download) (installation notes for each platform are there too — the builds are not code-signed yet).
2. Launch SecurePass and create a master password. Your encrypted vault is created at `~/.securepass/vault.dat`.
3. Optional: **Settings → Google Drive sync → Connect** to sync with your other devices. On a new device, choose **Restore from Google Drive** on the first screen.

There is **no master-password recovery**. Keep an exported `.spvault` backup somewhere safe.

---

## Development

### Prerequisites
- **Node.js** 18+ (CI uses 22)
- **Rust** stable, via [rustup](https://rustup.rs/)
- **Windows:** Visual Studio or Build Tools with the *Desktop development with C++* workload (Rust needs its linker); WebView2 ships with Windows 10/11
- **macOS:** `xcode-select --install`
- **Linux:** `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` and `build-essential` (see `.github/workflows/release.yml` for the full list)

### Run it
```bash
git clone https://github.com/VenkatesanNarayanan/SecurePass.git
cd SecurePass
npm install
npm run tauri dev
```

### Commands
```bash
npm run tauri dev        # full app with hot reload (Vite on port 1420)
npm run tauri build      # production installers
npm run check            # svelte-check + TypeScript

# from src-tauri/
cargo test               # Rust unit tests (includes a known-answer test for key derivation)
cargo clippy             # Rust lints
cargo audit              # dependency advisories (cargo install cargo-audit)
```

`npm run dev` starts only the frontend; every backend call fails without Tauri.

### Project structure
```
src/                         Frontend (SvelteKit SPA, Svelte 5 runes)
├── app.css                  Theme tokens (light/dark) and shared styles
├── lib/                     theme, categories, clipboard, strength, recently-used helpers
├── components/              Dashboard (shell), Sidebar, EntryList, EntryView, EntryForm,
│                            PasswordGenerator, Settings, VaultUnlock, dialogs, Icon
└── routes/                  +page.svelte (lock state, auto-lock), +layout.svelte (theme)
src-tauri/src/
├── commands.rs              All Tauri commands + AppState; vault file read/write
├── crypto.rs                Argon2id key derivation, ChaCha20-Poly1305
├── sync.rs                  Google OAuth, Drive API, 3-way merge, tombstones
├── vault.rs                 Vault / Entry / Field data model
└── lib.rs                   Tauri builder, command registration, window placement
```

### Releasing
`./scripts/release.sh <version> "<message>"` bumps the version in `package.json`, `tauri.conf.json` and `Cargo.toml`, commits, tags `v<version>` and pushes. The tag triggers the Release workflow, which builds every platform and publishes the installers to the website.

---

## Security design

### Encryption
| Property | Value |
|----------|-------|
| Cipher | ChaCha20-Poly1305 (AEAD), 256-bit key |
| Key derivation | Argon2id v1.3 — 64 MiB memory, 3 iterations, 1 lane |
| Salt | 32 random bytes per vault, stored in the file header; a new salt on every master-password change |
| Nonce | 12 random bytes, new on every save |
| AAD | `b"vault"` |

The Argon2 parameters are fixed in `crypto.rs` and are not stored in the vault file, so they can't change without a vault-format migration; a known-answer test guards them.

### Vault file format
```
[32-byte Argon2 salt][12-byte nonce][ChaCha20-Poly1305 ciphertext of the JSON vault]
```
The whole vault is encrypted as one unit (one nonce per save, not per entry). The same format is used for `~/.securepass/vault.dat`, `.spvault` exports and the Google Drive copy, so any device can derive the key from the master password and the salt in the file.

### What is stored where
| File | Contents | Encrypted |
|------|----------|-----------|
| `~/.securepass/vault.dat` | Your vault | Yes |
| `~/.securepass/cloud_tokens.json` | Google OAuth tokens (only if Drive is connected) | No — readable by your user account; deleted when you disconnect Drive |
| `~/.securepass/sync_state.json` | Drive file ID and last sync time | No (contains no secrets) |
| App local storage | Theme, timers, sort order, generator options, recently-used entry IDs and times | No (contains no secrets) |

### Memory
While the vault is unlocked, its decrypted contents are held in ordinary process memory. Locking (manually or by auto-lock) overwrites the encryption key and drops the decrypted vault, but the freed memory is not explicitly zeroed, and temporary key copies made during sync are not wiped either. Treat an unlocked SecurePass on a compromised machine as exposed — no password manager can fully protect against malware running as your user.

### Clipboard
Copied values are cleared from the clipboard after the configured timeout. Operating-system clipboard history (for example Windows **Win+V** history or cloud clipboard) may still keep a copy; turn those off if that matters to you.

### Google Drive
- OAuth 2.0 installed-app flow in your browser, with a local redirect listener (5-minute timeout)
- `drive.file` scope only — SecurePass can access only the files it created
- Google sees only the encrypted vault file
- The OAuth **client secret in `sync.rs` is intentionally public**: for desktop ("installed") apps Google does not treat it as confidential, because it can't be kept secret in a distributed binary. It identifies the app; it grants no access to anyone's data without that user signing in.

### Network
The app contacts only Google (when Drive is connected) and `https://www.securepass.wonderstek.com/version.json` for the update check. No telemetry, analytics or crash reporting. See the [privacy policy](https://www.securepass.wonderstek.com/privacy.html).

### Reporting a vulnerability
Please **don't** open a public issue. See [SECURITY.md](SECURITY.md).

---

## Troubleshooting

**`cargo` not found** — install Rust with [rustup](https://rustup.rs/), then open a new terminal.

**Windows: `link.exe` not found / linker errors** — add the *Desktop development with C++* workload in Visual Studio Installer (Modify), then open a new terminal.

**Port 1420 already in use** — another `tauri dev` or Vite server is running; stop it.

**App icon didn't update after changing `src-tauri/icons`** — Cargo doesn't rebuild for icon-only changes; run `cargo clean -p securepass` in `src-tauri/`.

**Google sign-in window didn't appear** — check that a default browser is set. The sign-in listener times out after 5 minutes.

**"Master Password Changed" prompt after syncing** — the master password was changed on another device; enter the new one to re-key this device.

---

## Contributing

Issues and pull requests are welcome.

- Run `npm run check`, `cargo test` and `cargo clippy` before opening a PR.
- Never log field values, master passwords or keys.
- Changes to `crypto.rs` or the vault file format affect every existing vault — explain the migration in the PR, and expect a careful review.

## License

[MIT](LICENSE)
