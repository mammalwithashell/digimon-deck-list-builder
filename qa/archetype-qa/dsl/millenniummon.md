# Millenniummon Rust DSL / Engine Gap Source

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Assessment target: `data/deck_library.json` archetype `Millenniummon`, with focus on the high-frequency BT18/BT19/Promo core and support options that define the current deck shell.

Purpose: source document for compiling a cross-archetype Rust DSL / engine gap spec. This is not an implementation plan and does not bless placeholders or hidden auto-selection. Each gap below is written as a reusable capability that should be grouped with blockers from other archetype audits before implementation.

## Verdict

`blocked`

Millenniummon is not currently ready as executable Rust DSL card content. The current Rust registry exposes only `BT18-019` from the assessed core, and that card is embedded from `_examples` rather than a production archetype card set. The legacy Python-era QA report in `qa/archetype-qa/millenniummon.md` is useful behavioral reference, but it must not be read as Rust DSL readiness evidence.

The most important reusable blockers are:

- Forced opponent hand reduction to a target hand size, with the opponent choosing cards and the effect receiving the number trashed.
- Effect-initiated DNA digivolve using one material in the battle area and one material from trash.
- Full DigiXros play flow, including action-mask legality, material selection, cost reduction, and material placement under the played Digimon.
- Production YAML and card-specific behavioral tests for the archetype core.

## Registry Evidence

Evidence command:

```powershell
python -c "import digimon_engine; ids=digimon_engine.load_implemented_card_ids(); targets=['BT18-015','BT18-019','BT18-073','BT19-065','BT19-070','BT19-075','BT19-101','P-193','P-205','P-220','EX1-066']; print({t:(t in ids) for t in targets}); print('count', len(ids))"
```

Observed result:

```text
{'BT18-015': False, 'BT18-019': True, 'BT18-073': False, 'BT19-065': False, 'BT19-070': False, 'BT19-075': False, 'BT19-101': False, 'P-193': False, 'P-205': False, 'P-220': False, 'EX1-066': False}
count 105
WARNING: raw_rust budget exceeded: 20 raw_rust fns for 73 DSL cards (27.4%)
```

## Card Readiness

| Card | Role | Status | Evidence | Remaining blocker |
|---|---|---|---|---|
| `BT18-019` Millenniummon | Primary Lv.7, DNA boss, opponent-trash deck return, trash recursion | test-gap | `code/digimon-engine/cards/_examples/BT18-019.yaml`; `code/digimon-engine/tests/effect_context/effect_initiated_dna_digivolve.rs` has reusable DNA trigger coverage | Move out of example-only status or explicitly accept example pack ownership; add card-specific behavioral regression for DNA On Play/When Digivolving, opponent-trash return choices, memory gain, and On Deletion trash replay |
| `BT18-015` Kimeramon | On-deletion DNA bridge into Millenniummon | engine-gap / dsl-gap | Printed text requires DNA using one in-play Machinedramon and one Kimeramon in trash | `G-MILL-EFFECT-DNA-TRASH-MATERIAL` |
| `BT18-073` Machinedramon | Alternative digivolve, De-Digivolve, on-deletion DNA bridge | engine-gap / dsl-gap | Current `effect_initiated_dna_digivolve` helper accepts two battle-area permanents, not trash material | `G-MILL-EFFECT-DNA-TRASH-MATERIAL` |
| `BT19-065` Machinedramon | DigiXros Lv.6, deletion and trash play bridge | engine-gap / dsl-gap | `docs/RUST_PYTHON_PARITY.md` tracks full DigiXros play flow as outstanding | `G-MILL-DIGIXROS-PLAY-FLOW` |
| `BT19-070` Kimeramon | Own deletion cost, multi-level opponent deletion, trash play bridge | card-yaml / test-gap | Behavior appears expressible with existing selection and zone-movement primitives, but card is not registered | Author YAML and add exact choice/mask tests |
| `BT19-075` MoonMillenniummon | Opponent hand reduction, Tamer deletion rider, leave-area replacement, security trash observer | engine-gap / dsl-gap | `docs/RUST_ENGINE_GAPS.md` tracks missing forced opponent hand reduction primitive | `G-MILL-FORCED-OPP-HAND-TO-COUNT` |
| `BT19-101` ZeedMillenniummon | Overclock, trash-to-deck cost, bottom-deck removal, no-source immunity | card-yaml / test-gap | Group 6 primitives cover Overclock and source-scoped immunity, but card is not registered | Author YAML and add source-count immunity, Overclock, and opponent-trash-return tests |
| `P-193` The Wicked God Emerges! | Draw/trash/place option, Delay play Wicked God | card-yaml / test-gap | Delay placement is implemented in the reusable roadmap, but this card is not registered | Author YAML and add Delay timing/free-play tests |
| `P-205` Insane Synthetic Monster | Color bypass with Digital Monster, draw/trash/place, Delay reduced play from trash | card-yaml / test-gap | Option color bypass and Delay placement have reusable support, but this card is not registered | Author YAML and add color-bypass, trash selection, and reduced-play tests |
| `BT19-099` The Wicked God Descends! | Reduced trash play, placed option, leave-field Delay replacement ladder | card-yaml / test-gap | Delay/replacement framework has landed for several cases, but this specific leave-field upgrade flow is untested | Author YAML and add leave-field replacement/free-play tests |
| `P-220` Millenniummon | Reboot, Blocker, DNA, De-Digivolve, trash-return cost, two trash plays by different levels | card-yaml / test-gap | Reboot, Blocker, De-Digivolve, and distinct-count selection primitives exist, but card is not registered | Author YAML and add multi-trash-return cost plus two-play distinct-level regression |
| `EX1-066` Analog Youth | Common search/trash support | card-yaml / test-gap | High-frequency support card, not registered | Author or reuse existing support-card YAML and add search/on-deletion memory-hatch regression |

