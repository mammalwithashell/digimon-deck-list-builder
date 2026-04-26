# DCGO ↔ Rust Keyword Parity

**Date:** 2026-04-24
**Scope:** Cross-engine parity tracker for printed keyword behaviors. Compares the DCGO C# source-of-truth implementation (`DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/`) against the Rust engine's `Keyword` enum consumption surface (`digimon-engine/src/**/*.rs`).

Sister document to [RUST_PYTHON_PARITY.md](RUST_PYTHON_PARITY.md) — that tracker catalogs semantic divergences in shared *subsystems* between Rust and Python; this tracker catalogs per-*keyword* behavioral fidelity against the C# source.

## Legend

- ✅ **Correct** — Rust behavior faithfully matches DCGO under the printed rules.
- 🟡 **Divergent** — Rust consumes the keyword but the semantics differ from DCGO in a way that affects gameplay.
- 🔴 **Parsed-but-unwired** — native keyword parse lands in `CardData::keywords`, no engine code acts on it.
- ❌ **Missing from enum** — DCGO has the keyword; Rust's `Keyword` enum does not.
- 🟣 **Deferred** — wire-up blocked on a known infrastructure gap (usually nested-selection-in-replacement).
- ⚪ **Parsed, no auto-install (intentional)** — the variant is parsed and available to hand-rolled scripts, but no `keyword_to_auto_effect` arm exists because real cards always pair the keyword with explicit effect text — auto-installing a generic effect would double-fire.

## Summary table

