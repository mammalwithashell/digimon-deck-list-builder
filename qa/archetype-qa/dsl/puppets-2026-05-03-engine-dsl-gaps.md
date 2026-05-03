# Puppets Rust Engine / DSL Gap Inventory

Date: 2026-05-03
Archetype: Puppets / Nyabootmon
Assessment source: `data/deck_library.json` archetype `Puppets`
Rust target: `code/digimon-engine/` plus YAML DSL under `code/digimon-engine/cards/`
Verdict: blocked

This file captures reusable Rust engine and DSL gaps surfaced by the Puppets archetype so they can be folded into a cross-archetype tracking spec. It is intentionally stricter than `qa/archetype-qa/Puppets.md`, which is a legacy Python-lane faithfulness report.

## Assessment Target

`data/deck_library.json` has 23 local `Puppets` decklists, with `BT22-042` Nyabootmon as the display card. `data/archetype_aliases.json` maps `Nyabootmon` as an alias for `Puppets`.

High-frequency core cards across those lists:

| Card | Name | Frequency | Core role |
|---|---:|---:|---|
| `BT22-042` | Nyabootmon | 23/23 | top-end Overclock, play Lv4 or lower Puppet, re-fire When Digivolving |
| `EX9-024` | Hanimon | 23/23 | hand-trash cost, Puppet recursion |
| `EX9-032` | Karakurumon | 23/23 | delete Token/Puppet to effect-digivolve |
| `EX9-033` | Kaguyamon | 23/23 | Alliance/Blocker aura, lowest-level delete, trash play |
| `EX9-067` | Mirai Kinosaki | 23/23 | reveal search, digivolve observer, reduced-cost play |
| `ST19-03` | Shoemon | 23/23 | reveal search, inherited security-DP aura |
| `P-165` | ShoeShoemon | 22/23 | Security play, Familiar token |
| `EX7-024` | Shoemon | 19/23 | Puppet digivolve cost reduction |
| `ST19-14` | Arisa Kinosaki | 19/23 | memory setter, Token/Puppet play observer, Rush grant |
| `BT16-055` | Namakemon | 18/23 | keyword protection/grant package |
| `BT22-002` | Kyaromon | 18/23 | inherited draw on Token/Puppet deletion |
| `EX7-027` | Chaperomon | 18/23 | Overclock, play Lv3 Puppet, prevent leave |

Newer `EX11` cards are lower-frequency in this snapshot but important for the same reusable gaps: `EX11-019`, `EX11-021`, `EX11-022`, `EX11-023`, `EX11-024`, `EX11-060`, and `EX11-061`.

## Current Implementation Evidence

- Production effects are embedded from YAML under `code/digimon-engine/cards/` by `code/digimon-engine/build.rs`, then registered through `code/digimon-engine/src/cards.rs`.
- There is no production `code/digimon-engine/cards/bt22/` directory in this worktree.
- `code/digimon-engine/cards/ex11/` currently contains only `EX11-008.yaml`, `EX11-012.yaml`, and `EX11-054.yaml`; the Puppet `EX11-019` through `EX11-024` and `EX11-060`/`EX11-061` package is not authored there.
- `code/digimon-engine/cards/ex9/` contains only `EX9-013.yaml`; the `EX9-024`/`EX9-027`/`EX9-032`/`EX9-033`/`EX9-067` Puppet core is not authored there.
- `code/digimon-engine/cards/p/` lacks `P-165.yaml` and `P-229.yaml`.
- Engine support exists for several reusable pieces: Overclock pending cost/mask flow, Familiar token behavior, Scapegoat/Barrier keyword auto-effects, Alliance interrupt flow, effect-initiated play/digivolve primitives, and reveal/add-to-hand movement.

## Gap Summary

| Gap ID | Type | Status | Blocks | Canonical tracker |
|---|---|---|---|---|
| `PUPPETS-G001` | dsl-gap / test-gap | open | Most Puppet core cards | none; archetype-local authoring backlog |
| `PUPPETS-G002` | engine-gap / dsl-gap | open | `BT22-042`, `BT22-040`, `EX11-024` | `qa/archetype-qa/engine-gaps.md` |
| `PUPPETS-G003` | engine-gap | open | `EX11-022`, `EX11-061`, related effect-play cleanup cards | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G004` | hybrid | partially resolved | `BT22-098`, `P-229` | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G005` | engine-gap / test-gap | open | `EX9-067`, `EX11-061`, `ST19-14`, `BT22-088` | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G006` | engine-gap / test-gap | open | `P-165`, `ST19-08`, other security end-of-battle play effects | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G007` | test-gap | open | `EX9-033`, `EX11-023`, Puppet aura package | none; add card-level regression coverage |

