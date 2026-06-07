#!/usr/bin/env python3
"""Placement-rule lint (RQ1) — WARN-MODE RATCHET.

Enforces the tier placement rule (docs/RUST_ENGINE_API.md §3): INLINE rules
machinery belongs in Tier 2 (`game_actions`), not the Tier-3 facade
(`effect_context/`). Flagged inline machinery:

  - `try_replace` (replacement-window dispatch),
  - direct `battle_area[..]` source mutation,
  - LOW-LEVEL observer dispatch — `self.game.fire_*` EXCEPT calls that delegate
    to a cohesive Tier-2 operation (allowlisted below). Delegating to a Tier-2
    fire op is correct facade behavior; constructing+firing observers inline is
    not.

RATCHET, not hard gate: per-(file, pattern) baseline counts; fails only if a
count INCREASES. Ratchet baselines DOWN as relocation lands. Shipped WARN-mode
via `.github/workflows/facade-placement-lint.yml` (`continue-on-error: true`);
promote to required once every baseline is zero.

Exit codes: 0 = at/under baseline; 1 = a new violation appeared.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "digimon-engine" / "src" / "effect_context"

# Tier-2 fire operations the facade may delegate to (NOT inline machinery).
ALLOWED_FIRE = {"fire_on_play", "fire_security_removed_observers"}

TRY_REPLACE = re.compile(r"\.try_replace\(")
FIRE_CALL = re.compile(r"self\.game\.(fire_[a-z_]+)\(")
BATTLE_MUT = re.compile(
    r"player_mut\([^)]*\)\.battle_area\[|battle_area\[[^\]]+\]\.card_sources"
)

# Per-(relpath, pattern) baseline — the Tier-3->Tier-2 relocation backlog.
# RATCHET DOWN as cleanup lands; never up.
BASELINE = {
    # try_replace: 0 — all six "would-be-X" windows route through Tier-2
    #   Game::would_replacement_proceeds / would_replacement_is_clear.
    # fire (inline): 0 — fire_security_removed_observers moved to Tier 2; the
    #   remaining self.game.fire_* calls are allowlisted Tier-2 delegations.
    # battle_area mutation: still inline at these sites (digixros / under-tamer
    #   source moves) — the remaining backlog.
    ("action/digivolve.rs", "battle_area[..] source mutation"): 1,
    ("action/play.rs", "battle_area[..] source mutation"): 2,
    ("action/sources.rs", "battle_area[..] source mutation"): 1,
}


def main() -> int:
    violations = []
    tracked = 0
    for path in sorted(ROOT.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        body = "\n".join(
            l for l in path.read_text(encoding="utf-8").splitlines()
            if not l.lstrip().startswith("//")
        )
        counts = {
            "try_replace (replacement window)": len(TRY_REPLACE.findall(body)),
            "fire_* inline (non-delegating)": sum(
                1 for m in FIRE_CALL.findall(body) if m not in ALLOWED_FIRE
            ),
            "battle_area[..] source mutation": len(BATTLE_MUT.findall(body)),
        }
        for label, count in counts.items():
            if count == 0:
                continue
            base = BASELINE.get((rel, label), 0)
            tracked += min(count, base)
            if count > base:
                violations.append((rel, label, count, base))

    if violations:
        print("FACADE PLACEMENT LINT — NEW violations (inline rules machinery in Tier-3 facade):")
        for rel, label, count, base in violations:
            print(f"  {rel}: {label} = {count} (baseline {base}) -> relocate to Tier 2 (game_actions)")
        print("\nSee the placement rule in docs/RUST_ENGINE_API.md §3.")
        return 1

    if tracked == 0:
        print("facade placement lint OK - 0 inline rules-machinery occurrences in the facade. "
              "Placement rule fully satisfied; promote the CI check to required.")
    else:
        print(f"facade placement lint OK - {tracked} tracked occurrence(s) at/under baseline "
              f"(Tier-3->Tier-2 relocation backlog; ratchet down as cleanup lands).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
