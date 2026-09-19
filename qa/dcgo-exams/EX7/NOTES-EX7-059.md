# NOTES — BeelStarmon ACE (EX7-059)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-059`):
**5 clauses** — `effect#0` … `effect#4`. DCGO script `EX7/Purple/EX7_059.cs`
exists — the card is **not** `unavailable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/EX7-059.json` does not exist): all five are
`unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-059#effect#0` | `[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 3` | `EX7-059-effect0.yaml` | lowers, asserts pass |
| `EX7-059#effect#1` | `[Hand] [Counter] <Blast Digivolve>` | `EX7-059-effect1.yaml` | lowers, asserts pass — oracle diverged as predicted (finding 2); engine FIXED, re-diff CLEAN → **confirmed** |
| `EX7-059#effect#2` | `[On Play] [When Digivolving] Return 1 Option card from your trash to the hand. Then, you may use 1 [Three Musketeers] trait Option card from your hand without paying the cost.` | `EX7-059-effect2.yaml` ([When Digivolving] arm, both halves) | lowers, asserts pass |
| `EX7-059#effect#3` | `[When Attacking] [Once Per Turn] By trashing 1 Option card from this Digimon's digivolution cards, you may use 1 [Three Musketeers] trait Option card …` | `EX7-059-effect3.yaml` | lowers, asserts pass |
| `EX7-059#effect#4` | `<Overflow (-4)>` | `EX7-059-effect4.yaml` | lowers, asserts pass — **oracle divergence predicted** (finding 3) |

Book: `qa/dcgo-exams/EX7/tm_ex7_059_pool.json` (decks `tm-beelstarmon-ace`,
`tm-red-opponent-059`). Every line keeps P0 on a ONE-Digimon board (the
centre-out slot rule, `../BT25/NOTES-BT25-083.md`), and none digivolves into
BeelStarmon BT25-085 or resolves a `<De-Digivolve>`.

## Prompt-shape facts (re-derived from `EX7_059.cs`)

- The special digivolution line passes `level: 5` and no `cardColor`: no
  colour check, so the RED Megadramon (EX7-011) is a legal base and the purple
  circle is not — one open route, no cost `SelectCountEffect`.
- `[On Play]` / `[When Digivolving]`: `SetUpActivateClass(..., -1, false, ...)`
  — MANDATORY, no `OptionalSkill`; `CanActivate` is only "on the battle area",
  so it always activates, but each half is wrapped in its own candidate check
  and asks nothing without a candidate. Half 1 is `SelectCardEffect(AddHand,
  Root.Trash, canNoSelect: () => false)`; half 2 is
  `SelectHandEffect(canNoSelect: true)`.
- `[When Attacking]`: `SetUpActivateClass(..., 1, true, ...)`, `CanActivate` =
  an Option among THIS permanent's `DigivolutionCards` → `OptionalSkill`, then
  `SelectCardEffect(Root.Custom, canNoSelect: () => true)`, then (only `if
  (trashed)` and a hand candidate exists) `SelectHandEffect(canNoSelect:
  true)`.
- `<Blast Digivolve>`: `CardEffectFactory.BlastDigivolveEffect` —
  `OptionalSkill` (`isOptional: true`, `SetIsCounterEffect(true)`), then
  `SelectPermanentEffect(maxCount 1, canNoSelect: false)` for the Digimon, then
  `PlayCardClass(payCost: false, activateETB: true)`.
- `<Overflow>` is NOT scripted per card in DCGO: `CEntity_Base.OverflowMemory`
  + `AceOverflowClass.Overflow()` (`CardController.cs` ~6131) from every
  leave-the-field path. No `ActivateClass`, so no `effect_activation` row — the
  clause is measured by the state diff alone.

## ENGINE FINDINGS measured while authoring (none fixed here — exam stage)

### 1. No counter timing on a player-target attack

`combat/mod.rs::try_enter_counter` returns `false` for `AttackTarget::Player`
("Scope: **Digimon-target attacks only**. Player-target attacks skip Counter to
match Python"). Measured: the first draft of `effect#1` had P1's Biyomon attack
P0's SECURITY with the ACE in hand and Megadramon on the field — our engine
parked nothing, and a `select:` there "answered no live prompt".

Counter timing is a step of EVERY attack (`general_rule.pdf` 11-1-3: Attack
declaration → Counter timing → Block timing → …), `<Blast Digivolve>` (16-25)
carries no attack-target restriction, and DCGO's `BlastDigivolveEffect` triggers
on `CanTriggerOnPermanentAttack(opponent permanent)` for any target. Class:
**our bug** (engine). Proposed id `G-ENGINE-COUNTER-WINDOW-PLAYER-TARGET-ATTACK`
for `docs/RUST_ENGINE_GAPS.md` (not added from this stage). Blast radius: every
`[Counter]` card — ACE Blast Digivolve, counter Options — against the most
common attack in the game.

The committed `effect#1` line aims the attack at a suspended Megadramon, where
both engines open the window, so it measures the clause rather than this gap.
Campaign-wide authoring rule that follows: **never let the opponent attack the
PLAYER while the scripted seat holds a usable `[Counter]` card** — DCGO asks an
`OptionalSkill` our engine never parks (`EX7-011-inherited0.yaml` and
`EX7-059-effect2.yaml` draw the ACE after the opposing attack for this reason).

### 2. Blast Digivolve does not draw the digivolution card

`combat/mod.rs::execute_blast_digivolve` moves the card from hand onto the
stack and enqueues `WhenDigivolving`; it never draws. Measured on
`effect#1`: P0's hand is 5 after the Blast (a digivolution draw would make it
6, the extra card being stack[12] = ST1-02).