## Detailed Gaps

### PUPPETS-G001: Production YAML Missing for Core Puppet Package

- **Type:** `dsl-gap` / `test-gap`
- **Status:** open
- **Blocks:** `BT22-042`, `EX9-024`, `EX9-032`, `EX9-033`, `EX9-067`, `ST19-03`, `P-165`, `EX7-024`, `ST19-14`, `BT22-002`, `EX7-027`, and most `EX11` Puppet cards.
- **Why it matters:** The Rust runtime only executes production card behavior that is registered from the embedded DSL pack or explicit Rust effects. The Puppet archetype cannot be used as a serious Rust training/evaluation target while its core cards are metadata-only.
- **Evidence:** The relevant set/card YAML files are absent from `code/digimon-engine/cards/`; only unrelated or adjacent cards are present in `ex9/`, `ex11/`, and `p/`.
- **First test:** Add a card-level DSL registration test for `BT22-042` and assert the compiled card has Overclock, When Digivolving, and other-deletion reactivation clauses.
- **Implementation hint:** Start with production YAML under `code/digimon-engine/cards/bt22/BT22-042.yaml` and card-level tests under `code/digimon-engine/tests/cards_behavioral/bt22/`.

### PUPPETS-G002: Re-Activate a Card's `[When Digivolving]` Effect From Another Trigger

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT22-042` Nyabootmon, `BT22-040` Cendrillmon, `EX11-024` Cendrillmon.
- **Effect text:** "When any of your other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects."
- **Why it matters:** This is a core Puppet payoff. The engine needs to enumerate eligible `[When Digivolving]` effects, expose the player choice if more than one branch is legal, and execute the selected effect with correct source attribution.
- **Evidence:** Existing gap tracker `qa/archetype-qa/engine-gaps.md` has "Activate Another Card's When Digivolving Effect", including `BT22-042`.
- **First test:** Put `BT22-042` in battle, delete another own Puppet, and assert the mask exposes a may-choice to activate the When Digivolving play/DP-reduction branch rather than auto-firing or no-oping.
- **Implementation hint:** Add a reusable effect-selection helper that can run a source card's `[When Digivolving]` effects from a non-digivolve trigger while preserving `source_permanent`, `source_card`, once-per-turn accounting, and pending-selection ordering.

### PUPPETS-G003: Effect-Played Permanent Provenance and Scheduled Turn-End Cleanup

- **Type:** `engine-gap`
- **Status:** open
- **Blocks:** `EX11-022` Karakurumon, `EX11-061` Mirai Kinosaki, and any Puppet effect that says to delete "the Digimon this effect played" at turn end.
- **Effect text examples:** "At turn end, delete the Digimon this effect played."
- **Why it matters:** The engine must remember exactly which permanent was played by a specific effect, then delete that permanent later if it is still present. Index-based battle-area references are not enough because other deletes/plays can shift slots.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` includes the reusable provenance/scheduled cleanup gap for "delete the Digimon this effect played".
- **First test:** Resolve `EX11-022` to play a 4000 DP or less Puppet from hand or trash, shift battle-area indices before turn end, and assert only the effect-played permanent is deleted.
- **Implementation hint:** Store a stable provenance key on effect-played permanents or schedule a cleanup keyed by stable `CardSource` identity rather than battle-area index.

### PUPPETS-G004: Event-Gated Delay Windows for Puppet Options

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** partially resolved
- **Blocks:** `BT22-098` Unique Emblem: Fable Waltz, `P-229` Unique Emblem: Narrative Ronde.
- **Effect text:** `BT22-098` delays when Arisa suspends; `P-229` delays when Mirai Kinosaki is played.
- **Why it matters:** These options are important Puppet consistency tools and must place into battle, watch later events, then expose the delayed digivolve choice through pending selections.
- **Evidence:** `qa/dsl-vocab-gaps.md` records this as partially resolved for `BT22-098`'s `on_suspend` slice. `P-229` remains blocked while `on_ally_played` is virtual/skipped and the process body still needs faithful reduced-cost effect digivolve.
- **First test:** Play `P-229`, play `Mirai Kinosaki` on a later turn, and assert the delayed option can be trashed to offer a level 6 or lower `LIBERATOR` digivolution from hand with cost reduced by 3.
- **Implementation hint:** Lower `on_ally_played` to a real entered-field event predicate or a more explicit `on_enter_field_anyone` condition, then reuse the event-gated Delay lifecycle.

