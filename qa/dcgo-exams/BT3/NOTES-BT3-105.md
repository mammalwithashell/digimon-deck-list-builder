# NOTES — Breath of the Gods (BT3-105)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT3-105`):
**2 clauses** — `effect#0`, `effect#1`.
DCGO script: `BT3/Black/BT3_105.cs` exists — the card is **not** `unavailable`.
Both clauses have a scenario; neither is `unreachable`.
Book: `qa/dcgo-exams/BT3/tm_bt3_105_pool.json` (decks `tm-breath-black`, `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT3-105#effect#0` | `[Main] 1 of your Digimon gains <Reboot> ... and "This Digimon can't have its DP reduced or be returned to its owner's hand or deck" until the end of your opponent's next turn.` | `BT3-105-effect0.yaml` | lowers, asserts pass |
| `BT3-105#effect#1` | `[Security] Your opponent's Digimon can't attack players for the turn.` | `BT3-105-effect1.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

Both files lowered and asserted as found. Prompt sequences re-derived from
`BT3_105.cs`:

- `effect#0` (OptionSkill region): `SetUpActivateClass(null, ..., -1, false,
  ...)` — no OptionalSkill; `HasMatchConditionPermanent` guard; ONE
  `SelectPermanentEffect(Custom, maxCount 1, canNoSelect: false)` over own
  battle-area Digimon; then `GainReboot` / `GainImmuneFromDPMinus` /
  `GainCanNotReturnToHand` / `GainCanNotReturnToDeck`, all
  `UntilOpponentTurnEnd`, no prompt. Ours (`BT3-105.yaml`): `optional:
  false`, one `select_own_permanent (kind: digimon)`, then `grant_keyword` +
  three `add_modifier`s. 1:1. The line fields TWO Digimon so the pick is a
  choice; a comment that claimed DCGO silently force-picks a lone candidate
  was corrected — under the harness both seats take
  `SelectPermanentEffect`'s AI branch, which consumes a scripted row
  regardless (`P/P-180-effect1.yaml`).
- `effect#1` (SecuritySkill region): `SetUpActivateClass(null, ..., -1,
  false, ...)`, `SetIsSecurityEffect(true)` →
  `GainCanNotAttackPlayerEffect(attacker: opponent battle-area Digimon,
  defender == null, UntilEachTurnEnd)`. No prompt. Ours: a continuous
  `CannotAttackPlayer` modifier over the opponent's Digimon, `end_of_turn`.

## What each line measures, and what it does not

- `effect#0`: the **<Reboot> half** is witnessed (Gotsumon attacks on T5,
  stays suspended, and reads unsuspended at P1's T6 breeding decision —
  after P1's unsuspend phase, before P0's). The **DP-immunity and
  no-bounce halves** are not probed: the quiet ST-1 opponent has no −DP or
  bounce effect to aim at Gotsumon. A future probe needs a per-card book
  with such an effect (e.g. a black/purple opponent) and would measure only
  that nothing changes.
- `effect#1`: the restriction is a standing prohibition that neither engine
  projects as state, and the exam cannot script an ILLEGAL action to prove
  it bites (an `attack: { target: security }` by P1's second Digimon would
  fail to lower here and be refused by DCGO's `InputDriver` there — neither
  is a measurement). The line therefore measures that both engines run the
  flip identically and that the restriction is scoped to PLAYERS:
  Vermilimon still attacks P0's suspended Gotsumon (a Digimon target) after
  the flip, which is legal under the clause. The negative half ("can't
  attack players") stays **unmeasured by this line**; the clause is reached
  and the wire agrees, but a `confirmed` here should be read with that
  scope.

## Slot hygiene pass (2026-09-18, second resume) — both lines re-authored

The campaign's centre-out rule (`../BT25/NOTES-BT25-083.md`: `attack:` /
`digivolve:` `field.N` reaches DCGO as a COMPACT index into
`GetFieldPermanents()`, frame-id order, and `CardSource.PreferredFrame()`
seats Digimon at frames 4, 3, 5, …) was measured AFTER the audit above. Both
files broke it, and both would have produced a scenario-artefact divergence,
not a finding:

- `effect#0` fielded Gotsumon + Jazamon and attacked with `field.0`: DCGO's
  slot 0 is the second-played Jazamon, which carries no grant, so DCGO's T6
  row would read Jazamon `suspended: true` against our all-unsuspended board.
  Re-authored: Jazamon (T1), a second Jazamon + Gotsumon (T3); Gotsumon is the
  THIRD Digimon — `field.2` on both engines — and is the grant target (by
  identity, three candidates) and the attacker. Same turn count (witness at
  step 14).
- `effect#1` fielded Vermilimon + Biyomon for P1 and addressed both: DCGO
  would have flipped BT3-105 with Vermilimon and thrown Biyomon at Gotsumon.
  Re-authored: Vermilimon (T2), a second Vermilimon (T4, P1's T4 draw so
  `play:` is never ambiguous), Biyomon (T6) — Biyomon is `field.2` on both
  engines, and the Digimon-target attack uses `field.0`, which names "a
  Vermilimon" on either engine; the two copies are indistinguishable in the
  projected multiset. Gotsumon attacks on T7, the clause turn is T8, the
  witness moved from step 16 to step 20.

Both lower sim-only with `tm_bt3_105_pool.json`; all asserts pass.
