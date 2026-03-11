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

<!-- Entry template:
### {Gap Title}
- **Discovered in:** {archetype name} ({date})
- **Card(s):** {CARD_ID} — {card_name}
- **Effect text:** "{relevant text}"
- **What's missing:** {description of engine capability needed}
- **Suggested change:** {brief proposal}
- **Workaround:** {if any, otherwise "None — BLOCKED"}
-->
