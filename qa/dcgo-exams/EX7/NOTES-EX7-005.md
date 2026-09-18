# NOTES — Kapurimon (EX7-005)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-005`):
**1 clause** — `inherited#0`.
DCGO script: `EX7/Black/EX7_005.cs` exists — the card is **not** `unavailable`.
The clause has a scenario; nothing is `unreachable`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `EX7-005#inherited#0` | `[Your Turn] [Once Per Turn] When effects place Option cards with the [Three Musketeers] trait in this Digimon's digivolution cards, gain 1 memory.` | `EX7-005-inherited0.yaml` (book `tm_ex7_005_pool.json`, deck `tm-kapurimon-ex7-005-egg` = the shared 50 with EX7-005 ×4 as the egg deck) | lowers, 7/7 asserts pass |

## The line

It is the committed `EX7-051-effect1.yaml` (Sparrowmon's `[Start of Your
Main Phase]`: place a `[Three Musketeers]` Option from hand under one of
your Digimon, `<Draw 1>`) played over a **Kapurimon** egg instead of the
shared book's ST6-01. Sparrowmon reaches the black egg through its
alt-circle "Lv.2 w/[Three Musketeers] in text: Cost 0" (the
`EX7-051-effect0` shape — one route, no cost prompt). At T5 the promoted
Sparrowmon's own effect places Bind Red Trigger P-180 (an **Option** with
the **trait**) under itself, and Kapurimon — now in that Digimon's
digivolution cards — gains 1 memory: **3 → 4**, the only observable of
the clause. Sparrowmon's `<Draw 1>` resolves alongside.

The `EX7-051-effect1.yaml` header explains it avoided Kapurimon precisely
because Kapurimon "has no YAML spec in our engine yet"; that is no longer
true (`code/digimon-engine/cards/ex7/EX7-005.yaml`, this run), which is
what makes this line authorable. The sibling header is stale on that one
sentence; not edited here (not this stage's card).

## Adversarial pre-Unity review — prompt sequence re-derived from `EX7_005.cs`

1. **Placer rows** are the four of `EX7-051-effect1.yaml`, unchanged:
   Sparrowmon's `OptionalSkill` (`isOptional: true`) is `dcgo_only`
   (our single declinable union pick is gate and pick at once); the
   hand-or-trash question is a direct `SetBool(canSelectHand)` with the
   trash empty — no RPC, no row; `SelectHandEffect` (P-180) and
   `SelectPermanentEffect` (Sparrowmon, the lone own Digimon) are SHARED.
2. **The clause asks nothing.** `SetUpActivateClass(..., 1 /* OPT */,
   FALSE, ...)` → not optional; the lone stacked OnAddDigivolutionCards
   effect goes straight to `AddMemory(1)` (`ICardEffect.cs:1130`,
   `UseOptional || !IsOptional`). Ours: no `optional`, one firing from the
   per-host batch window, `gain_memory: 1`. **Any prompt at this point on
   either side is a finding** — the line has no row for one, so a DCGO
   prompt would abort the job with a prompt mismatch and report itself.
3. **Trigger gate.** `IsExistOnBattleAreaDigimon(card)` resolves the
   buried Kapurimon through `PermanentOfThisCard()`; `IsOwnerTurn`;
   `permanentCondition: permanent == card.PermanentOfThisCard()` (the
   host is Sparrowmon's own permanent ✓); `cardEffectCondition:
   EffectSourceCard != null` (Sparrowmon's ActivateClass ✓);
   `cardCondition: IsOption && ContainsTraits("Three Musketeers")`
   (P-180 ✓ — note `EX7_005.cs` checks only the spaced spelling, while
   `EX7_051.cs` also accepts "ThreeMusketeers"; `HasThreeMusketeersTraits`
   in `CardSource.cs:3683` uses the spaced form too, so P-180's trait
   string must be the spaced one for LadyDevimon's own script to work —
   consistent).
4. **Order.** DCGO stacks the trigger and resolves it after Sparrowmon's
   `<Draw 1>`; ours flushes the batch window after the placing effect
   completes. The next decision row (P0's pass) sees memory 4 and the
   drawn card on both sides.
5. **Memory.** The placement is a cost, not memory; +1 → 4; P0 passes at
   4 (P1 then starts at 3). No zero-crossing anywhere on the line.

## Not measured by this line

- The **Option-only** half of the gate (a `[Three Musketeers]` trait
  **Digimon** card placed under the stack must NOT fire): the
  `BT7-056-inherited0.yaml` line does exactly that (EX7-073 under a
  Kapurimon-based stack) and asserts the memory delta of Dorumon alone —
  a Kapurimon firing there would show as an extra +1 and be a finding on
  BOTH cards' lines.
- The **`[Once Per Turn]`** lockout (a second qualifying placement in
  the same turn) — needs a second placer in one turn; not authored.
- An **opponent's** effect placing the Option on your turn (DCGO
  `EffectSourceCard != null` admits it; ours is not owner-gated either)
  — the quiet-opponent pool has no such effect.
