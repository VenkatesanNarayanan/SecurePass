# Android release signing

`.github/workflows/android.yml` builds a release APK for 64-bit ARM phones on every version tag (called from `release.yml`). The job signs the APK and publishes it to the website and the GitHub Release only when all the secrets below exist. Without them it uploads an unsigned APK as a workflow artifact and publishes nothing.

## 1. Create the release keystore (once)

```bash
keytool -genkeypair -v \
  -keystore securepass-release.keystore \
  -alias securepass \
  -keyalg RSA -keysize 4096 -validity 10000
```

`keytool` asks for a keystore password, a key password (pressing Enter reuses the keystore password) and a name for the certificate.

**Back up the keystore file and both passwords** somewhere safe, such as a password manager entry with the file attached. Never commit the keystore to git. Android only installs an update over an existing app when it is signed with the same key. If the keystore is lost, every user has to uninstall SecurePass to move to a newly signed build, and uninstalling deletes the vault on the phone unless it is synced to Google Drive or exported.

## 2. Add the GitHub secrets

From the repository root, with the GitHub CLI:

```bash
base64 -i securepass-release.keystore | gh secret set ANDROID_KEYSTORE_BASE64
gh secret set ANDROID_KEYSTORE_PASSWORD   # paste the keystore password when asked
gh secret set ANDROID_KEY_ALIAS --body securepass
gh secret set ANDROID_KEY_PASSWORD        # paste the key password when asked
gh secret set ANDROID_GOOGLE_CLIENT_ID    # the Android OAuth client ID, e.g. 1234-abcd.apps.googleusercontent.com
```

`ANDROID_GOOGLE_CLIENT_ID` is the **Android** OAuth client from the Google Cloud console (APIs & Services → Credentials), not the desktop one in `sync.rs`. It is kept out of the repo. At build time it becomes a compile-time constant (`SECUREPASS_ANDROID_CLIENT_ID`), and its reversed form becomes the deep-link scheme that brings Google's sign-in redirect back into the app. The scheme is passed with `tauri android build --config`, so `tauri.conf.json` keeps an empty list. A build without it has no Google Drive sync on Android, and CI won't publish one.

You can also add them under GitHub → Settings → Secrets and variables → Actions.

## 3. Test without releasing

GitHub → Actions → **Android** → **Run workflow**. The signed APK appears as the `release-artifacts-android` artifact of that run. Install it on a phone and check that it works before tagging a release.

## Building locally

```bash
export SECUREPASS_ANDROID_CLIENT_ID=1234-abcd.apps.googleusercontent.com   # e.g. in ~/.zshrc
./scripts/build-apk.sh
```

The deep-link plugin writes the scheme into `src-tauri/gen/android/app/src/main/AndroidManifest.xml` during the build. `build-apk.sh` restores the committed file afterwards. If you build another way (for example `npm run tauri android build -- --config ...`), run `git checkout -- src-tauri/gen/android/app/src/main/AndroidManifest.xml` before committing. GitHub's push protection also blocks a push that contains the ID.

## Update banner

The release workflow adds an `android` section (version, APK link) to the website's `version.json` whenever a signed APK is published, and keeps the previous one if a release has no APK. The Android app's update banner reads only that section, so it appears only when a newer APK is available.

## Switching from the debug-signed APK

Earlier APKs (`scripts/build-apk.sh`) are signed with the Android debug key. A phone that has one installed refuses the release-signed APK as an update ("App not installed" / package conflict). On those phones:

1. Sync the vault to Google Drive, or export a `.spvault` backup.
2. Uninstall SecurePass.
3. Install the new APK, then restore from Drive or import the backup.

`scripts/build-apk.sh` still builds a debug-signed APK for local testing.
