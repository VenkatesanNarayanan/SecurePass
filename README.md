# 🔐 SecurePass - Encrypted Password Manager

“Your passwords. Your device. Your cloud.”

A secure, cross-platform password manager built with Rust and Svelte, featuring military-grade encryption and a modern, intuitive interface.

![SecurePass](https://img.shields.io/badge/SecurePass-1.0.0-blue?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-1.91.1-orange?style=flat-square)
![Svelte](https://img.shields.io/badge/Svelte-5.0-orange?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2.0-purple?style=flat-square)

## ✨ Features

### 🔒 Security First
- **Military-grade encryption** using ChaCha20-Poly1305 AEAD
- **Argon2id key derivation** for master password hashing
- **Secure memory handling** with automatic zeroization
- **Encrypted vault storage** with unique nonces per entry

### 🎯 Core Functionality
- **Password vault management** with categorized entries
- **Secure password generation** with customizable options
- **Auto-lock timer** with configurable timeout
- **Clipboard security** with automatic clearing
- **Export/Import** encrypted vault backups

### 🎨 User Experience
- **Modern, responsive UI** built with Svelte 5
- **Category-based templates** for different entry types
- **Real-time search and filtering**
- **Keyboard shortcuts** for power users
- **Cross-platform support** (macOS, Windows, Linux)

### 📱 Entry Categories
- **Web/App Accounts** - URL, Username, Password, Recovery Email
- **SSH/Servers** - Hostname, Port, Username, Password, Private Key
- **Banking/Financial** - Bank Name, Username, Password, Account Number
- **Email Accounts** - Email, Password, IMAP/SMTP servers
- **Wi-Fi Networks** - SSID, Password, Network Type
- **Custom Fields** - Flexible field types for any use case

## 🚀 Quick Start

### Prerequisites
- **Node.js** 18+ ([Download](https://nodejs.org/))
- **Rust** 1.70+ ([Install](https://rustup.rs/))

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd securepass
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Start development server**
   ```bash
   npm run tauri dev
   ```

4. **Build for production**
   ```bash
   npm run tauri build
   ```

## 📖 Usage Guide

### First Time Setup
1. Launch SecurePass
2. Create a strong master password
3. Your encrypted vault will be created automatically

### Creating Password Entries
1. Click **"➕ New Entry"** or press **Ctrl+N**
2. Select an entry category (Web/App, SSH, Bank, etc.)
3. Fill in the required fields
4. Use the **🔐** button to generate secure passwords
5. Save your entry

### Password Generator
- **Customizable length** (8-64 characters)
- **Character types**: uppercase, lowercase, numbers, symbols
- **Exclude similar characters** (prevents 0/O, 1/l/I confusion)
- **Real-time strength indicator**
- **One-click copy to clipboard**

### Security Features
- **Auto-lock**: Vault locks automatically after 5 minutes of inactivity
- **Clipboard security**: Copied passwords auto-clear after 30 seconds
- **Secure clipboard**: Uses system clipboard with proper cleanup

### Backup & Restore
- **Export vault**: Create encrypted backups with **"📤 Export"**
- **Import vault**: Restore from backup files with **"📥 Import"**
- **Cross-platform compatibility**: Backups work across all supported platforms

### Keyboard Shortcuts
- **Ctrl+N** (Cmd+N): Create new entry
- **Ctrl+L** (Cmd+L): Lock vault
- **Escape**: Close modals
- **Ctrl+F**: Focus search (coming soon)

## 🛠️ Development

### Project Structure
```
securepass/
├── src/                    # Frontend (Svelte)
│   ├── components/         # UI components
│   ├── routes/            # SvelteKit routes
│   └── app.html           # Main HTML template
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── commands.rs    # Tauri command handlers
│   │   ├── crypto.rs      # Encryption utilities
│   │   ├── lib.rs         # Tauri app setup
│   │   ├── main.rs        # Application entry point
│   │   └── vault.rs       # Vault data structures
│   └── tauri.conf.json    # Tauri configuration
└── package.json           # Node.js dependencies
```

### Available Scripts

```bash
# Development
npm run tauri dev          # Start development server
npm run dev               # Start frontend only

# Building
npm run tauri build       # Build production app
cargo build --release     # Build Rust backend only

# Code Quality
npm run lint              # Lint frontend code
cargo clippy              # Lint Rust code
cargo fmt                 # Format Rust code

# Testing
cargo test                # Run Rust tests
npm test                  # Run frontend tests
```

### Architecture

**Frontend (Svelte + TypeScript):**
- Modern reactive UI with Svelte 5
- TypeScript for type safety
- Responsive design with CSS Grid/Flexbox
- Component-based architecture

**Backend (Rust + Tauri):**
- High-performance Rust backend
- Tauri for cross-platform desktop app
- ChaCha20-Poly1305 encryption
- Secure memory management

**Security Model:**
- Master password → Argon2id → Encryption key
- Unique nonce per encrypted entry
- Secure clipboard handling
- Automatic key zeroization

## 🔐 Security Details

### Encryption Algorithm
- **AEAD**: ChaCha20-Poly1305 (Authenticated Encryption with Associated Data)
- **Key Derivation**: Argon2id with 32-byte salt
- **Key Size**: 256-bit encryption keys
- **Nonce**: Unique 96-bit nonce per entry

### Data Storage
- **File Format**: Encrypted JSON with binary header
- **Location**: `~/.securepass/vault.dat` (Unix) / AppData (Windows)
- **Backup Format**: `.spvault` files with same encryption

### Memory Security
- **Zeroization**: Sensitive data cleared from memory
- **No Plaintext Storage**: Passwords never stored in plaintext
- **Secure Clipboard**: Automatic cleanup after timeout

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines
- Follow Rust and TypeScript best practices
- Add tests for new functionality
- Update documentation for API changes
- Ensure security review for crypto-related changes

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Security Notice

- **Never share your master password**
- **Keep backup files secure** - they contain your encrypted data
- **Use strong, unique master passwords**
- **Enable auto-lock** for unattended devices
- **Regularly backup your vault**

## 🆘 Troubleshooting

### Common Issues

**"Command not found: cargo"**
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Port already in use**
```bash
# Kill process on port 1420
lsof -ti:1420 | xargs kill -9
```

**Build fails on macOS**
```bash
# Install Xcode command line tools
xcode-select --install
```

### Getting Help
- Check the [Issues](https://github.com/your-repo/issues) page
- Review the [AGENTS.md](AGENTS.md) for development details
- Join our community discussions

---

**Built with ❤️ using Rust, Svelte, and Tauri**

*SecurePass - Your passwords, securely managed.* 🔐✨</content>
</xai:function_call: write>
<parameter name="filePath">README.md
