#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-}"
MESSAGE="${2:-}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: ./scripts/release.sh <version> \"<release message>\""
  echo "  e.g. ./scripts/release.sh 1.1.16 \"fix: auto-sync after add/delete, merge on import\""
  exit 1
fi

if [[ -z "$MESSAGE" ]]; then
  echo "Error: release message is required."
  echo "  e.g. ./scripts/release.sh 1.1.16 \"fix: auto-sync after add/delete, merge on import\""
  exit 1
fi

# Strip leading 'v' if supplied
VERSION="${VERSION#v}"
TAG="v${VERSION}"

echo "Releasing ${TAG}: ${MESSAGE}"

# ── Bump package.json ──────────────────────────────────────────────────────
node -e "
  const fs = require('fs');
  const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  pkg.version = '${VERSION}';
  fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
  console.log('  package.json →', pkg.version);
"

# ── Bump tauri.conf.json ───────────────────────────────────────────────────
node -e "
  const fs = require('fs');
  const conf = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  conf.version = '${VERSION}';
  fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n');
  console.log('  tauri.conf.json →', conf.version);
"

# ── Commit, tag, push ──────────────────────────────────────────────────────
git add package.json src-tauri/tauri.conf.json
git commit -m "${MESSAGE}"
git tag -a "${TAG}" -m "${MESSAGE}"
git push
git push origin "${TAG}"

echo "Done — ${TAG} pushed. GitHub Actions will build and publish."
