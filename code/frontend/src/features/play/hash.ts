/** FNV-1a 32-bit hash of a string, returned as a non-negative number.
 *  Deterministic: the same input always yields the same value, so it can seed
 *  reproducible-but-random-looking choices (AI deck selection, opponent alias).
 *  Single source of truth for the project's seed hashing. */
export function fnv1a(seed: string): number {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i += 1) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return Math.abs(h);
}
