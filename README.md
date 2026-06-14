# 🔐 SecurePass - Encrypted Password Manager

> Your passwords. Your device. Your cloud.

A secure, cross-platform password manager built with Rust and Svelte, featuring military-grade encryption, Google Drive sync, and a modern intuitive interface.

![SecurePass](https://img.shields.io/badge/SecurePass-1.1.12-blue?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-1.91.1-orange?style=flat-square)
![Svelte](https://img.shields.io/badge/Svelte-5.0-orange?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2.0-purple?style=flat-square)

---

## ✨ Features

### 🔒 Security First
- **Military-grade encryption** using ChaCha20-Poly1305 AEAD
- **Argon2id key derivation** (64 KiB memory, 3 iterations) for master password hashing
- **Secure memory handling** with automatic key zeroization on lock
- **Zero plaintext storage** — passwords never written unencrypted to disk
- **Change master password** — re-derives a new key with a fresh salt and re-encrypts the entire vault

### ☁️ Google Drive Sync
- **Personal Drive storage** — vault file lives in your own Google Drive (`drive.file` scope only; SecurePass cannot access other files)
- **Entry-level 3-way merge** — changes made independently on two devices are merged automatically; only true conflicts are surfaced to you
- **Tombstone-based deletion** — entries deleted on one device are correctly removed on the other during the next sync
- **Sync needed indicator** — the sync button pulses amber when local or remote changes are detected
- **Conflict resolution UI** — side-by-side view of conflicting versions; pick which to keep per entry
- **Restore from Google Drive** — first-launch on a new device can pull the vault from Drive without needing the original device

### 🔄 Cross-Device Password Change
- When the master password is changed on Device A and Device B tries to sync, SecurePass detects the key mismatch and prompts for the new password
- Device B derives the new key using the **same salt** embedded in the Drive file (ensuring both devices use an identical key), merges any local-only changes, and re-saves locally — no manual file copying needed

### 🎯 Core Functionality
- **Password vault management** with categorized entries
- **Secure password generation** with customizable length and character sets
- **Auto-lock timer** with configurable timeout (1–60 minutes)
- **Clipboard security** with automatic clearing (5–300 seconds)
- **Export / Import** encrypted vault backups (`.spvault`)
- **Import from PasswordLocker** — migrate entries from JSON exports

### 🎨 User Experience
- **Modern, responsive UI** built with Svelte 5 (runes)
- **Category-based templates** with sensible field defaults per entry type
- **Real-time search and category filtering**
- **Custom password fields** displayed masked in entry view (reveal on click)
- **Keyboard shortcuts** for power users
- **Cross-platform** — macOS, Windows, Linux

### 📁 Entry Categories
| Category | Fields |
|----------|--------|
| **Web / App** | URL, Username, Password, Recovery Email |
| **SSH / Server** | Hostname, Port, Username, Password, Private Key |
| **Bank / Financial** | Bank Name, Username, Password, Account Number |
| **Email** | Email, Password, IMAP/SMTP servers |
| **Wi-Fi** | SSID, Password, Network Type |
| **Generic** | Fully customisable fields |

---

## 🚀 Quick Start

### Prerequisites
- **Node.js** 18+ ([Download](https://nodejs.org/))
- **Rust** 1.70+ ([Install](https://rustup.rs/))

### Installation

```bash
git clone <repository-url>
cd securepass
npm install
npm run tauri dev
```

### Build for Production
```bash
npm run tauri build
```

---

## 📖 Usage Guide

### First Time Setup
1. Launch SecurePass
2. Create a strong master password (8+ characters)
3. Your encrypted vault is created at `~/.securepass/vault.dat`

### Creating Entries
1. Click **➕ New Entry** or press **Ctrl+N**
2. Choose a category — fields pre-populate for that type
3. Use the **🔐** generator button to create a strong password
4. Save

### Password Generator
- Length 8–64 characters
- Toggle uppercase, lowercase, numbers, symbols
- Exclude visually similar characters (0/O, 1/l/I)
- Real-time strength indicator

### Google Drive Sync

#### Connect (first time)
1. Open **⚙️ Settings → Cloud Storage**
2. Click **Connect Google Drive** — your browser opens for sign-in
3. After authorising, click **Sync Now** or use the sync button in the header

#### Sync button states
| State | Meaning |
|-------|---------|
| ☁️ _X min ago_ | Synced, no changes detected |
| ↑↓ Sync needed 🔴 | Local or remote changes detected |
| ⟳ Syncing… | Sync in progress |

#### Restore on a new device
1. On the new device, open SecurePass (no vault yet)
2. Click **☁️ Restore from Google Drive** on the unlock screen
3. Complete Google sign-in in your browser
4. Enter your master password — vault is downloaded and saved locally

#### After changing the master password
When Device A changes the master password and Device B next syncs:
1. SecurePass detects the key mismatch automatically
2. A prompt appears: **"Master Password Changed"**
3. Enter the new master password — Device B re-derives the key using the Drive vault's salt, merges any local changes, and saves with the new key

### Change Master Password
1. Open **⚙️ Settings → Change Master Password**
2. Enter current password, new password, and confirmation
3. Click **Change Password** — the vault is re-encrypted locally and re-uploaded to Drive if connected

### Backup & Restore
- **Export**: Header → **📤 Export** → saves `.spvault` file
- **Import**: Header → **📥 Import** → supports `.spvault` and PasswordLocker `.json`

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| Ctrl/Cmd+N | New entry |
| Ctrl/Cmd+L | Lock vault |
| Escape | Close modal |

---

## 🛠️ Development

### Project Structure
```
securepass/
├── src/                        # Frontend (Svelte 5)
│   ├── components/
│   │   ├── Dashboard.svelte        # Main shell, sync button, re-auth modal
│   │   ├── EntryList.svelte        # Grid grouped by category
│   │   ├── EntryForm.svelte        # Create / edit with category templates
│   │   ├── EntryView.svelte        # Read-only detail, masked password fields
│   │   ├── PasswordGenerator.svelte
│   │   ├── SearchBar.svelte
│   │   ├── Settings.svelte         # Auto-lock, clipboard, change password, Drive
│   │   ├── ConflictResolutionModal.svelte
│   │   └── ConfirmDialog.svelte
│   └── routes/
│       ├── +page.svelte            # Root: unlock/lock state, auto-lock timer
│       └── +layout.ts              # ssr = false (SPA mode)
├── src-tauri/
│   └── src/
│       ├── commands.rs             # All Tauri command handlers + AppState
│       ├── crypto.rs               # CryptoManager (Argon2id + ChaCha20-Poly1305)
│       ├── sync.rs                 # OAuth, Drive API, 3-way merge, tombstones
│       ├── vault.rs                # Vault / Entry / Field data structures
│       ├── lib.rs                  # Tauri builder + command registration
│       └── main.rs                 # Entry point
└── package.json
```

### Available Scripts

```bash
# Development
npm run tauri dev          # Full dev server (Rust + frontend hot-reload)
npm run dev               # Frontend only (invoke() calls will fail)

# Building
npm run tauri build       # Production build
cargo build --release     # Rust backend only (run from src-tauri/)

# Code Quality
cargo clippy              # Rust linting
cargo fmt                 # Rust formatting
npm run check             # svelte-check + tsc

# Testing
cargo test                # Rust unit tests
cargo audit               # Audit Rust deps for vulnerabilities
npm audit                 # Audit npm deps
```

---

## 🔐 Security Details

### Encryption
| Property | Value |
|----------|-------|
| Algorithm | ChaCha20-Poly1305 (AEAD) |
| Key derivation | Argon2id — 64 KiB memory, 3 iterations, 1 thread |
| Salt | 32 bytes, random, stored in vault file header |
| Nonce | 12 bytes, random per save |
| Key size | 256 bits |
| AAD | `b"vault"` |

### Vault File Format
```
[32-byte Argon2 salt][12-byte nonce][ChaCha20-Poly1305 ciphertext]
```
The entire JSON-serialised vault is encrypted as one unit. The salt is embedded in the file so any device can re-derive the correct key given the master password.

### Data Storage
- **Vault**: `~/.securepass/vault.dat`
- **Drive tokens**: `~/.securepass/cloud_tokens.json`
- **Sync state**: `~/.securepass/sync_state.json`
- **Backup format**: `.spvault` (same binary format as `vault.dat`)

### Memory Security
- `CryptoManager::zeroize_key()` zeros the in-memory key bytes on lock
- Master password and derived key are never written to disk in plaintext
- Clipboard is cleared automatically after the configured timeout

### Google Drive Security
- OAuth 2.0 installed-app flow with local redirect listener (5-minute timeout)
- `drive.file` scope only — SecurePass can only access files it created
- Tokens stored locally in `~/.securepass/cloud_tokens.json`
- The vault file on Drive is fully encrypted; Google cannot read its contents

---

## ⚠️ Security Notices

- **Never share your master password** — there is no recovery mechanism
- **Keep backup files secure** — `.spvault` files contain your encrypted data
- **Use a strong, unique master password** — this is the only key to your vault
- **Enable auto-lock** on shared or unattended devices
- **Back up regularly** — export a `.spvault` file or use Google Drive sync

---

## 🆘 Troubleshooting

**`cargo` not found**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Port 1420 already in use**
```bash
lsof -ti:1420 | xargs kill -9
```

**Build fails on macOS**
```bash
xcode-select --install
```

**Google Drive sign-in window did not appear**
Make sure your default browser is set and not blocking pop-ups. The OAuth listener times out after 5 minutes.

**"Incorrect master password" after syncing from another device**
The sync button will detect this and show the **Master Password Changed** prompt. Enter the new password to re-key this device automatically.

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes
4. Push and open a Pull Request

**Guidelines:**
- Follow Rust and TypeScript best practices
- Never log field values, master passwords, or derived keys
- Run `cargo clippy` and `npm run check` before submitting
- Any change to `crypto.rs` requires a security review

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

**Built with Rust, Svelte, and Tauri**