## Reusable Gaps

### G-MILL-FORCED-OPP-HAND-TO-COUNT

- **Type:** engine-gap / dsl-gap
- **Primary Millenniummon blocker:** `BT19-075`
- **Cross-archetype capability:** Force a target player to reduce hand size to a printed threshold, while that target player chooses which cards leave hand.
- **Required behavior:** The helper must install a pending selection owned by the affected player, move exactly the selected hand cards to trash, return the count trashed to the effect process, and allow downstream riders such as "for every 2 cards trashed" to use that count.
- **No-approximations note:** The active player must not auto-select opponent hand cards. Unknown hand identity for the non-owner UI does not remove the opponent's right to choose.
- **Known tracker:** `docs/RUST_ENGINE_GAPS.md` has a forced opponent hand reduction entry for `BT19-075`.
- **First test:** With `BT19-075` resolving against an opponent with 8 cards in hand and two opponent Tamers in play, assert the opponent receives a three-card discard selection, the chosen cards move to trash, and the controller may delete exactly one Tamer from the "for every 2" rider.

### G-MILL-EFFECT-DNA-TRASH-MATERIAL

- **Type:** engine-gap / dsl-gap
- **Primary Millenniummon blockers:** `BT18-015`, `BT18-073`
- **Cross-archetype capability:** Effect-initiated DNA digivolve where one or more materials come from non-battle zones, especially trash, while preserving material ordering, event payloads, and On DNA / When Digivolving trigger semantics.
- **Required behavior:** The engine must expose legal choices for the in-play material, trash material, and hand card being digivolved into. Resolution must build the resulting stack faithfully, remove the trash material from trash, and emit the same digivolution event shape used by normal DNA digivolution plus a clear effect-origin marker.
- **No-approximations note:** Do not silently choose the first matching trash card or the first matching hand card.
- **Known tracker:** No single tracker row currently names this exact Millenniummon form. It should be added to `docs/RUST_ENGINE_GAPS.md` or `qa/archetype-qa/engine-gaps.md` before implementation.
- **First test:** With `BT18-073` deleted, a `BT19-070`/Kimeramon in battle, a matching Machinedramon in trash, and `BT18-019` in hand, assert a pending DNA choice is offered, chosen trash material becomes a source, and `BT18-019` resolves its DNA-gated On Play/When Digivolving branch.

### G-MILL-DIGIXROS-PLAY-FLOW

