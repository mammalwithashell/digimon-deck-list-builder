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
| Jamming | `CanNotBeDestroyedByBattleClass` — blocks **all** battle destruction | 🟡 | Rust only applies to losing security battle, not Digimon-vs-Digimon combat loss |
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
| Progress | `CanNotAffectedClass` on attacker during attack, filtered `IsOpponentEffect` + top-card-only | 🟡 (**wrong**) | Rust currently gates SecuritySkill drain instead — see §Progress below |
| SecurityAttackPlus(N) | Adds N security attacks to the Digimon | 🔴 | Parsed; consumed **nowhere**. `resolve_player_security_loop` uses `SecurityAttackChange` modifier only |
| SecurityAttackMinus(N) | Same shape, negative delta | 🔴 | Same gap |
| DeDigivolve(N) | Active skill — remove N top digivolution cards from target | 🔴 | Parsed, unconsumed. Script-level helper `ctx.de_digivolve_n` landed in Phase 10, but native printed form isn't wired to auto-emit the effect |
| DrawX(N) | "Draw N" on Option cards | 🔴 | Parsed, unconsumed |
| Armor | DCGO has no standalone "Armor" keyword — only `Armor Purge` | 🔴 | Rust enum variant + printed parse exist with no DCGO counterpart. Candidate for removal or rename |
| Save | Place deleted card under own Digimon/Tamer as bottom source | 🔴 | Parsed, unconsumed. Rust also aliases "MaterialSave" to this variant — see §Save / MaterialSave below |
| Fortitude | Play self from trash free + unsuspended when ally deleted, if sources available | 🔴 | Parsed, unconsumed. Note: Rust's enum has `GrantBarrier` where `GrantFortitude` should live — see §Fortitude below |
| Decoy | Redirect deletion to another own permanent (color-filtered) | 🔴 | Parsed; `GrantDecoy` exists; no consumer |
| Material | Not a DCGO keyword under this name — likely collision with C# `MaterialSave` | 🔴 | Rust variant exists; no DCGO counterpart. Remove or merge |
| Blast | `Blast Digivolve` counter-window play | 🟡 | Parsed as `Keyword::Blast`. Rust handles the mechanic via the distinct `Effect::blast_digivolve` flag + `try_enter_counter`, NOT via this keyword variant — so the variant is dead code |
| MaterialSave(count) | Move up-to-N own stack sources under another permanent — active skill | ❌ | Not in Rust enum; not scripted |
| MindLink | Attach Tamer card to a Digimon with empty Tamer slot | ❌ | Not in Rust enum |
| Iceclad | Passive immunity to suspension | ❌ | Not in Rust enum (`ModifierType::CannotSuspend` exists, but no keyword wrapper) |
| Execute | Active skill — attack unsuspended opp, self-delete on end-of-attack | ❌ | Not in Rust enum |
| Retaliation | When deleted by battle, destroy the winner | ❌ | Not in Rust enum |
| Scapegoat | Delete another own Digimon to cancel own deletion | ❌ | Not in Rust enum |
| Training | Active skill — suspend self + place top deck card as own bottom source face-down | ❌ | Not in Rust enum; Python has handling, Rust does not |

## Detailed notes on the divergences

### Progress — wrong site entirely

DCGO's [`Progress.cs`](../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Progress.cs) `ProgressProcess` installs a `CanNotAffectedClass` onto the attacking permanent's `UntilEndAttackEffects`. The class carries:

- **`SkillCondition`** — `IsOpponentEffect(cardEffect, cardSource)` — only blocks opponent-sourced effects.
- **`CardCondition`** — target card must be the attacker's current top card.
- **Lifetime** — `UntilEndAttack` — expires when the attack resolves.

It does **not** skip the defender's `SecuritySkill` phase. Digital Gate Open's `[Security]` effect (play a ≤3-cost Digimon from hand/trash; add self to hand) has no attacker-targeting clause, so Progress has nothing to block and both halves of the security effect should resolve. An effect like Mega Death's "delete an opp Digimon with cost ≤5" **does** target the attacker, so the Progress attacker is excluded from the selection pool (but the prompt still installs and the defender may pick a different target).