### PUPPETS-G005: Effect-Initiated Play/Digivolve Event Context and "By This Effect" Filters

- **Type:** `engine-gap` / `test-gap`
- **Status:** open
- **Blocks:** `EX9-067`, `EX11-061`, `ST19-14`, `BT22-088`, and related Tamer observer loops.
- **Effect text examples:** "When any of your Digimon digivolve into a [Puppet] trait Digimon..." and "When effects play one of your Tokens or a Digimon with the [Puppet] trait..."
- **Why it matters:** Puppet Tamers care whether a Digimon was played or digivolved by effects, and they often immediately play or grant keywords to the entered/digivolved permanent. Normal hand play/digivolve event context has coverage, but effect-initiated paths need to prove the same context and provenance.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` notes follow-up paths remain open for effect-created permanents, token play, play-from-trash context, and effect-initiated digivolve context unless separately tested.
- **First test:** Trigger `EX11-061` by effect-digivolving into a Puppet and assert Mirai can suspend to play a level 3 Puppet from hand, then schedule the correct turn-end cleanup for that played card.
- **Implementation hint:** Thread `PlaySource::ByEffect` / effect source identity into `OnEnterFieldAnyone` and `OnDigivolve` trigger context for every effect-created play/digivolve path.

### PUPPETS-G006: Security End-of-Battle Play for Puppet Digimon

- **Type:** `engine-gap` / `test-gap`
- **Status:** open
- **Blocks:** `P-165` ShoeShoemon and similar cards whose security text resolves at end of battle.
- **Effect text:** "[Security] At the end of the battle, play this card without paying the cost."
- **Why it matters:** The timing is not just normal security reveal. The card must survive the security battle window long enough to play itself at the correct sub-timing, fire On Play, and avoid being trashed by the normal security disposition.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` tracks end-of-battle security sub-timing as a separate phase-granular concern; token and play primitives exist, but no Puppet card-level regression proves this exact flow.
- **First test:** Reveal `P-165` from security in battle, finish the battle, assert it enters battle area without paying cost, then assert its On Play Familiar token effect resolves.
- **Implementation hint:** Reuse the existing security pending-card disposition machinery where possible, but ensure the action/mask path does not collapse optional choices or skip On Play.

### PUPPETS-G007: Puppet Aura Package Needs Card-Level Regression Coverage

- **Type:** `test-gap`
- **Status:** open
- **Blocks:** confidence for `EX9-033`, `EX11-023`, `ST19-14`, and related aura/keyword packages.
- **Effect text examples:** "All of your Tokens and [Puppet] trait Digimon gain <Alliance> and <Blocker>" and "When effects play one of your Tokens or Puppet Digimon... gains <Rush>."
- **Why it matters:** The engine has general Alliance, Blocker, Rush, keyword grants, and lowest-level predicates, but the exact Puppet combination is broad and action-mask-sensitive.
- **Evidence:** Combat tests cover Alliance and Overclock generically; production Puppet cards do not yet have end-to-end YAML behavioral tests.
- **First test:** Author `EX9-033` YAML, place Puppet and Token allies, assert Alliance and Blocker masks appear only for legal targets, then delete another Digimon and assert the once-per-turn lowest-level delete plus end-turn trash-play choices behave correctly.
- **Implementation hint:** Keep this as a card-level regression suite rather than a new engine primitive unless the test uncovers missing mask or aura scope behavior.

## Cross-Archetype Spec Tags

Use these tags when normalizing gaps across archetypes:

- `missing-production-yaml`
- `effect-refire`
- `effect-play-provenance`
- `scheduled-cleanup`
- `event-gated-delay`
- `on-ally-played`
- `effect-initiated-event-context`
- `security-end-of-battle`
- `aura-keyword-regression`

## Suggested Build Order

1. `BT22-042` Nyabootmon card test for Overclock plus re-fired When Digivolving.
2. `EX9-032` Karakurumon card test for delete-cost effect digivolve.
3. `EX9-033` Kaguyamon card test for Alliance/Blocker aura plus lowest-level delete.
4. `P-165` ShoeShoemon card test for security end-of-battle play plus Familiar token.
5. `P-229` event-gated Delay test for Mirai-played trigger.
