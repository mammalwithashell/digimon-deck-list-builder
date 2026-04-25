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
| Fragment(N) | Trash N sources from own stack to cancel deletion | 🟣 | Enum variant + printed parse work; consumer returns empty Vec — blocked on nested-select-in-replacement |
| ArmorPurge | Trash 1 digivolution source as active skill | 🟣 | Same deferral as Fragment |
| Partition | Split stack by 2 color/trait groups, play one from each on leave | 🟣 | Same deferral |
| Progress | `CanNotAffectedClass` on attacker during attack, filtered `IsOpponentEffect` + top-card-only | 🟢 | Gated at all `ctx.*` mutation entry points; `add_modifier` short-circuits unconditionally on `progress_excludes(target, Some(self.player))` — see §Progress below |
| SecurityAttackPlus(N) | Adds N security attacks to the Digimon | ✅ | Consumed at `resolve_player_security_loop` via `Game::security_attack_keyword_bonus` alongside `ModifierType::SecurityAttackChange` (Phase A §A3). |
| SecurityAttackMinus(N) | Same shape, negative delta | ✅ | Consumed at `resolve_player_security_loop` via `Game::security_attack_keyword_bonus` alongside `ModifierType::SecurityAttackChange` (Phase A §A3). |
| DeDigivolve(N) | Active skill — remove N top digivolution cards from target | 🔴 | Parsed, unconsumed. Script-level helper `ctx.de_digivolve(_, _, amount=Some(N))` landed in Phase 10, but native printed form isn't wired to auto-emit the effect |
| DrawX(N) | "Draw N" on Option cards | 🔴 | Parsed, unconsumed |
| Save | Place deleted card under own Digimon/Tamer as bottom source | 🔴 | Parsed, unconsumed. |
| Fortitude | Play self from trash free + unsuspended when ally deleted, if sources available | 🔴 | Parsed, unconsumed. Note: Rust's enum has `GrantBarrier` where `GrantFortitude` should live — see §Fortitude below |
| Decoy | Redirect deletion to another own permanent (color-filtered) | 🔴 | Parsed; `GrantDecoy` exists; no consumer |
| Blast Digivolve | `Blast Digivolve` counter-window play | 🔴 | Parsed as `Keyword::BlastDigivolve` (renamed Phase A §A2). Auto-install of `Effect::blast_digivolve` from the keyword is Phase D work. |
| MaterialSave(count) | Move up-to-N own stack sources under another permanent — active skill | 🔴 | Enum variant + parametric `<Material Save N>` parser landed Phase A §A5; auto-install in Phase D. |
| MindLink | Attach Tamer card to a Digimon with empty Tamer slot | ❌ | Not in Rust enum |
| Iceclad | Compare digivolution-card count instead of DP in battle (except vs Security Digimon); higher count wins, tie = both delete | ❌ | Not in Rust enum. Previous description ('immunity to suspension') was incorrect; actual mechanic is digi-card-count battle compare per RULES_CONTEXT 16-34. Wiring: Phase F2. |
| Execute | Active skill — attack unsuspended opp, self-delete on end-of-attack | ❌ | Not in Rust enum |
| Retaliation | When deleted by battle, destroy the winner | ❌ | Not in Rust enum |
| Scapegoat | Delete another own Digimon to cancel own deletion | ❌ | Not in Rust enum |
| Training | Active skill — suspend self + place top deck card as own bottom source face-down | ❌ | Not in Rust enum; Python has handling, Rust does not |

## Detailed notes on the divergences

### Progress — wrong site entirely

Phase A landed the partial fix: the wrong `SecuritySkillDrain` gate was never re-introduced, and `Game::progress_excludes` now gates `select_opponent_permanent`. Phase B (2026-04-24) closed the mutation-site coverage: `ctx.delete_permanent`, `ctx.return_to_hand`, `ctx.return_to_deck`, `ctx.de_digivolve` (including the `amount=Some(N)` N-pop variant), `ctx.suspend`, and the negative-DP path through `ctx.add_dp_modifier` / `ctx.add_modifier` are all now hard-gated.

**Source-attribution model.** Gates apply at the `EffectContext` layer where the source controller is statically known via `self.player`; Game-level fire-sites stay agnostic so rule-driven mutations (own-sourced deletes, security-check redirects, cost trash) flow through unchanged. Observers consume cause via the new `ctx.deletion_cause()` / `ctx.was_deleted_by_effect()` / `ctx.was_deleted_by_opponent()` accessors (Phase B §B5).

See the spec at [superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md](superpowers/specs/2026-04-24-dcgo-keyword-parity-design.md) §5 Phase A/B for the full plan.

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

The gate's predicate is exactly DCGO's: target is the current Progress
attacker AND source is the opposite player. No hostility classification,
no sign check.

### Blast keyword variant is dead code

Resolved Phase A §A2 — renamed to `Keyword::BlastDigivolve`; auto-install deferred to Phase D.

