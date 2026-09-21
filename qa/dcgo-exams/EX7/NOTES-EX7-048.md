# NOTES — Gundramon (EX7-048)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-048`):
**4 clauses** — `effect#0` … `effect#3`. DCGO script `EX7/Black/EX7_048.cs`
exists — the card is **not** `unavailable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/EX7-048.json` does not exist): all four are
`unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-048#effect#0` | `[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4` | `EX7-048-effect0.yaml` | lowers, asserts pass |
| `EX7-048#effect#1` | `<Blocker>` | `EX7-048-effect1.yaml` | lowers, asserts pass |
| `EX7-048#effect#2` | `[On Play] [When Digivolving] Reveal the top 6 … You may use 1 [Three Musketeers] trait Option card among them without paying the cost. Return the rest to the top or bottom of the deck.` | `EX7-048-effect2.yaml` ([When Digivolving] arm, use taken) | lowers, asserts pass |
| `EX7-048#effect#3` | `[All Turns] When any of your [Three Musketeers] trait Digimon would leave the battle area other than by your effects, by trashing 1 Option card from this Digimon's digivolution cards, they don't leave.` | `EX7-048-effect3.yaml` | lowers, asserts pass |

Book: `qa/dcgo-exams/EX7/tm_ex7_048_pool.json` (decks `tm-gundramon`,
`tm-red-opponent-048`). The opponent deck swaps BT1-014 ×2 for **Tai Kamiya
ST1-12 ×2**: `effect#3` needs a red PERMANENT for Gaia Force that is not a
Digimon (see that file's header). Both decks stay at 50 + 4 eggs.

Every line enters through Megadramon (EX7-011, red Lv.5) → Gundramon for 4, on
a ONE-Digimon board. The mandatory `[When Digivolving]` reveal therefore rides
on all four files; `effect#0` / `effect#1` keep it at its smallest (no
candidate among the six → no "use" pick, only the top-or-bottom return).

## Prompt-shape facts (re-derived from `EX7_048.cs` + `RevealLibrary.cs`)

- Shared OP/WD: `ActivateClassesForSharedEffects(optional: false)` — mandatory,
  no `OptionalSkill`. `SimplifiedRevealDeckTopCardsAndSelect(6, ONE condition,
  maxCount 1, canNoSelect: true, DeckTopOrBottom)`:
  `maxCount = Math.Min(1, revealed.Count(cond))`, so with no candidate the
  `SelectCardEffect` is skipped entirely; with one, it is asked once and the
  condition's `selectCardCoroutine` (`PlayOptionCards(payCost: false, root:
  Library)`) runs when it closes — i.e. the Option resolves BEFORE the
  remainder is placed (ours: `use_option_from_revealed` precedes
  `place_remainder_on_deck`).
- `DeckTopOrBottom`: a `generic_bool` ("Deck Top" = true / "Deck Bottom" =
  false) then, for 2+ cards, ONE ordering `SelectCardEffect`. Ours: an
  `EffectChoice` named by label + a position-by-position `OrderedPermutation`.
  Split `sim_only` / `dcgo_only`, the confirmed `BT25-078-effect1.yaml` idiom
  (`decline: true, dcgo_only` = `select_bool: false`).
- `[All Turns]` (`EffectTiming.WhenRemoveField`, `isOptional: true`):
  `OptionalSkill` → `SelectPermanentEffect(canNoSelect: true)` "Select 1 Digimon
  to prevent the deletion of" → `SelectCardEffect(Root.Custom, canNoSelect: ()
  => true)`. Our `kind: replacement` is per-subject, so DCGO's "which Digimon"
  row is `dcgo_only`.

## ENGINE FINDING — a FREE Option use fizzles when the printed cost is "unaffordable"

Measured while authoring `effect#2`'s first draft (the `[On Play]` arm):
hard-play Gundramon (12: 3 → −9), pick P-180 from the reveal. Our engine
accepted the pick and then **silently did not use the Option** — P1's security
stayed 5, nothing was tucked, and P-180 went back to the deck with the
remainder (the 5-card ordering row left `OrderedPermutation { remaining: 1 }`
= P-180). Same result with the black Der Blitz (EX7-070), so colour is not the
cause. On the `[When Digivolving]` arm (gauge −1) the identical pick works.

Cause: `Game::option_legal_play_modes` (`game_actions/mod.rs` ~915) filters
modes by `(self.memory - use_cost) >= memory_min` using the PRINTED use cost,
with no regard for `OptionCostPolicy::Free`. At −9, a cost-6 Option gives −15 <
−10 → no legal mode → `play_option_core` returns `Invalid`, and
`use_option_from_revealed` discards the result. "Without paying the cost" pays
nothing, so no affordability test applies (DCGO: `PlayOptionCards(payCost:
false)`). Class: **our bug** (engine; reaches every effect-driven free or
reduced Option use — `use_option_from_hand/trash/revealed/source` — whenever
the gauge is already deep on the opponent's side, which is exactly when a
12-cost body has just been hard-played). Proposed id
`G-ENGINE-FREE-OPTION-USE-GATED-ON-PRINTED-COST`. **Not fixed here.**

Consequence for this card: the `[On Play]` arm of `effect#2` cannot be measured
cleanly today — an oracle run would abort at DCGO's tuck
`SelectPermanentEffect` (a prompt we never park) and record this gap, not the
clause. The committed file measures the `[When Digivolving]` arm, where both
engines walk the same sequence. The On-Play probe, for whoever fixes the
engine (book `tm_ex7_048_pool.json`):

```yaml
decks:
  p0: { stack: [EX7-048, ST1-02, EX7-008, BT25-082, EX7-010,
                BT25-078, BT25-078, EX7-008, EX7-051, EX7-051,
                EX7-010,
                BT25-083, P-180, BT25-083, BT25-082, EX7-051, EX7-008], rest: tm-gundramon }
steps:            # T1 pass/pass, T2 pass/pass, T3 breeding pass, then:
  - { actor: 0, do: { play: { card: EX7-048, from: hand } } }        # 3 -> -9
  - { actor: 0, do: { select: { cards: [P-180] } } }                 # SelectCardEffect
  - { actor: 0, do: { select: { targets: [own.field.0] } } }         # P-180's tuck -- ours never asks it
```