| Keyword | DCGO mechanism | Rust status | Notes |
|---|---|---|---|
| Rush | Passive `HasRush` flag — exempts summoning sickness | ✅ | `can_attack` + mask both gate on it |
| Blocker | Passive flag — permits blocking | ✅ | `try_enter_block` requires it (or Collision) |
| Jamming | `CanNotBeDestroyedByBattleClass` — blocks **all** battle destruction | ✅ | Correct as-is per RULES_CONTEXT 16-8 (security-only). Previous parity-doc 🟡 flag was based on an incorrect reading of DCGO; corrected in Phase A. |
| Piercing | Forces security continuation after winning Digimon battle | ✅ | `combat.rs:502` post-battle hook |
| Reboot | Auto-unsuspend at start of controller's turn | ✅ | `game_phases.rs:70` ResetPhase hook |
| Blitz | End-of-turn player attack if opponent has ≥1 memory | ✅ | Mask + action path |
| Raid | Retarget to tied-for-highest-DP Digimon | ✅ | Both combat retarget and mask emission |
| Alliance | Select ally, suspend it, +ally DP / +1 S-Attack | ✅ | `try_enter_alliance` — trait-match filter not yet enforced (tracked in RUST_ENGINE_GAPS) |
| Vortex | End-of-turn attack bypassing summoning sickness and interrupts | ✅ | `can_attack(vortex=true)` + mask |
| Overclock | End-of-turn bonus attack by sacrificing a trait-filtered ally | ✅ | `game_phases.rs:274/347` + mask |
| Collision | Grants virtual Blocker to all opp Digimon during own attack | ✅ | `try_enter_block:1237` checks attacker-Collision |
| Barrier | Trash top deck card to cancel own deletion | ✅ | Auto-install `WhenWouldBeDeleted` in `cards/keyword_effects.rs:49` |
| Evade | Send self to deck bottom instead of trash on deletion | ✅ | Auto-install `WhenWouldBeDeleted` in `cards/keyword_effects.rs:77` |
| Decode | Return self to own hand instead of opp deck/hand | ✅ | Auto-install two `WhenWouldBeReturnedTo*` replacements in `cards/keyword_effects.rs:103` |
| Fragment(N) | Trash N sources from own stack to cancel deletion | ✅ | Auto-installed in Phase D 2026-04-25; **optional** replacement via `CountCappedZone::Material` source-pick + `ctx.cancel_leave`. (Initial Phase D implementation was mandatory per DCGO `Fragment.cs:38` `canNoSelect: () => false`; flipped to optional 2026-04-25 — see commit `e85cb823`.) See `keyword_effects.rs` and `tests/keyword_phase_d/fragment_n.rs`. |
| ArmorPurge | Trash current top Digimon, promote next source as new top, cancel deletion | ✅ | Auto-installed in Phase D 2026-04-25; **optional** replacement via `ctx.armor_purge_top` primitive + `ctx.cancel_leave`. (Optionality flipped 2026-04-25 — see commit `bbd976fa`.) See `keyword_effects.rs` and `tests/keyword_phase_d/armor_purge.rs`. |
| Partition | On leave-field (not battle/own-effect): pick + play 2 cards from own sources free+unsuspended | ✅ | Auto-installed in Phase D 2026-04-25; `OnDeletion` trigger with cause filter (`!Battle && !OwnEffect`), 2-pick source selection. See `keyword_effects.rs` and `tests/keyword_phase_d/partition.rs`. |
| Progress | `CanNotAffectedClass` on attacker during attack, filtered `IsOpponentEffect` + top-card-only | ✅ | Gated at all `ctx.*` mutation entry points; `add_modifier` short-circuits unconditionally on `progress_excludes(target, Some(self.player))` (Phase B + Phase E prep — see §Progress below). |
| SecurityAttackPlus(N) | Adds N security attacks to the Digimon | ✅ | Consumed at `resolve_player_security_loop` via `Game::security_attack_keyword_bonus` alongside `ModifierType::SecurityAttackChange` (Phase A §A3). No `keyword_to_auto_effect` entry needed — keyword is queried directly at the resolution site, not emitted as a declarative effect. |
| SecurityAttackMinus(N) | Same shape, negative delta | ✅ | Consumed at `resolve_player_security_loop` via `Game::security_attack_keyword_bonus` alongside `ModifierType::SecurityAttackChange` (Phase A §A3). No `keyword_to_auto_effect` entry needed — keyword is queried directly at the resolution site, not emitted as a declarative effect. |
| DeDigivolve(N) | Active skill — remove N top digivolution cards from target | ⚪ | Phase E cards.json survey 2026-04-25: 160 printings, all paired with explicit effect text + timing tag. Zero bare printings. Auto-install would double-fire alongside hand-rolled `[Main]` actions on every printing — intentionally NOT auto-installed. Script-level helper `ctx.de_digivolve(_, _, amount=Some(N))` is the consumer for hand-rolled scripts. |
| DrawX(N) | "Draw N" on Option cards | ⚪ | Phase E cards.json survey 2026-04-25: 452 printings across Digimon/DigiEgg/Tamer/Option, all combined with explicit timing tags. Zero bare printings. Auto-install would double-fire — intentionally NOT auto-installed. Script-level `ctx.draw(player, n)` is the consumer for hand-rolled scripts. |
| Save | Place self under own Tamer as bottom source, cancel deletion | ✅ | Auto-installed in Phase D 2026-04-25; optional `WhenWouldBeDeleted` replacement, Tamer pick + `ctx.place_card_under_permanent_bottom` + `ctx.cancel_leave`. See `keyword_effects.rs` and `tests/keyword_phase_d/save.rs`. |
| Fortitude | Play self from trash free + unsuspended when self-stack deleted, if sources available | ✅ | Auto-installed in Phase D 2026-04-25; `OnDeletion` trigger with source-count gate, `ctx.play_from_trash_free_unsuspended`. See `keyword_effects.rs` and `tests/keyword_phase_d/fortitude.rs`. |
| Decoy | Redirect deletion of any same-controller ally to self | ✅ | Auto-installed in Phase D 2026-04-25; `WhenWouldBeDeleted` subscription on ally deletions, `ctx.substitute_replacement`. See `keyword_effects.rs` and `tests/keyword_phase_d/decoy.rs`. |
| Blast Digivolve | `Blast Digivolve` counter-window play | 🔴 | Parsed as `Keyword::BlastDigivolve` (renamed Phase A §A2). Auto-install of `Effect::blast_digivolve` from the keyword is deferred (Phase D scope did not include BlastDigivolve). |
| MaterialSave(count) | Move up-to-N own stack sources under another permanent — `[Main]` active skill | ✅ | Auto-installed in Phase D 2026-04-25; `MainOnField` effect with gate (≥1 source + ≥1 own Tamer), own-Tamer pick then source-pick via `CountCappedZone::Material`, tuck via `ctx.place_card_under_permanent_bottom`. See `keyword_effects.rs` and `tests/keyword_phase_d/material_save.rs`. |
| MindLink | Attach Tamer card to a Digimon with empty Tamer slot | ✅ | Auto-installed in Phase F 2026-04-25; `MainOnField` Tamer skill picks an own non-Tamer non-token Digimon with no non-face-down Tamer source and tucks self underneath via `attach_tamer_to_digimon`. New `face_down: bool` field on `CardSource` honors DCGO's `IsFlipped` filter. See `keyword_effects.rs::Keyword::MindLink` arm and `tests/keyword_phase_f/mind_link.rs`. RULES_CONTEXT 16-27. |
| Iceclad | Compare digivolution-card count instead of DP in battle (except vs Security Digimon); higher count wins, tie = both delete | ✅ | Combat-resolution branch in `combat::resolve_battle` swaps DP compare for `card_sources.len()` compare when either combatant has Iceclad (security battles unaffected — they route through `resolve_player_security_loop`). See `combat.rs:resolve_battle` and `tests/keyword_phase_f/iceclad.rs`. RULES_CONTEXT 16-34. |
| Execute | Active skill — attack unsuspended opp, self-delete on end-of-attack | ✅ | Auto-installed in Phase F 2026-04-25; `EndOfYourTurn` triggered effect grants `MayAttack` + `CanAttackUnsuspended` for the end-of-turn-attack window and queues `EndOfAttack` self-deletion. The printed 'may' surfaces at the EOT-action phase PASS exit (substrate adaptation: drainer auto-fires single optional triggers; observable end states match DCGO). See `keyword_effects.rs::Keyword::Execute` arm and `tests/keyword_phase_f/execute.rs`. RULES_CONTEXT 16-37. |
| Retaliation | When deleted by battle, destroy the winner | ✅ | Auto-installed in Phase E 2026-04-25; `OnDeletion` trigger gated on `deletion_cause() == Battle`, deletes opposing combatant via `ctx.battle_opponent_of` (new accessor) with explicit `OwnEffect` cascade cause. See `keyword_effects.rs` and `tests/keyword_phase_e/retaliation.rs`. RULES_CONTEXT 16-12. |
| Scapegoat | Delete another own Digimon to cancel own deletion | ✅ | Auto-installed in Phase E 2026-04-25; `WhenWouldBeDeleted` substitute replacement gated on `cause != OwnEffect`, optional outer dialog → parked own-permanent pick → sync substitute. See `keyword_effects.rs` and `tests/keyword_phase_e/scapegoat.rs`. RULES_CONTEXT 16-31. Phase F 2026-04-25: outer-dialog UX gap closed via the new `replacement_condition` builder. Substrate change: `replacement::collect_candidates` now threads `cause` into a per-effect `replacement_condition` closure; Scapegoat uses it to gate on `cause != OwnEffect` AND ≥1 substitute candidate (DCGO `CanActivateScapegoat` parity). See `tests/keyword_phase_f/scapegoat_cause_filter.rs`. |
| Training | Active skill — suspend self + place top deck card as own bottom source face-down | ✅ | Auto-installed in Phase F 2026-04-25; `MainOnField` skill (battle area + breeding area) suspends self (cost) and places deck top at bottom of self stack, face-down via `training_place_deck_top_under_self_face_down`. New breeding-area `[Main]` dispatch substrate (`Game::activate_breeding_main_training` + `mask.rs` Training-only emitter at slot 14). See `keyword_effects.rs::Keyword::Training` arm and `tests/keyword_phase_f/training.rs`. RULES_CONTEXT 16-40. |