The 2026-04-24 `§2.5c` commit wires Progress at the wrong site — the `SecuritySkillDrain` phase gate in `combat.rs` — which causes Digital Gate Open to incorrectly no-op when the attacker has Progress. This commit will be reverted; the correct consumption belongs at opponent-effect mutation sites, not at the security phase boundary.

**Fix outline:**
- Keep `Keyword::Progress` and `ModifierType::ImmunityToOpponentEffects` in the enum.
- Revert the `SecuritySkillDrain` gate in `combat.rs`.
- Wire the check at selection filters (`select_opponent_permanent`, `select_any_permanent`, multi-select filters) to exclude a `Progress + is_attacking` target whose controller is the opposite side of the selector.
- Wire at `delete_permanent_with_effects` and other mutation sites when the mutation is sourced by an opponent effect (requires a source-attribution parameter on these entry points — tracked in RUST_ENGINE_GAPS under "WhenWouldBeDeleted" replacement framework extensions).
- Suppress negative-DP `modifiers.add` calls from opponent-sourced effects targeting the Progress attacker.

This is semantically equivalent to widening `CannotBeAffected` / `CannotBeSelectedByEffect` semantics to fire implicitly for a Progress-and-attacking permanent against opponent-sourced mutations.

Python has the same SecuritySkill-skip bug at [`player.py:614-617`](../digimon_gym/engine/core/player.py#L614). Fixing Rust here diverges from Python behavior but matches DCGO and the printed rules. Note this deliberate divergence in `docs/RUST_PYTHON_PARITY.md` when the fix lands.

### Jamming — scope too narrow

DCGO installs [`CanNotBeDestroyedByBattleClass`](../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Jamming.cs) via `AddEffectToPermanent`. Any battle-deletion query consults it, covering both:

1. Digimon-vs-Digimon combat where attacker's DP < defender's DP (attacker normally dies).
2. Digimon-vs-security battle where attacker's DP < security Digimon's DP.

Rust only checks Jamming at [`combat.rs:1816`](../digimon-engine/src/combat.rs) — the security DP battle branch. A Jamming attacker that loses a regular Digimon fight at attack-declare time still dies in Rust. That matches Python's historical behavior but diverges from printed rules (e.g. BT14-099 UlforceV-dramon X).

**Fix:** add `has_keyword(attacker, Jamming)` check at the end of the Digimon-vs-Digimon DP compare in `resolve_pending_battle`, before the `delete_permanent_with_effects(attacker)` call.

### Blast keyword variant is dead code

`Keyword::Blast` is parsed from `<Blast Digivolve>` text, but Rust's actual Blast Digivolve flow runs through the distinct `Effect::blast_digivolve` boolean flag set by card scripts on their Counter-window effect, consumed by `try_enter_counter`. The `Keyword::Blast` variant is therefore unused. Options:

- Drop the variant + parsing entry.
- Or auto-install a declarative `blast_digivolve=true` effect when the keyword is parsed, matching DCGO's `BlastDigivolution.cs` factory pattern.

### Save / MaterialSave name collision

DCGO has **two distinct keywords**:

- **Save** — when this card is deleted and lands in trash, place it under another permanent as bottom source. Replacement-effect-on-deletion timing.
- **MaterialSave(count)** — active skill on a Digimon, move up-to-N own digivolution-stack sources under another permanent. Main-phase active-skill timing.

Rust's `parse_printed_keywords` routes both names to `Keyword::Save`, which has different trigger surfaces and should not share a variant.

**Fix:** introduce `Keyword::MaterialSave(u8)`, stop aliasing "MaterialSave" to `Save` in the parser, and route each to its own consumer.

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

1. **Progress semantics fix** — affects Royal Knights (Imperialdramon:PM), DNA Omnimon core, Rocks. Highest blast radius, and a wrong implementation currently shipped.
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
