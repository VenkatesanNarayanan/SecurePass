// "Recently used" tracking for the entry list.
//
// An entry counts as used when you copy one of its fields, reveal a secret or
// open its link — not when you merely select it, so arrowing through the list
// doesn't reshuffle it.
//
// Stored per device in localStorage as { entryId: epoch ms }. It holds only
// random entry UUIDs and times (no titles or values), and deliberately stays
// out of the encrypted vault: writing the vault on every copy would mark it
// changed and trigger Google Drive syncs on every device.

const STORAGE_KEY = "sp_recent_use";
const MAX_ENTRIES = 500;

export type RecentUse = Record<string, number>;

export function loadRecentUse(): RecentUse {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : {};
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function save(map: RecentUse): RecentUse {
  // Keep only the most recent MAX_ENTRIES ids.
  const trimmed = Object.fromEntries(
    Object.entries(map)
      .sort((a, b) => b[1] - a[1])
      .slice(0, MAX_ENTRIES)
  );
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(trimmed));
  } catch {
    /* storage unavailable — keep it for this session only */
  }
  return trimmed;
}

/** Records a use of `entryId` now and returns the updated map. */
export function markUsed(map: RecentUse, entryId: string): RecentUse {
  return save({ ...map, [entryId]: Date.now() });
}

/** Drops ids that are no longer in the vault (deleted entries). */
export function pruneRecentUse(map: RecentUse, existingIds: string[]): RecentUse {
  const keep = new Set(existingIds);
  const ids = Object.keys(map);
  if (ids.every((id) => keep.has(id))) return map;
  return save(Object.fromEntries(Object.entries(map).filter(([id]) => keep.has(id))));
}
