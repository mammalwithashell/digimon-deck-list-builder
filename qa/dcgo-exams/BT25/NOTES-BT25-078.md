# NOTES — Gazimon (BT25-078)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-078`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `inherited#0`.
DCGO script: `BT25/Purple/BT25_078.cs` exists — the card is **not** `unavailable`.
All four clauses have a scenario; none is `unreachable`.

| Clause | Text | Scenario | Book | Sim-only |
|---|---|---|---|---|
| `BT25-078#effect#0` | `[Digivolve] Lv.2 w/[Three Musketeers] in text or w/[TS] trait: Cost 0` | `BT25-078-effect0.yaml` | `BT25/tm_bt25_078_pool.json` (`three-musketeers-gazimon`: Black [TS] egg BT25-005, Youkomon ST6-07 x2 for EX7-070 x2) | lowers, asserts pass |
| `BT25-078#effect#1` | `[When Moving]` (the shared body through the OTHER timing; "place as bottom digivolution card" branch) | `BT25-078-effect1.yaml` | `EX7/three_musketeers_pool.json` | lowers, asserts pass (repaired 2026-09-18) |
| `BT25-078#effect#2` | `[On Play] Reveal the top 3 … add 1 … to the hand, or you may place 1 … Return the rest to the bottom of the deck.` ("add to hand" branch) | `BT25-078-effect2.yaml` | `EX7/three_musketeers_pool.json` | lowers, asserts pass (repaired 2026-09-18) |
| `BT25-078#inherited#0` | `<Retaliation>` | `BT25-078-inherited0.yaml` | `BT25/tm_bt25_078_pool.json` | lowers, asserts pass incl. the clause's witness (`p1.field: []`, `p1.trash: [BT4-014]`); **oracle CONFIRMED 2026-09-18 after the engine fix below** |

Note the two books: `effect#1`/`effect#2` need the shared pool (Purple egg
ST6-01), `effect#0`/`inherited#0` the per-card one. Lowering all four with one
`--decks` fails two of them on `unknown deck` — that is a book mismatch, not a
broken scenario.

## Repairs made on resume (2026-09-18) — the crashed author's drafts did not lower

- `effect#1`: Gazimon was never stacked into the opening hand; the draw budget
  forgot that a digivolution draws 1 (the reveal is `stack[13..15]`, not
  `[12..14]`); our `EffectChoice` was answered with `value: 1`, which cannot be
  resolved (now `choice: "bottom digivolution card", sim_only: true`, the
  `d5f23792f` form).
- `effect#2`: same `value: 0` → `choice: "card to hand"`; and the line asserted
  "memory 3 → 0 on turn 1" — T1 memory is 0, the play crosses to -3 and the turn
  passes, so the trailing step is P1's breeding decision.
- **Both: DCGO's prompt ORDER was wrong.** `BT25_078.cs` calls
  `SimplifiedRevealDeckTopCardsAndSelect(... RemainingCardsPlace.DeckBottom)`
  and only AFTER it returns asks its add-or-place `generic_bool` and moves the
  picked card. The deck-bottom ordering `SelectCardEffect`
  (`ReturnRevealedCardsToLibraryBottom`, 2 leftovers) is asked INSIDE the helper,
  so DCGO's sequence is **pick → ordering → [generic_bool] → move**. The draft
  had the bool before the ordering (`effect#1`) and no row at all for OUR
  `OrderedPermutation { remaining: 2 }` park (both files).
- **The ordering row is split `sim_only` + `dcgo_only`, on purpose.** Our
  `choose_from_reveal` moves the picked card at pick time, DCGO after the
  ordering; a shared ordering row would compare those two pre-decision
  snapshots and report a `p0.hand` / `p0.field[0].sources` divergence that is
  pure sequencing (nothing observable can happen between the two points). The
  pick itself is the one SHARED decision on each line.

## Prompt-existence differences folded on these lines (each is a C# fact)

- Outer accept/decline gate: `BT25-078.yaml` authors the clause `optional: true`;
  DCGO registers it `optional: false` (`ActivateClassesForSharedEffects`) and
  carries the "may" on the pick's `canNoSelect: true`. `sim_only`.
- Branch choice: ours asks hand-or-source FIRST (`select_effect_choice`); DCGO
  decides it AFTER the pick, and only asks (`SetBoolSelection` → `generic_bool`)
  when the picked card has the [Three Musketeers] TRAIT — for a text-only card
  (Sparrowmon) it takes the silent `SetBool(true)` path, which is not the hooked
  RPC and consumes no row.

