# NOTES — Tankmon (EX7-043)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-043`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script
`EX7/Black/EX7_043.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet: all three are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-043#effect#0` | `[Digivolve] Lv.3 w/[Three Musketeers] in text: Cost 2` | `EX7-043-effect0.yaml` | lowers, 6/6 asserts pass |
| `EX7-043#effect#1` | `[On Play] [When Digivolving] By returning 3 cards with the [Three Musketeers] trait from your hand or trash to the top of the deck, <De-Digivolve 1> 1 of your opponent's Digimon.` | `EX7-043-effect1.yaml` ([On Play] arm) | lowers, 8/8 |
| `EX7-043#inherited#0` | `<Reboot>` | `EX7-043-inherited0.yaml` | lowers, 8/8 |

Book: `qa/dcgo-exams/EX7/tm_ex7_043_pool.json` (deck `tm-tankmon`: EX7-043 ×4, the
vanilla black Lv.5 MetalTyrannomon ST5-10 ×4, and a RED egg deck, Koromon ST1-01 ×4).

All three files were left UNCOMMITTED by a crashed agent. This stage audited each
against `EX7_043.cs`, re-lowered it against the current engine (after `9aa21ed0f`,
`c7581932c`, `d74328811`, `35958972b`) and found nothing to change. What was checked:

## `effect#0`
The printed circle is also "Black Lv.3 / 2", so the base is the RED Lv.3 ToyAgumon
EX7-008 (its text names the trait) — legal only through the clause
(`TopCard.IsLevel3 && TopCard.HasText("Three Musketeers")`, no colour test). EX7-008
is never PLAYED (its `[On Play]` is a 3-card reveal); it is digivolved in the
breeding area over a red egg and promoted, so nothing of its own fires. Tankmon's
`[When Digivolving]` is gated on THREE trait cards in hand + trash
(`SharedCanActivateCondition: cardCount() >= 3`; our clause `condition:` is the same
gate) and P0 holds none → nothing asked on either side. One Digimon on the board.

## `effect#1` — cardinality split, plus the `<De-Digivolve>` count row
- DCGO: `OptionalSkill` (isOptional TRUE) → ONE `SelectHandEffect` (maxCount 3,
  canEndNotMax) → trash pick skipped (empty trash) → ONE ordering `SelectCardEffect`
  over the three → `SelectPermanentEffect` (De-Digivolve target) →
  **`SelectCountEffect`** from `IDegeneration(permanent, 1, activateClass)` (no
  `DegenerationCountRuling`; consumed under the harness even with the lone
  candidate `{1}` — `../BT21/NOTES-BT21-074.md`).
- Ours: three sequential `select_union_zone` picks (the 3rd lands on top), no
  separate ordering, nothing parked for `<De-Digivolve 1>`.
- So the cost rows are `sim_only` ×3 against `dcgo_only` ×3, the target row is
  SHARED, and the count row is `dcgo_only` `value: 1`. The ordering row names
  BeelStarmon first ("lower numbers on top") so both engines leave the same card on
  top of the deck. **The count row is already present** — this is one of the lines
  the campaign-wide De-Digivolve repair targets, and it was applied before the crash.
- BeelStarmon BT25-085 is a DUAL card since `20df249e6`; here it is only a HAND card
  paid as cost (it carries the `[Three Musketeers]` trait on both faces), never
  played or digivolved into, so the dual-card repair does not apply.
- P1's target is a real two-card stack (Kokatorimon BT1-014 over Biyomon ST1-02), so
  `<De-Digivolve 1>` has something to trash; P1 holds ONE Digimon (slot-safe).

## `inherited#0`
Tankmon (T3) → vanilla MetalTyrannomon ST5-10 on top (T5, one route, no cost
prompt) → attack the player → pass → `suspended: false` at P1's first T6 decision.
`RebootSelfStaticEffect(isInheritedEffect: true)` is static: nothing is asked.