## Detailed notes on the divergences

### Progress — wrong site entirely

Phase A landed the partial fix: the wrong `SecuritySkillDrain` gate was never re-introduced, and `Game::progress_excludes` now gates `select_opponent_permanent`. Phase B (2026-04-24) closed the mutation-site coverage: `ctx.delete_permanent`, `ctx.return_to_hand`, `ctx.return_to_deck`, `ctx.de_digivolve` (including the `amount=Some(N)` N-pop variant), `ctx.suspend`, and `ctx.add_dp_modifier` / `ctx.add_modifier` are all now hard-gated (the modifier path was subsequently broadened in Phase E prep — see § Progress — gate scope).

**Source-attribution model.** Gates apply at the `EffectContext` layer where the source controller is statically known via `self.player`; Game-level fire-sites stay agnostic so rule-driven mutations (own-sourced deletes, security-check redirects, cost trash) flow through unchanged. Observers consume cause via the new `ctx.deletion_cause()` / `ctx.was_deleted_by_effect()` / `ctx.was_deleted_by_opponent()` accessors (Phase B §B5).

See the spec at [superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md](superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md) §5 Phase A/B for the full plan.

**`place_as_bottom_source` — reviewed, no gate.** Phase B's final code review flagged `EffectContext::place_as_bottom_source` as a candidate site. Verdict after cross-checking DCGO: not gated. DCGO's primitive [`Permanent.AddDigivolutionCardsBottom`](../DCGO/Assets/Scripts/Script/Permanent.cs#L1104) has no internal `CanNotBeAffected` check, and DCGO scripts that intentionally place a source under an opponent's Digimon (e.g. [EX10-059](../DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_059.cs#L218)) do not filter the target on `CanNotBeAffected` either. Adding a card under a stack is not "affecting" the target in DCGO's semantics — the TopCard's status is unchanged. Gating in Rust would over-restrict relative to DCGO. The decision is documented inline at [`effect_context/mod.rs::place_as_bottom_source`](../digimon-engine/src/effect_context/mod.rs).

### Progress — gate scope (Phase B + Phase E prep)

The gate is consumed at the script-API mutation entry points in
`digimon-engine/src/effect_context/mod.rs`. As of the Phase E preparatory
broadening, the suppressed mutation set is:

- `ctx.delete_permanent`
- `ctx.return_to_hand`
- `ctx.return_to_deck`
- `ctx.de_digivolve`
- `ctx.suspend`
- `ctx.add_modifier` / `ctx.add_dp_modifier` — every `ModifierType` variant,
  every value (positive or negative), DCGO-faithful and hostility-blind.
  Mirrors DCGO's `targetPermanent.TopCard.CanNotBeAffected(activateClass)`
  check that every `GiveEffectToPermanent/*.cs` helper performs.

Out-of-scope at the gate (deliberate):

- **Player-scoped flood gates** — install on `Player`, not `Permanent`,
  and don't reach `add_modifier`. (Examples: `DrawBlock`, `MemoryBlock`,
  `CannotPlayDigimonByEffect`.)
