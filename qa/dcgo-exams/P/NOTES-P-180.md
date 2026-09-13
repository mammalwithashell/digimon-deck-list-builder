# NOTES — Bind Red Trigger (P-180)

Denominator (`clause_coverage.extract --card-ids P-180`): **3 clauses** —
`effect#0`, `effect#1`, `effect#2`. All three have a scenario; none is
`unreachable` or `unavailable` (`P/Red/P_180.cs` exists in DCGO).

| Clause | Scenario | Sim-only |
|---|---|---|
| `P-180#effect#0` | `P-180-effect0.yaml` | lowers, asserts pass — **oracle abort predicted** (below) |
| `P-180#effect#1` | `P-180-effect1.yaml` | lowers, asserts pass |
| `P-180#effect#2` | `P-180-effect2.yaml` | lowers, asserts pass |

## Predicted finding on `P-180#effect#0` — trigger timing (engine-side)

The only effect in this pool that trashes an Option from digivolution cards
without a follow-up prompt of its own **and** exists in the prebuilt harness
binary is LadyDevimon BT25-083's [When Attacking] cost. Its body continues
after the trash with "you may use 1 [Three Musketeers] trait Option card from
your trash with the cost reduced by 3", and the two engines order that
continuation against the trashed card's own trigger differently:

- **DCGO** (`ITrashDigivolutionCards.TrashDigivolutionCards()` →
  `AutoProcessing.StackSkillInfos(hashtable, OnDigivolutionCardDiscarded)`):
  the trigger is STACKED and resolves after LadyDevimon's effect finishes.
  The use-from-trash `SelectCardEffect` is asked first.
- **Our engine** (`game_actions/link.rs::trash_specific_source_card` →
  `enqueue_triggered` + `maybe_drain_effect_queue`): the trigger's target
  pick parks immediately after the trash, BEFORE LadyDevimon's
  use-from-trash prompt (observed at lowering: the step after the trash sees
  a parked `OppField` prompt).

`general_rule.pdf` 15-8-3-2 (digest: docs/digimon-rules/digest.md) — a
trigger-type effect "can't activate during rule/effect processing; it waits
as pending activation" — makes DCGO's order the rules-correct one. This is a
**candidate engine bug** (effect-queue drained mid-process), not a P-180 text
issue, and not a translation gap the exam should fold.

`P-180-effect0.yaml` is authored in OUR order so that it lowers; the oracle
run is expected to abort at the trigger-pick row with "our engine expected
`SelectPermanentEffect`, DCGO asked `SelectCardEffect`". That abort is the
measurement and should be triaged as `diverged` with the citation above.
Re-ordering the rows with `sim_only:`/`dcgo_only:` would make both engines
walk their own order and compare only end states — deliberately NOT done,
because it would launder the ordering divergence into a `confirmed`.

## Harness translation gap noticed while authoring (not a rules finding)

A `select: { cards: [ID] }` answer against our `UnionZone { zones: material }`
prompt cannot be resolved: `selection_resolve.rs`'s `Hand | UnionZone` arm
scans only hand and trash, so a digivolution-card candidate (a
`SOURCE_SELECT` id) is reported as "card pick not found". The three attack
lines answer that pick with `{ yes: true, sim_only: true }` (the resolver's
single-accept-id path) and put DCGO's card row on the wire as `dcgo_only`.
Worth a resolver arm for `UnionZone` material candidates (mirroring
`SelectionKind::Material`'s `material_source_ids`).
