# NOTES — Wisdom Training (P-108)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-108`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `P/Purple/P_108.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 3 clauses, 3 scenarios, 0 unreachable.** All three lower sim-only
with `tm_p_108_pool.json` (decks `tm-wisdom`, `quiet-opponent`).

| Clause | Text | Scenario |
|---|---|---|
| `P-108#effect#0` | `[Main] Reveal the top 2 cards of your deck. Add 1 purple card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.` | `P-108-effect0.yaml` |
| `P-108#effect#1` | `[Main] <Delay> … 1 of your Digimon may digivolve into a purple Digimon card in the hand with the digivolution cost reduced by 2.` | `P-108-effect1.yaml` |
| `P-108#effect#2` | `[Security] Place this card in the battle area.` | `P-108-effect2.yaml` |

## Adversarial pre-Unity review (from `P_108.cs`)

- `[Main]`: `SetUpActivateClass(null, …, -1, false, …)` → no `OptionalSkill`.
  `SimplifiedRevealDeckTopCardsAndSelect` with ONE condition → one
  `SelectCardEffect`; the single leftover is bottomed with no ordering prompt
  (our one-choice `OrderedPermutation` is answered `sim_only`, the documented
  `docs/DCGO_EXAM.md` row). One bucket → none of the multi-bucket add-timing
  quirk.
- `<Delay>`: `isOptional: false`; the Option is deleted first with no prompt,
  then `SelectPermanentEffect` (`canNoSelect: true`, single candidate still
  asked), then `DigivolveIntoHandOrTrashCard(isHand: true)` →
  `SelectHandEffect` (`canNoSelect: true`). Youkomon ST6-07's only circle is
  Purple Lv.3 / 2 → 0 after the reduction: one cost, no cost-choice prompt.
  Same shape as `../LM/LM-056-effect2.yaml`.
- `[Security]`: `PlaceSelfDelayOptionSecurityEffect` — mandatory, no prompt,
  and (unlike LM-056) it does **not** run the `[Main]`: no reveal.
- ST6-05 Elecmon / ST6-07 Youkomon are vanilla (no DCGO script); Pagumon
  ST6-01's inherited is `[On Deletion]` only.

## FINDING (engine, measured while authoring): a bare Digi-Egg does not satisfy an Option's colour requirement in our engine

First draft: hatch the purple Pagumon ST6-01 on T1 and play P-108 the same
turn. Lowering refused it — `step 1: no legal action matches Play { card:
"P-108" }` — with only the purple egg in the breeding area.

- DCGO allows it: `CardSource.MatchColorRequirement` (`CardSource.cs` ~305)
  walks `Owner.GetFieldPermanents()` and accepts any permanent whose
  `TopCard.IsPermanent`, and `CEntity_Base.IsPermanent` is
  `Digimon || Tamer || DigiEgg`. A hatched egg counts.
- Ours: `action/mask.rs::option_color_match_available` adds the breeding
  permanent's colours only `if breeding.is_digimon(card_data)`, and
  `Permanent::is_digimon` is `CardKind::Digimon | CardKind::Dual` — a
  `digi_egg` top card is excluded. (The pinning test
  `a_breeding_area_digimon_satisfies_the_color_requirement` uses a Lv.3.)

A Lv.2 in the breeding area is a Digimon for the rules (it is in play as an
In-Training Digimon), so DCGO's reading looks like the right one; not fixed
here. The committed lines breeding-digivolve into Elecmon first, which both
engines accept. The same asymmetry is what `../LM/LM-056-effect0.yaml` leans on
from the other side (its egg is the WRONG colour, so it is unaffected).

## SIDE FINDING (card YAML, not this card): ST1-02 / ST1-04 digivolve onto any Lv.2

The same failed lowering listed `Digivolve hand 2 onto breeding area` (Biyomon
ST1-02, red) and hand 3 (Dracomon ST1-04) as LEGAL over the PURPLE Pagumon:
`code/digimon-engine/cards/st1/ST1-02.yaml` authors its circle as
`from: { level_eq: 2 }` with no `color_is: red`. The printed circle is red.
Not exercised by any line here; recorded for the starter-deck owners.
