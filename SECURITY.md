# Security policy

SecurePass stores people's passwords, so security reports are taken seriously and handled privately.

## Reporting a vulnerability

**Please don't open a public issue, discussion or pull request for a security problem.**

Report it privately through GitHub instead:

1. Go to the repository's **Security** tab.
2. Click **Report a vulnerability** (or open [this link](https://github.com/VenkatesanNarayanan/SecurePass/security/advisories/new)).
3. Describe the problem, the affected version, and how to reproduce it. A proof of concept helps but isn't required.

Only the maintainers can see the report. You'll get an acknowledgement within **7 days**, and we'll keep you updated while it's investigated and fixed. Once a fix is released, we'll publish an advisory and credit you if you'd like.

## Supported versions

Fixes go into the **latest release** only. The app checks for updates on start-up and shows a banner when a newer version is available; please update before reporting.

## Scope

In scope — anything that could expose vault contents or the master password, for example:

- weaknesses in encryption, key derivation or the vault file format (`src-tauri/src/crypto.rs`, `commands.rs`)
- ways to read or modify a vault without the master password
- secrets leaking to disk, logs, the clipboard beyond its timeout, or the network
- problems in Google Drive sync or the OAuth flow (`src-tauri/src/sync.rs`) that expose data or tokens
- issues in the published installers or the release pipeline

Known limitations (documented in the README, not vulnerabilities by themselves, though improvements are welcome):

- decrypted entries live in ordinary memory while the vault is unlocked, and an attacker running code as your user can read them
- operating-system clipboard history may keep copied values
- Google Drive tokens are stored unencrypted in `~/.securepass/cloud_tokens.json`
- the Google OAuth client secret in `sync.rs` is public by design for desktop apps
- installers are not code-signed yet

## Design

The encryption design (ChaCha20-Poly1305, Argon2id 64 MiB / 3 iterations, file format) is described in the README under **Security design**.