`general_rule.pdf` 8-1-3-3 makes "draws 1 card" part of the digivolution
procedure and 16-25-4 says Blast Digivolve "is an effect that digivolves the
chosen Digimon"; DCGO's `PlayCardClass` draws for ANY digivolution (`if
(isEvolution) … new DrawClass(card.Owner, 1, null).Draw()`,
`CardController.cs` ~1759-1762). Class: **our bug** (engine). **Predicted
oracle verdict for `effect#1`: `diverged`**, lead `p0.hand` on the trailing
row. (Oracle run 2026-09-18 diverged exactly there: step 17 `p0.hand`, DCGO
holding the extra ST1-02.)

**RESOLVED (2026-09-19, ENGINE FIX).** `execute_blast_digivolve` now calls
`draw()` after the stack mutation and before the `WhenDigivolving` fan-out,
mirroring `effect_initiated_digivolve`. Test:
`cards_behavioral::ex7::ex7_059::ex7_059_blast_digivolve_draws_one_card`
(failed before, passes after). Re-diff against the preserved sidecar
`20260918T133748Z_6990b2ac….state.jsonl` is CLEAN; `effect#1` now asserts
`p0.hand` (6 cards). Gap: `G-ENGINE-BLAST-DIGIVOLVE-NO-DRAW`
(`docs/RUST_ENGINE_GAPS.md`).

Follow-up finding (NOT fixed, not measured by this exam): the same blast path
also does not enqueue the global `OnDigivolve` observer that
`effect_initiated_digivolve` fires, so a "when a Digimon digivolves" watcher
would miss a Blast Digivolve. Needs its own citation + test before a fix.

### 3. `<Overflow>` is applied turn-player-relative, not owner-relative

`Game::apply_ace_overflow_for_sources` (`game/mod.rs` ~1402) does
`self.memory += penalty` on the raw seesaw ("positive = favor of
`memory_pair.0`", i.e. the TURN player). When the ACE leaves on the OPPONENT's
turn its owner therefore GAINS 4. Measured on `effect#4`: P1 pays 8 from 3
(gauge 5 on P0's side); after the ACE is deleted our engine reads
`p0.memory: 9`; the printed text ("lose 4 memory") and DCGO
(`cardSource.Owner.AddMemory(-OverflowMemory)`) give **1**. On the owner's own
turn the sign happens to be right, which is the only case
`ex7_059_overflow_minus_4_fires_on_leave_field` covers. Class: **our bug**
(engine; affects every ACE in the pool, and the opponent's turn is when ACEs
usually leave). Fix shape: `lose_memory_for_player(owner, 4)` per leaving ACE
(`game/memory.rs` already has the owner-relative helper). **Predicted oracle
verdict for `effect#4`: `diverged`**, lead `p0.memory` on the trailing row;
`p0.memory` is left out of that file's `assert:` block.

## Harness translation notes

- The counter-window prompt is `SelectionKind::Hand` whose single accept id is
  `encode_digivolve(hand, field)` — card AND target in one id.
  `selection_resolve.rs`'s Hand arm cannot name it by `cards:` ("card pick
  'EX7-059' not found in Hand prompt … valid [400]"), so `effect#1` answers it
  through the single-accept-id `yes:` path, `sim_only`, and carries DCGO's two
  rows (`OptionalSkill`, `SelectPermanentEffect` by top-card id) as
  `dcgo_only`. With TWO legal Blast targets (or two Blast cards) the prompt is
  unanswerable today — same family as `G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK`
  (`NOTES-EX7-073.md`).
- A main-phase verb (`digivolve:`) written by the DEFENDER during the counter
  window lowers without complaint and then does nothing: the adapter applies a
  plain `Action` as the turn player, `resolve_selection` rejects it, and the
  selection stays pending until the next `pass`. Use `select:` there.
- `effect#3`'s cost pick is a `UnionZone` over `material`; the resolver scans
  hand and trash only, so the digivolution card is answered `yes:` /
  `sim_only` with DCGO's `SelectCardEffect` row `dcgo_only` (the P-180 idiom).
