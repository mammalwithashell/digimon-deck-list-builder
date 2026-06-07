#!/usr/bin/env python3
"""Placement-rule lint (RQ1) — WARN-MODE RATCHET.

Enforces the tier placement rule (docs/RUST_ENGINE_API.md §3): rules machinery
(replacement-window dispatch `try_replace`, observer firing `fire_*`, direct
`battle_area[..]` source mutation) belongs in Tier 2 (`game_actions`), NOT in
the Tier-3 facade (`effect_context/`).

The broader relocation beyond `de_digivolve` (B1 follow-up) is intentionally
incomplete, so this lint is a RATCHET, not a hard gate: it records the current
per-file occurrence counts as a baseline and fails only if a file's count
*increases* (a NEW violation). As the deferred cleanup lands, lower the baseline.

Shipped in WARN mode via CI `continue-on-error: true` (see
.github/workflows/facade-placement-lint.yml) — matches the action-space-codegen
-drift rollout pattern; promote to required once the baseline reaches zero.

Exit codes: 0 = at/under baseline; 1 = a new violation appeared.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "digimon-engine" / "src" / "effect_context"

PATTERNS = {
    "try_replace (replacement window)": re.compile(r"\.try_replace\("),
    "fire_* (observer dispatch)": re.compile(r"self\.game\.fire_[a-z]"),
    "battle_area[..] source mutation": re.compile(
        r"player_mut\([^)]*\)\.battle_area\[|battle_area\[[^\]]+\]\.card_sources"
    ),
}

# Per-(relpath, pattern) baseline. Occurrences allowed today (the B1 / broader
# Tier-3->Tier-2 relocation backlog). RATCHET DOWN as cleanup lands; never up.
BASELINE = {
    ("action/security.rs", "try_replace (replacement window)"): 1,
    ("action/trash.rs", "try_replace (replacement window)"): 4,
    ("action/zones.rs", "try_replace (replacement window)"): 1,
    # fire_* trash-source sites (the 4 divergent OnDigivolutionCardTrashed
    # dispatches) ratcheted to ZERO 2026-06-07: all route through the Tier-2
    # `trash_source_and_fire[_with_cause]` primitive (§10.4 complete). The two
    # remaining fire_* below are DIFFERENT observers (security-removal,
    # on-play), part of the broader Tier-3->Tier-2 backlog.
    ("action/security.rs", "fire_* (observer dispatch)"): 1,
    ("selections.rs", "fire_* (observer dispatch)"): 1,
    ("action/digivolve.rs", "battle_area[..] source mutation"): 1,
    ("action/play.rs", "battle_area[..] source mutation"): 2,
    ("action/sources.rs", "battle_area[..] source mutation"): 1,
}


def main() -> int:
    violations = []
    tracked = 0
    for path in sorted(ROOT.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")
        # strip line comments cheaply (avoid counting in `// ...` prose)
        lines = [l for l in text.splitlines() if not l.lstrip().startswith("//")]
        body = "\n".join(lines)
        for label, pat in PATTERNS.items():
            count = len(pat.findall(body))
            if count == 0:
                continue
            base = BASELINE.get((rel, label), 0)
            tracked += min(count, base)
            if count > base:
                violations.append((rel, label, count, base))

    if violations:
        print("FACADE PLACEMENT LINT — NEW violations (rules machinery added to Tier-3 facade):")
        for rel, label, count, base in violations:
            print(f"  {rel}: {label} = {count} (baseline {base}) -> relocate to Tier 2 (game_actions)")
        print("\nSee the placement rule in docs/RUST_ENGINE_API.md §3.")
        return 1

    print(f"facade placement lint OK - {tracked} tracked occurrence(s) at/under baseline "
          f"(B1 / Tier-3->Tier-2 relocation backlog; ratchet down as cleanup lands).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
