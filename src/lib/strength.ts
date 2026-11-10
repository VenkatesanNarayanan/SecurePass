// Rough password strength estimate for the UI (length + character variety).
// Not a security control — just guidance shown next to password fields.

export interface Strength {
  /** 0 = empty, 1 weak, 2 fair, 3 good, 4 strong */
  score: 0 | 1 | 2 | 3 | 4;
  label: string;
  color: string;
}

const COMMON = /^(password|passw0rd|qwerty|letmein|welcome|admin|iloveyou|monkey|dragon|abc123|123456)/i;

export function passwordStrength(pw: string): Strength {
  if (!pw) return { score: 0, label: "", color: "var(--strength-empty)" };

  let points = 0;
  if (pw.length >= 8) points++;
  if (pw.length >= 12) points++;
  if (pw.length >= 16) points++;
  if (pw.length >= 20) points++;
  const classes = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^a-zA-Z0-9]/].filter((r) => r.test(pw)).length;
  points += classes;

  // Obvious patterns cap the score regardless of length.
  if (COMMON.test(pw) || /^(.)\1+$/.test(pw) || (/(19|20)\d{2}!?$/.test(pw) && pw.length < 14)) {
    points = Math.min(points, 3);
  }

  if (points <= 3) return { score: 1, label: "Weak", color: "var(--strength-weak)" };
  if (points <= 5) return { score: 2, label: "Fair", color: "var(--strength-fair)" };
  if (points <= 7) return { score: 3, label: "Good", color: "var(--strength-good)" };
  return { score: 4, label: "Strong", color: "var(--strength-strong)" };
}
