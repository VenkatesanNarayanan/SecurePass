#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

# One EXIT trap for everything (a second `trap ... EXIT` would replace it).
RESTORE_MANIFEST=""
TMP_DIR=""
cleanup() {
  if [[ -n "$RESTORE_MANIFEST" ]]; then git checkout -- "$RESTORE_MANIFEST"; fi
  if [[ -n "$TMP_DIR" ]]; then rm -rf "$TMP_DIR"; fi
}
trap cleanup EXIT

# ── Android SDK / NDK / JDK ─────────────────────────────────────────────────
export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
NDK_VERSION="$(ls "${ANDROID_HOME}/ndk" | sort -V | tail -n1)"
export NDK_HOME="${NDK_HOME:-${ANDROID_HOME}/ndk/${NDK_VERSION}}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$NDK_HOME}"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home}"

echo "ANDROID_HOME=${ANDROID_HOME}"
echo "NDK_HOME=${NDK_HOME}"
echo "JAVA_HOME=${JAVA_HOME}"

# ── Build ────────────────────────────────────────────────────────────────
# Google Drive sync needs the Android OAuth client ID, which is kept out of
# the repo (docs/ANDROID_RELEASE.md). It becomes a compile-time constant
# (SECUREPASS_ANDROID_CLIENT_ID) and the deep-link scheme in the build config.
if [[ -z "${SECUREPASS_ANDROID_CLIENT_ID:-}" ]]; then
  echo "warning: SECUREPASS_ANDROID_CLIENT_ID is not set; this APK will have no Google Drive sync." >&2
  npm run tauri android build -- --apk true
else
  export SECUREPASS_ANDROID_CLIENT_ID
  SCHEME="com.googleusercontent.apps.${SECUREPASS_ANDROID_CLIENT_ID%.apps.googleusercontent.com}"
  MANIFEST=src-tauri/gen/android/app/src/main/AndroidManifest.xml
  # The deep-link plugin writes the scheme into the committed manifest during
  # the build; put the committed version back afterwards so it isn't committed.
  RESTORE_MANIFEST="$MANIFEST"
  # Force the deep-link plugin's build script to run, so it writes the scheme
  # into the manifest even when cargo's cache says nothing changed.
  for target in aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android; do
    (cd src-tauri && cargo clean -p tauri-plugin-deep-link --target "$target" --release)
  done
  npm run tauri android build -- --apk true \
    --config "{\"plugins\":{\"deep-link\":{\"mobile\":[{\"scheme\":[\"${SCHEME}\"]}]}}}"
fi

# ── Locate the unsigned universal release APK ───────────────────────────────
APK_SRC="$(find src-tauri/gen/android -type f -name '*-unsigned.apk' -path '*universal/release*' | head -n1)"
if [[ -z "$APK_SRC" ]]; then
  echo "Build finished but no unsigned release APK was found under src-tauri/gen/android." >&2
  exit 1
fi

# ── zipalign + debug-sign ────────────────────────────────────────────────────
# No release keystore is configured for this project, so we sign with the
# Android debug keystore — same cert used by all previously shipped
# SecurePass.apk builds (fingerprint a695e851...), so it installs cleanly
# over an existing install.
BUILD_TOOLS_DIR="$(ls -d "${ANDROID_HOME}"/build-tools/*/ | sort -V | tail -n1)"
DEBUG_KEYSTORE="${HOME}/.android/debug.keystore"

if [[ ! -f "$DEBUG_KEYSTORE" ]]; then
  echo "Debug keystore not found at ${DEBUG_KEYSTORE}." >&2
  exit 1
fi

TMP_DIR="$(mktemp -d -t securepass-apk)"
ALIGNED_APK="${TMP_DIR}/aligned.apk"
SIGNED_APK="${TMP_DIR}/signed.apk"

"${BUILD_TOOLS_DIR}zipalign" -p -f 4 "$APK_SRC" "$ALIGNED_APK"
"${BUILD_TOOLS_DIR}apksigner" sign \
  --ks "$DEBUG_KEYSTORE" --ks-pass pass:android \
  --ks-key-alias androiddebugkey --key-pass pass:android \
  --out "$SIGNED_APK" "$ALIGNED_APK"
"${BUILD_TOOLS_DIR}apksigner" verify --print-certs "$SIGNED_APK"

cp "$SIGNED_APK" ./SecurePass.apk

echo "Done — signed APK copied to ./SecurePass.apk (from ${APK_SRC})"
