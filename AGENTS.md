# SecurePass Agent Guidelines

> **Note:** the *Implementation Phases* below are the original project plan. Where they differ from the code, the code and `README.md` ("Security design") are authoritative — for example, the vault is encrypted as one unit with a fresh nonce per save, not per entry.

## Build Commands
- `npm run tauri build` - Build production app for all platforms
- `npm run tauri dev` - Start development server
- `cargo build --release` - Build Rust backend only

## Test Commands
- `cargo test` - Run all Rust tests (from `src-tauri/`; unit tests live in `crypto.rs` and `vault.rs`)
- `cargo test specific_test_name` - Run single Rust test
- There is no frontend test runner and no integration-test suite yet

## Lint Commands
- `npm run lint` / `npm run check` - Type-check the frontend (svelte-check)
- `cargo clippy` - Lint Rust code
- `cargo fmt` - Format Rust code

## Development Environment Setup

### Local Project Setup
**Initialize Tauri Project:**
```bash
npx create-tauri-app@latest . --template svelte-ts
```

**Key Dependencies to Install:**
- **Frontend:** `@tauri-apps/api`, `svelte`, `@types/node`, `vite`, `typescript`, `eslint`
- **Backend:** `tauri`, `serde`, `tokio`, `chacha20poly1305`, `argon2`, `rand`
- **Security:** ChaCha20-Poly1305 AEAD, Argon2id password hashing

**Environment Requirements:**
- Node.js 18+
- Rust 1.70+ (install via: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Tauri CLI installed globally (`npm install -g @tauri-apps/cli`)

**Common Setup Issues:**
- **"cargo command not found"**: Install Rust using: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Permission issues**: Ensure you have proper permissions for the project directory
- **Node.js version**: Use Node.js 18+ (check with: `node --version`)
- **After Rust installation**: Restart your terminal or run `source ~/.cargo/env`
- **Verify installation**: Run `cargo --version` and `rustc --version`

## Testing Commands

### Build & Development
```bash
# Install dependencies
npm install

# Start development server
npm run tauri dev

# Build for production
npm run tauri build

# Build Rust only
cargo build

# Run tests
cargo test        # from src-tauri/
```

### Code Quality
```bash
# Lint Rust code
cargo clippy

# Format Rust code
cargo fmt

# Lint frontend code
npm run lint
```

### Security Audit
```bash
# Audit Rust dependencies
cargo audit

# Audit npm dependencies
npm audit
```

## Implementation Phases

### Phase 1: Project Initialization
1. **Tauri + Svelte Setup**
   - Run: `npx create-tauri-app@latest . --template svelte-ts`
   - Verify: `package.json`, `src/`, `src-tauri/`, `tauri.conf.json` created
   - Test: `npm run tauri dev` should start development server

2. **Dependency Installation**
   - Frontend: `npm install @tauri-apps/api @types/node eslint`
   - Backend: Add to `Cargo.toml`: `serde`, `tokio`, `chacha20poly1305`, `argon2`, `rand`
   - Security audit: `cargo audit` and `npm audit`

### Phase 2: Core Security Infrastructure
1. **Cryptographic Core (Rust)**
   - Implement `crypto.rs`: ChaCha20-Poly1305 encryption/decryption
   - Implement `key_derivation.rs`: Argon2id key derivation from master password
   - Create `vault.rs`: Vault operations (implemented: whole-vault encryption, fresh nonce per save)
   - AAD: implemented as the constant `b"vault"` (per-entry AAD was planned, not built)

2. **Data Models**
   - `Entry` struct: id, category, fields, tags, timestamps
   - `Field` struct: name, type, encrypted_value
   - `Vault` struct: entries, metadata, version

3. **File I/O Operations**
   - Encrypted JSON storage (implemented; SQLite was not used)
   - Secure temp file handling
   - File integrity validation

### Phase 3: User Interface & Core Features
1. **Svelte Components**
   - `VaultUnlock.svelte`: Master password entry
   - `Dashboard.svelte`: Main interface with entries list
   - `EntryForm.svelte`: Dynamic form for creating/editing entries
   - `EntryList.svelte`: Grid display of password entries
   - `SearchBar.svelte`: Search and filtering controls

2. **Category Templates**
   - Web/App: URL, Username, Password, Recovery Email
   - SSH/Server: Hostname, Port, Username, Password/Private Key, Passphrase
   - Bank/Financial: Bank Name, Username, Password, Account Number, Support Number
   - Email: Email Address, Password, IMAP/SMTP servers
   - Wi-Fi: SSID, Password, Network Type
   - Generic: Username, Password, Notes

3. **Search & Filtering**
   - Real-time search by title, username, tags
   - Category filtering
   - Date-based filtering

### Phase 4: Advanced Features & Testing
1. **Security Features**
   - Auto-lock timer implementation
   - Secure clipboard with auto-clear (configurable seconds)
   - Memory zeroing for secrets in Rust (partial: the key is overwritten on lock; decrypted entries are not zeroed)

2. **Cloud Sync (Future)**
   - Encrypted vault file upload/download
   - Conflict detection and resolution
   - Provider-specific APIs (Google Drive OAuth2, iCloud, OneDrive)

3. **Testing & Quality**
   - Unit tests for crypto operations
   - Integration tests for vault operations
   - Security audits and penetration testing

## Code Style Guidelines

### Security First
- Never log sensitive data (passwords, keys, tokens)
- Use secure memory handling (zero out secrets after use)
- Validate all inputs, especially user-provided data
- Use constant-time operations for crypto comparisons

### Rust (Backend)
- Use `Result<T, E>` for error handling
- Prefer `&str` over `String` for read-only strings
- Use `serde` for serialization with derive macros
- Follow Rust naming: `snake_case` for functions/variables, `PascalCase` for types

### Svelte (Frontend)
- Use TypeScript for type safety
- Prefer `const` over `let`, avoid `var`
- Use descriptive component names: `PasswordEntry.svelte`
- Handle async operations with proper error boundaries

### General
- Use meaningful commit messages: "feat: add password generator"
- Keep functions small and focused
- Add unit tests for crypto operations
- Use environment variables for configuration