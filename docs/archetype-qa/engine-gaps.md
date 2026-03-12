# Engine Gaps Tracker

This file accumulates engine mechanics that are missing or incomplete, discovered during archetype implementation. Each entry includes the card that exposed the gap and what engine change is needed.

Last updated: 2026-03-10

## Known Gaps

### From Prior Issues (MEMORY.md)

1. **Also Treated As (Name Aliasing)** — Cards like BT23-077 ("Also treated as [Sistermon Noir]") have name aliasing not modeled in engine. Scripts have `pass # descriptive-tagged: also_treated_as_name`.

2. **Disable Effect** — Cards like BT24-040 reference "Disable 1 of your opponent's Digimon's effects" but engine has no mechanic to selectively disable individual effects on a permanent. Scripts have `pass # descriptive-tagged: disable_effect`.

3. **Top/Bottom Deck Choice** — BT23-057 Gankoomon's cost reduction returns cards to deck but player should choose top or bottom for each card. Currently defaults to bottom. (See TODO in script.)

## Gaps Discovered During Archetype Implementation

### One-Shot Digivolve Cost Hook
- **Discovered in:** BG Imperial (2026-03-11)
- **Card(s):** BT3-103 — Hidden Potential Discovered!
- **Effect text:** "[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5."
- **What's missing:** Player-level temporary digivolution cost reduction hook that fires once on the next qualifying digivolve event, with suspend-as-cost.
- **Suggested change:** Add `player.register_one_shot_digivolve_hook(condition_fn, cost_fn, effect_fn)` API that intercepts the next digivolve and applies cost reduction + side effects.
- **Workaround:** None — BLOCKED. Security effect is implemented (play self free).

### Grant Triggered Effect to Opponent's Permanent
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT14-044 — Palmon
- **Effect text:** "[Start of Your Main Phase] 1 of your opponent's Digimon gains '[All Turns] When this Digimon becomes suspended, lose 2 memory.' until the end of their turn."
- **What's missing:** Ability to grant a temporary triggered effect (OnTappedAnyone → lose memory) to an opponent's permanent with expiry.
- **Suggested change:** Add `permanent.grant_temp_effect(ICardEffect, expiry)` that attaches an effect to a permanent with time-based removal.
- **Workaround:** Descriptive-tagged in script. Main structure present but process is a stub.

### Effect-Based Play Lock
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT9-047 — Pomumon
- **Effect text:** "[All Turns] Players can't play Digimon by effects."
- **What's missing:** A play-lock mechanism that prevents effect-based Digimon plays while allowing normal main-phase plays.
- **Suggested change:** Add `ModifierType.CANNOT_PLAY_BY_EFFECT` that `effect_play_from_zone` checks before executing.
- **Workaround:** Descriptive-tagged — field presence condition fires but no enforcement.

### Aura-Style CANNOT_UNSUSPEND for New Entries
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** BT12-057 — Quartzmon
- **Effect text:** "[All Turns] All other Digimon and Tamers don't unsuspend."
- **What's missing:** Permanents entering the field AFTER Quartzmon don't receive CANNOT_UNSUSPEND. No pre-unsuspend aura hook exists.
- **Suggested change:** Add aura-style modifier registration that auto-applies to new permanents entering the field.
- **Workaround:** CANNOT_UNSUSPEND applied to all permanents at digivolve time; new entries are not affected.

### Hand-Activated Main Effects on Digimon Cards
- **Discovered in:** Zephaga (2026-03-11)
- **Card(s):** EX11-032 — GrandGalemon
- **Effect text:** "[Hand] [Main] If you have [Shoto Kazama], by placing 1 [Galemon] from your trash as any of your [Pteromon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
- **What's missing:** Digimon cards with [Hand] [Main] effects that can be activated from hand during main phase (not a play or digivolve action). Requires special hand-activation UI flow.
- **Suggested change:** Add hand-activation action type for cards with [Hand] [Main] effects. The engine would need to check hand cards for activatable main effects during main phase.
- **Workaround:** Partially approximated with effect_digivolve_from_hand + add_card_source_bottom for the trash placement.

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
