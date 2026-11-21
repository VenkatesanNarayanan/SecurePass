// Biometric unlock (see src-tauri/src/biometric.rs): fingerprint on Android,
// Windows Hello on Windows. The backend reports which, so the UI can word it.

export type BiometricKind = "fingerprint" | "windowsHello";

export interface BiometricStatus {
  available: boolean;
  enabled: boolean;
  kind: BiometricKind;
  passwordIntervalDays: number;
}

export const NO_BIOMETRICS: BiometricStatus = {
  available: false,
  enabled: false,
  kind: "fingerprint",
  passwordIntervalDays: 7,
};

export function biometricText(kind: BiometricKind, days: number) {
  if (kind === "windowsHello") {
    return {
      title: "Windows Hello unlock",
      unlockButton: "Unlock with Windows Hello",
      waiting: "Waiting for Windows Hello…",
      offHint:
        `Open your vault with Windows Hello (fingerprint, face or your Windows Hello PIN) instead of typing the ` +
        `master password. The master password is still needed every ${days} days.`,
      onHint:
        `On. Windows Hello's PIN also works in place of a fingerprint. Your master password is still needed at ` +
        `least every ${days} days.`,
      icon: "fingerprint",
    };
  }
  return {
    title: "Fingerprint unlock",
    unlockButton: "Unlock with fingerprint",
    waiting: "Waiting for fingerprint…",
    offHint:
      `Open your vault with a fingerprint instead of typing the master password. Only a fingerprint works, not ` +
      `the phone's PIN, and the master password is still needed every ${days} days.`,
    onHint:
      `On. Your master password is still needed at least every ${days} days, and after a fingerprint is added ` +
      `to this phone.`,
    icon: "fingerprint",
  };
}