- **Type:** engine-gap / dsl-gap / data-gap
- **Primary Millenniummon blocker:** `BT19-065`
- **Cross-archetype capability:** Full DigiXros play flow for cards with material recipes and per-material cost reduction.
- **Required behavior:** Card data/YAML must describe DigiXros recipes; action-mask generation must surface legal play actions at the reduced cost only when valid materials are available; resolution must prompt material choices, apply the printed cost reduction, play the card, and place selected materials under it in the correct order.
- **No-approximations note:** A fixed reduced play cost without material selection is not faithful and is not sufficient for training.
- **Known tracker:** `docs/RUST_ENGINE_GAPS.md` notes DigiXros alias support but keeps full DigiXros play flow and recipe-cost UX separate. `docs/RUST_PYTHON_PARITY.md` tracks DigiXros cost reduction as outstanding.
- **First test:** With `BT19-065` in hand and five unique Lv.5-or-lower Cyborg/Composite candidates across allowed zones, assert the main action mask exposes a reduced-cost play, prompts material choices, moves selected materials under `BT19-065`, and rejects duplicate card numbers where the printed recipe requires different numbers.

### G-MILL-CARD-YAML-REGRESSION-BATCH

- **Type:** card-yaml / test-gap
- **Primary Millenniummon blockers:** all core cards except example-pack `BT18-019`
- **Cross-archetype capability:** Archetype unlock checkpoints must distinguish reusable primitive readiness from production card ownership.
- **Required behavior:** For each card moved into production YAML, add a focused behavioral test that proves all player-visible choices are mask-backed and all optional branches can be declined.
- **No-approximations note:** Legacy Python QA is reference evidence only. It cannot close Rust DSL readiness by itself.
- **Known tracker:** Group 9 archetype unlock work in `docs/superpowers/plans/2026-05-02-gap-group-9-archetype-unlocks.md`.
- **First test batch:** Add `code/digimon-engine/tests/cards_behavioral/bt18/`, `bt19/`, and `p/` cases for `BT18-019`, `BT19-075`, `BT19-065`, and `P-220` before broad support cards.

## Resolved Or Partially Resolved Prerequisites

These capabilities should be referenced as dependencies rather than reopened as new gaps unless Millenniummon card-specific tests find a regression:

- **Overclock:** reusable tests exist under `code/digimon-engine/tests/combat/group6_overclock.rs`; needed by `BT19-101`.
- **Reboot / Blocker / source-scoped immunity:** Group 6 docs mark core modifier/keyword support resolved; needed by `P-220` and `BT19-101`.
- **De-Digivolve:** existing engine primitives are sufficient for `BT18-073` and `P-220`, subject to card-specific tests.
- **Delay placement:** reusable Delay placement support is tracked as resolved in `qa/dsl-vocab-gaps.md`; needed by `P-193`, `P-205`, and `BT19-099`.
- **Distinct-by multi-selection:** `select_count_capped_multi` with `distinct_by` exists in the DSL and is used by the `BT18-019` example.

## Spec Compilation Notes

When folding these findings into the next cross-archetype spec:

1. Merge `G-MILL-FORCED-OPP-HAND-TO-COUNT` into a selection/action-mask group, because the selecting player is the opponent and the hand contents are hidden from the controller.
2. Merge `G-MILL-EFFECT-DNA-TRASH-MATERIAL` with non-hand/non-battle digivolution work, not with simple DNA cost-data authoring. It needs stack construction and event payload semantics.
3. Merge `G-MILL-DIGIXROS-PLAY-FLOW` with other cost/material play-flow gaps. This is broader than a YAML alias and must be proven at action-mask time.
4. Keep `G-MILL-CARD-YAML-REGRESSION-BATCH` in the archetype unlock group after reusable primitives land.
5. Do not expand `ACTION_SPACE_SIZE`, tensor layouts, or model metadata as part of these card unlocks unless a newly discovered player-visible choice cannot be represented by existing pending-selection/action IDs. If that happens, route it through `docs/ACTION_SPEC.md` and `docs/TENSOR_SPEC.md` as a contract change.

## Suggested Implementation Order

1. Add tracker rows for `G-MILL-EFFECT-DNA-TRASH-MATERIAL`, then write the failing trash-material DNA regression.
2. Implement forced opponent hand-to-count and prove `BT19-075` hand reduction plus Tamer rider.
3. Implement full DigiXros play flow and prove `BT19-065`.
4. Promote or relocate `BT18-019` from example-only status into the intended production ownership path and add card-specific tests.
5. Author `BT19-070`, `BT19-101`, `P-193`, `P-205`, `BT19-099`, and `P-220` YAML only after their reusable primitives have tests.
6. Re-run the archetype readiness workflow and update `qa/archetype-qa/millenniummon.md`, this file, and the central gap trackers with exact commands.