## TRIAGE 2026-09-18 — `inherited#0` was OUR BUG; ENGINE FIX `a46c74066`; re-diffed CLEAN, verdict `confirmed`

Order of evidence: printed text (card face / `data/card_bundles/BT25-078.md`:
inherited `<Retaliation>`), general_rule.pdf **16-12-1/-3/-4** (mandatory; the
triggered instance activates "as long as the battled opponent's Digimon is in
the battle area"), DCGO `BT25_078.cs:113-115` (one
`RetaliationSelfEffect(isInheritedEffect: true)` under `OnDestroyedAnyone`).
All three agree with DCGO's recording; ours was wrong.

**The diagnosis in the section below is superseded on defect 1.** The second,
keyword-less TriggerOrder branch was NOT the `[When Moving]` body — `on_move` is
only ever enqueued by `move_from_breeding`. It was a DUPLICATE `<Retaliation>`:
the engine synthesized the keyword's trigger once from cards.json
`inherited_text` and once from the YAML's `scope: inherited` `grant_keyword`
clause. Both queued while the card was under the top; resolved from the trash,
the effect list was one entry shorter (`under_top` gating), so the second
branch's slot fell off the end — hence "BT25-078" with no keyword and a no-op.
Defect 2 (battle opponent read at resolution time) was exactly as described and
is `G-ONDELETION-PARK-CLEARS-BATTLE-STATE` in `docs/RUST_ENGINE_GAPS.md`, now
RESOLVED. A third defect surfaced on the way: `CardData::keywords` (parsed from
all three text fields) put the inherited `<Retaliation>` on Gazimon's own FACE.

The scenario dropped its `sim_only` TriggerOrder row (no prompt opens in either
engine now) and asserts P1's state. Same fix closed `EX7-051#inherited#0`.
Tests: `tests/cards_behavioral/bt25/bt25_078.rs` §5.

## (superseded) Sim-side engine finding on `inherited#0` — `<Retaliation>` does not delete when a second trigger is stacked beside it

Re-measured 2026-09-18 on the current harness binary (probe: the committed line
plus `p1.field: []`): our engine leaves Vermilimon BT4-014 on P1's field, 8000
DP, suspended. At Youkomon's deletion our engine parks a 2-candidate
`TriggerOrder` `[BT25-078 <Retaliation>, BT25-078]` — the second branch is
Gazimon's [When Moving] body, queued by the board-wide `on_move` observer scan
when the stack leaves the battle area (it no-ops at resolution: its
`event_permanent_is_source` timing condition fails). By the time
`<Retaliation>` resolves, the attack context is gone and `battle_opponent_of`
finds nothing to delete. Two defects, one line:

1. a deletion is not a move — `on_move` should not enqueue on it (DCGO's
   `WhenMovingClass` `CanTriggerOnMove` never fires here; DCGO has exactly ONE
   trigger and opens no `MultipleSkills`);
2. `<Retaliation>` (16-12, Mandatory) must still delete the Digimon it battled
   when its resolution is deferred behind an ordering prompt — the battle
   opponent has to be captured at trigger time.

The scenario answers the extra `TriggerOrder` `sim_only` (named by keyword) and
does NOT assert `p1.field` / `p1.trash`, so the file passes the sim-only gate;
the oracle run is expected to read `diverged` at the trailing row (DCGO deletes
Vermilimon, we do not). That `diverged` is the finding this line exists to
surface. Not fixed here (exam stage).

## Card-data finding — `BT25-078.yaml` prints Gazimon BLACK

`code/digimon-engine/cards/bt25/BT25-078.yaml` authors `color: [black]` and a
`Lv.2 Black / 0` alt-path. The official Bandai DB (`data/card_bundles/BT25-078.md`),
the card face and DCGO (`BT25/Purple/`) all print it **Purple**, circle
`Purple Lv.2 / 0`. `data/cards.json` has the right colour and circle, which is
why the Purple-egg digivolve on `effect#1` lowers; the YAML's extra Black route
additionally admits a Black Lv.2 WITHOUT [TS]/[Three Musketeers] text, which the
card does not print. On `effect#0` the mislabel is not separable from the clause
(BT25-005 is Black AND [TS], so either path admits it). Fix belongs to the
card-fix gate, not this stage.
