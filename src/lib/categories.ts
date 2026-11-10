// Display metadata for entry categories. Colours are CSS variables from
// src/app.css so they follow the light/dark theme.

export interface CategoryMeta {
  key: string;
  label: string;
  icon: string;
  bg: string;
  fg: string;
  dot: string;
}

export const CATEGORY_ORDER = ["web-app", "ssh-server", "bank-financial", "email", "wi-fi", "generic"];

const KNOWN: Record<string, Omit<CategoryMeta, "key">> = {
  "web-app":        { label: "Web & Apps",    icon: "globe",  bg: "var(--cat-web-bg)",     fg: "var(--cat-web-fg)",     dot: "var(--cat-web-dot)" },
  "ssh-server":     { label: "SSH & Servers", icon: "server", bg: "var(--cat-ssh-bg)",     fg: "var(--cat-ssh-fg)",     dot: "var(--cat-ssh-dot)" },
  "bank-financial": { label: "Banking",       icon: "bank",   bg: "var(--cat-bank-bg)",    fg: "var(--cat-bank-fg)",    dot: "var(--cat-bank-dot)" },
  "email":          { label: "Email",         icon: "mail",   bg: "var(--cat-email-bg)",   fg: "var(--cat-email-fg)",   dot: "var(--cat-email-dot)" },
  "wi-fi":          { label: "Wi-Fi",         icon: "wifi",   bg: "var(--cat-wifi-bg)",    fg: "var(--cat-wifi-fg)",    dot: "var(--cat-wifi-dot)" },
  "generic":        { label: "Generic",       icon: "key",    bg: "var(--cat-generic-bg)", fg: "var(--cat-generic-fg)", dot: "var(--cat-generic-dot)" },
};

/** Title-case an unknown category key ("my-thing" → "My Thing"). */
function humanize(key: string): string {
  return key.replace(/[-_]/g, " ").replace(/\b\w/g, (l) => l.toUpperCase());
}

export function categoryMeta(key: string | null | undefined): CategoryMeta {
  const k = key || "generic";
  const known = KNOWN[k];
  if (known) return { key: k, ...known };
  return { key: k, ...KNOWN.generic, label: humanize(k) };
}

/** Known categories in display order, then any custom ones alphabetically. */
export function sortCategories(keys: string[]): string[] {
  const known = CATEGORY_ORDER.filter((k) => keys.includes(k));
  const custom = keys.filter((k) => !CATEGORY_ORDER.includes(k)).sort();
  return [...known, ...custom];
}

/** First letter (or digit) of a title, for the entry tile. */
export function initialOf(title: string): string {
  const m = (title || "").match(/[\p{L}\p{N}]/u);
  return m ? m[0].toUpperCase() : "?";
}

export function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "";
  const d = new Date(iso);
  const mins = Math.floor((Date.now() - d.getTime()) / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins} min ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs} h ago`;
  const days = Math.floor(hrs / 24);
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;
  return d.toLocaleDateString();
}