- **Attack-target redirection** — goes through `ctx.redirect_attack`,
  not a `ModifierType`. Tracked separately for Phase E proper if a
  redirect-on-Progress-carrier interaction surfaces.
- **Rule-driven mutations** — battle damage, cost-paid trash, EOT
  expiry. `progress_excludes` returns `false` when source is `None`.
- **`ctx.place_as_bottom_source`** — see the preceding paragraph for the
  cross-checked rationale.

The gate's predicate is exactly DCGO's: target is the current Progress
attacker AND source is the opposite player. No hostility classification,
no sign check.

### Blast keyword variant is dead code

Resolved Phase A §A2 — renamed to `Keyword::BlastDigivolve`; auto-install remains deferred (not included in Phase D).

### Save / MaterialSave name collision

Resolved Phase A §A5 — `Keyword::MaterialSave(u8)` split out; parser + `dsl_cards/modifier_map.rs` aliasing removed.

### Fortitude enum mis-mapping

Rust's `ModifierType` enum has `GrantBarrier` in the slot where `GrantFortitude` would naturally sit. The Rust-side agent's inventory flags that `Keyword::Fortitude`'s granted-modifier lookup returns `GrantBarrier`. That is either:

- A mis-mapping that conflates two distinct keywords, or
- A deliberate repurposing that should be documented.

Since no Fortitude card grants the keyword via modifier yet, the simplest fix is to drop the granted form for Fortitude entirely; when a real card needs it, add a proper `GrantFortitude` variant.

### SecurityAttackPlus / Minus — resolved Phase A §A3

Resolved 2026-04-24. `Game::security_attack_keyword_bonus` sums printed `Keyword::SecurityAttackPlus(N)` / `Keyword::SecurityAttackMinus(N)` at `resolve_player_security_loop` alongside the existing `ModifierType::SecurityAttackChange` modifier sum. This follows the "query keyword directly at consumption site" pattern used for Blocker / Rush / Jamming — no `keyword_to_auto_effect` entry is required, and `Effect.security_attack_change` remains the granted-modifier path for cards that want to add or remove security attacks dynamically.

### Parametric auto-install — resolved Phase E §E3/E4 (resolved-as-no-op)

A 2026-04-25 cards.json survey found that `DeDigivolve(N)` (160 printings) and `DrawX(N)` (452 printings) are NEVER printed as bare keywords on real cards — every printing pairs the keyword with explicit effect text plus an explicit timing tag (`[Main]`, `[On Play]`, `[When Attacking]`, etc.). Auto-installing a generic `MainOnField` skill or `OnPlay` effect would double-fire alongside every card's hand-rolled action.

The variants are parsed (so deck-builder validation, RL action masking, and hand-rolled scripts can read them) but no `keyword_to_auto_effect` arm is added for either. Hand-rolled scripts call `ctx.de_digivolve(_, _, amount=Some(N))` and `ctx.draw(player, n)` directly. Marked ⚪ in the summary table.

This closes the "parametric auto-install gap" — the original framing assumed bare printings existed; they don't.

### Scapegoat outer-dialog UX — Phase F 2026-04-25 — resolved

Phase E §E2 left a known divergence from DCGO: with `.optional()` set at builder time, the outer "may" accept dialog parked even on `OwnEffect` cause and even with zero candidates for the inner pick. End-state matched DCGO via PASS, but the spurious dialog itself was a UX leak.