### Save / MaterialSave name collision

Resolved Phase A §A5 — `Keyword::MaterialSave(u8)` split out; parser + `dsl_cards/modifier_map.rs` aliasing removed.

### Fortitude enum mis-mapping

Rust's `ModifierType` enum has `GrantBarrier` in the slot where `GrantFortitude` would naturally sit. The Rust-side agent's inventory flags that `Keyword::Fortitude`'s granted-modifier lookup returns `GrantBarrier`. That is either:

- A mis-mapping that conflates two distinct keywords, or
- A deliberate repurposing that should be documented.

Since no Fortitude card grants the keyword via modifier yet, the simplest fix is to drop the granted form for Fortitude entirely; when a real card needs it, add a proper `GrantFortitude` variant.

### SecurityAttackPlus / Minus parametric not wired

`Effect.security_attack_change` is a real field consumed by `resolve_player_security_loop` via the `SecurityAttackChange` modifier sum, but the native keyword parse of `<Security A. +2>` does not auto-emit a matching effect. A card with printed `<Security A. +2>` parses its keyword but doesn't get +2 security attacks unless a `CardEffect` script also emits the modifier by hand.

**Fix:** extend `cards/keyword_effects.rs::keyword_to_auto_effect` to return a declarative effect emitting `SecurityAttackChange` with value `N` for `Keyword::SecurityAttackPlus(N)` / `Keyword::SecurityAttackMinus(-N)`.

### Parametric auto-install gap (generalized)

The same "parsed but no auto-install" pattern applies to:

- `DeDigivolve(N)` — no auto-emit of a main-phase active skill.
- `DrawX(N)` — no auto-emit of a `[Main]` draw effect.
- `Security A. ±N` — covered above.

Fix shape is uniform: extend `keyword_to_auto_effect` to emit a matching declarative or active-skill `Effect` when the keyword is parametric and native-printed.

## Missing-keyword backfill priorities

Ordered by archetype relevance to the alpha scope (Royal Knights, Jesmon GX, Rocks, Medusamon, Dark Masters):

| Priority | Keyword | Why |
|---|---|---|
| 1 | Retaliation | Dark Masters core: BT15-077 LadyDevimon, BT15-079 Piedmon |
| 2 | MaterialSave(count) | Several Medusamon and Dark Masters entries use it |
| 3 | Scapegoat | Dark Masters (LM-043 Darkdramon) |
| 4 | Training | Tied to TestCards.Training active-skill; needed for Rocks pre-evo slots |
| 5 | Execute | Appears only on a handful of non-archetype cards; defer |
| 6 | Iceclad, MindLink | Not in any alpha-target archetype; defer |

## Gap ranking (consolidated for scheduling)

Ranked by alpha-archetype blast radius:

1. ~~**Progress semantics fix**~~ — ✅ resolved Phase A + B. Selection-filter exclusion + opponent-mutation-site gating both landed.
2. **Fragment(N) wire-up** — Rocks archetype is built on Fragment. Blocked by nested-selection-in-replacement. Fix cascades to ArmorPurge + Partition.
3. **SecurityAttackPlus/Minus auto-install** — printed on many cards across all archetypes; trivial to add.
4. **Jamming scope widening** — affects any attacking Digimon losing a regular Digimon battle; tens of cards.
5. **Save distinct from MaterialSave** — Save is on staples in multiple archetypes (P-186 Gallantmon etc.), needs its own variant + auto-install.
6. **Retaliation enum variant + replacement wire-up** — Dark Masters archetype blocker.
7. **Fortitude / DeDigivolve(N) parsed-form auto-install / Decoy** — known card lists.
8. **Execute / Iceclad / MindLink / Training / MaterialSave** — not in alpha archetypes; defer past alpha.

## Source citations

- DCGO keyword implementations: `DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/*.cs` (behaviors) and `DCGO/Assets/Scripts/Script/CardEffectFactory/KeyWordEffects/*.cs` (factory wrappers). 28 files total.
- Rust keyword enum: [`digimon-engine/src/enums.rs`](../digimon-engine/src/enums.rs) (`Keyword` ~line 265, `ModifierType::Grant*` ~line 355).
- Native parsing: [`digimon-engine/src/card_data.rs::parse_printed_keywords`](../digimon-engine/src/card_data.rs).
- Unified keyword query: [`digimon-engine/src/game.rs::has_keyword`](../digimon-engine/src/game.rs).
- Auto-installed replacements: [`digimon-engine/src/cards/keyword_effects.rs`](../digimon-engine/src/cards/keyword_effects.rs).
- Major consumption sites: [`digimon-engine/src/combat.rs`](../digimon-engine/src/combat.rs), [`digimon-engine/src/action/mask.rs`](../digimon-engine/src/action/mask.rs), [`digimon-engine/src/game_phases.rs`](../digimon-engine/src/game_phases.rs).