**Resolution (Phase F, commit `10ca36b8`).** New `replacement_condition: Option<EffectReplacementConditionFn>` field on `Effect` with a `.replacement_condition(...)` builder; `replacement::collect_candidates` now threads `cause` into the closure so candidate-side gating can read it. Scapegoat is re-mounted to gate on `cause != OwnEffect` AND ≥1 substitute candidate, matching DCGO's `CanActivateScapegoat` pre-filter. Behavioral coverage in `tests/keyword_phase_f/scapegoat_cause_filter.rs`.

## Missing-keyword backfill priorities

Ordered by archetype relevance to the alpha scope (Royal Knights, Jesmon GX, Rocks, Medusamon, Dark Masters):

| Priority | Keyword | Why |
|---|---|---|
| ~~1~~ | ~~Retaliation~~ | ~~Dark Masters core: BT15-077 LadyDevimon, BT15-079 Piedmon~~ ✅ resolved Phase E 2026-04-25 |
| ~~2~~ | ~~MaterialSave(count)~~ | ~~Several Medusamon and Dark Masters entries use it~~ ✅ resolved Phase D |
| ~~2~~ | ~~Scapegoat~~ | ~~Dark Masters (LM-043 Darkdramon)~~ ✅ resolved Phase E 2026-04-25 |
| ~~3~~ | ~~Training~~ | ~~Tied to TestCards.Training active-skill; needed for Rocks pre-evo slots~~ ✅ resolved Phase F 2026-04-25 |
| ~~4~~ | ~~Execute~~ | ~~Appears only on a handful of non-archetype cards; defer~~ ✅ resolved Phase F 2026-04-25 |
| ~~5~~ | ~~Iceclad, MindLink~~ | ~~Not in any alpha-target archetype; defer~~ ✅ resolved Phase F 2026-04-25 |

## Gap ranking (consolidated for scheduling)

Ranked by alpha-archetype blast radius:

1. ~~**Progress semantics fix**~~ — ✅ resolved Phase A + B. Selection-filter exclusion + opponent-mutation-site gating both landed.
2. ~~**Fragment(N) wire-up**~~ — ✅ resolved Phase D 2026-04-25. Fragment(N), ArmorPurge, Partition all landed. Cascaded fix is complete.
3. **SecurityAttackPlus/Minus auto-install** — printed on many cards across all archetypes; trivial to add.
4. **Jamming scope widening** — affects any attacking Digimon losing a regular Digimon battle; tens of cards.
5. ~~**Save distinct from MaterialSave**~~ — ✅ resolved Phase D 2026-04-25. Save, Decoy, Fortitude, MaterialSave(N) all auto-installed.
6. ~~**Retaliation enum variant + replacement wire-up**~~ — ✅ resolved Phase E 2026-04-25. `OnDeletion` trigger auto-installed; Dark Masters archetype blocker cleared.
7. ~~**Fortitude / DeDigivolve(N) parsed-form auto-install / Decoy**~~ — ✅ Fortitude + Decoy resolved Phase D 2026-04-25. DeDigivolve(N) auto-install resolved-as-no-op Phase E 2026-04-25 (zero bare printings; see ⚪ note in summary table).
8. ~~**Execute / Iceclad / MindLink / Training**~~ — ✅ resolved Phase F 2026-04-25. All four enum-missing keywords wired (Execute + MindLink + Training auto-installs; Iceclad consumed at `combat::resolve_battle`). Plus the Phase E Scapegoat outer-dialog UX divergence closed via the new `replacement_condition` builder. ✅ resolved Phase F 2026-04-25.

## Source citations

- DCGO keyword implementations: `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/*.cs` (behaviors) and `DCGO/Assets/Scripts/Script/CardEffectFactory/KeyWordEffects/*.cs` (factory wrappers). 28 files total.
- Rust keyword enum: [`digimon-engine/src/enums.rs`](../digimon-engine/src/enums.rs) (`Keyword` ~line 265, `ModifierType::Grant*` ~line 355).
- Native parsing: [`digimon-engine/src/card_data.rs::parse_printed_keywords`](../digimon-engine/src/card_data.rs).
- Unified keyword query: [`digimon-engine/src/game.rs::has_keyword`](../digimon-engine/src/game.rs).
- Auto-installed replacements: [`digimon-engine/src/cards/keyword_effects.rs`](../digimon-engine/src/cards/keyword_effects.rs).
- Major consumption sites: [`digimon-engine/src/combat.rs`](../digimon-engine/src/combat.rs), [`digimon-engine/src/action/mask.rs`](../digimon-engine/src/action/mask.rs), [`digimon-engine/src/game_phases.rs`](../digimon-engine/src/game_phases.rs).
