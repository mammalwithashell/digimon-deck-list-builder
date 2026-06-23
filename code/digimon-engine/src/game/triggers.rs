//! Event drain + declarative/granted triggers (Tier 1).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

/// `true` when `DIGIMON_FORCE_FULL_DECLARATIVE_REBUILD` is set — forces
/// `tick_declarative_effects` to always run the full clear-and-rebuild (the
/// oracle baseline / a safety fallback that bypasses the fingerprint skip).
/// Read once and cached so the per-tick cost is a single load.
fn force_full_declarative_rebuild() -> bool {
    static FORCE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FORCE.get_or_init(|| std::env::var("DIGIMON_FORCE_FULL_DECLARATIVE_REBUILD").is_ok())
}

/// Fold a `CardSource`'s declarative-relevant identity into the fingerprint:
/// which card it is (`data_index`), token-ness, face-down-ness, and any
/// effect-granted name aliases (read by name-matching declarative conditions).
fn hash_card_source_decl(
    cs: &crate::card_source::CardSource,
    h: &mut std::collections::hash_map::DefaultHasher,
) {
    use std::hash::Hash;
    cs.data_index.hash(h);
    cs.is_token.hash(h);
    cs.face_down.hash(h);
    cs.also_treated_as.hash(h);
}

/// Fold a `Permanent`'s declarative-relevant dynamic state into the
/// fingerprint: its full digivolution stack and linked cards, suspend/attack
/// flags, turn-timing counters, the per-turn activation summary, and option
/// state — i.e. everything a continuous-effect source-scan or condition can read.
fn hash_permanent_decl(
    perm: &crate::permanent::Permanent,
    h: &mut std::collections::hash_map::DefaultHasher,
) {
    use std::hash::Hash;
    perm.card_sources.len().hash(h);
    for cs in &perm.card_sources {
        hash_card_source_decl(cs, h);
    }
    perm.linked_cards.len().hash(h);
    for cs in &perm.linked_cards {
        hash_card_source_decl(cs, h);
    }
    perm.is_suspended.hash(h);
    perm.is_attacking.hash(h);
    perm.turn_played.hash(h);
    perm.turn_digivolved.hash(h);
    perm.attacks_this_turn.hash(h);
    // `effect_activations` is a `HashMap` — fold an order-independent summary.
    perm.effect_activations.len().hash(h);
    let activations: u64 = perm
        .effect_activations
        .values()
        .map(|&v| v as u64)
        .fold(0u64, u64::wrapping_add);
    activations.hash(h);
    format!("{:?}", perm.option_state).hash(h);
}

impl Game {
    /// Allocate the next monotonic event sequence number.
    pub fn next_event_seq(&mut self) -> u64 {
        let s = self.event_seq;
        self.event_seq += 1;
        s
    }

    /// Drain accumulated events, returning them in emission order. The
    /// `HeadlessRunner::step` wrapper calls this after each action so the
    /// PyO3 layer can expose a per-step event list.
    pub fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// Borrow accumulated events without draining them. Debug tooling uses
    /// this to checkpoint and assert on incremental event emission.
    pub fn events(&self) -> &[crate::events::GameEvent] {
        &self.events
    }

    /// Re-materialize declarative (continuous) effects, but only when the
    /// declarative-relevant game state has changed since the last rebuild.
    ///
    /// `materialize_declaratives_full` clears and rebuilds the whole board's
    /// materialized declarative state from scratch — scanning every permanent,
    /// stack source, breeding, face-up security, linked card, and floating-mass
    /// descriptor, then re-running every continuous-effect condition/process
    /// closure. That is the engine's hottest per-step cost, and most actions
    /// don't change which declarative sources exist (or any state their
    /// conditions read). This wrapper fingerprints the declarative-relevant
    /// inputs and skips the rebuild when the fingerprint is unchanged, so the
    /// materialized modifier state is byte-identical to an always-rebuild while
    /// the redundant rebuilds are elided.
    ///
    /// Correctness is guarded two ways: the fingerprint
    /// ([`Game::declarative_fingerprint`]) reads *all* declarative-relevant
    /// state (so it is location-independent — it doesn't matter which of the
    /// ~14 call sites invoked the tick or who mutated state), and in debug
    /// builds a differential oracle re-runs a full rebuild every tick and
    /// asserts the materialized state matches. `DIGIMON_FORCE_FULL_DECLARATIVE_REBUILD`
    /// forces the always-rebuild path (oracle baseline / fallback).
    pub fn tick_declarative_effects(&mut self) {
        let fingerprint = self.declarative_fingerprint();
        let can_skip = !force_full_declarative_rebuild()
            && self.modifiers.declarative_memo() == Some(fingerprint);

        if can_skip {
            // Differential oracle (debug/test only): the SKIP is the only case
            // where stale materialized state could ship — when the fast path
            // rebuilds, its result *is* a full rebuild and needs no check. So
            // verify exactly the skips: snapshot the preserved state, force a
            // fresh full rebuild, and assert they match. A missed fingerprint
            // input surfaces here as a failing assert across the
            // behavioral/card/archetype suites rather than a silently-stale
            // modifier shipping to release. Gating the oracle on skips (not
            // every tick) also keeps its extra rebuild off the deep
            // effect-resolution rebuild ticks, so it doesn't inflate peak
            // stack depth.
            #[cfg(debug_assertions)]
            {
                let skipped = self.modifiers.materialized_snapshot();
                self.materialize_declaratives_full();
                let rebuilt = self.modifiers.materialized_snapshot();
                self.modifiers.set_declarative_memo(fingerprint);
                debug_assert!(
                    skipped == rebuilt,
                    "declarative materialization divergence (incremental skip produced \
                     stale state — a fingerprint input is missing):\n  skipped={skipped:#?}\n  rebuilt={rebuilt:#?}"
                );
            }
            return;
        }

        self.materialize_declaratives_full();
        // The full rebuild only adds/clears *materialized* declaratives; it does
        // not touch the board or the non-materialized inputs the fingerprint
        // reads, so `fingerprint` still describes current state.
        self.modifiers.set_declarative_memo(fingerprint);
    }

    /// Full clear-and-rebuild of the board's materialized declarative state.
    /// This is the always-correct reference path; [`Game::tick_declarative_effects`]
    /// skips it when the declarative-relevant state is unchanged.
    pub fn materialize_declaratives_full(&mut self) {
        self.modifiers.clear_materialized_declaratives();

        let mut sources = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.data_index,
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));

                let stack_size = perm.card_sources.len();
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    if source_index + 1 >= stack_size {
                        continue;
                    }
                    sources.push((
                        source.data_index,
                        source.handle(),
                        Some(handle),
                        player_id,
                        true,
                    ));
                }
            }

            if let Some(perm) = player.breeding_area.as_ref() {
                let handle = PermanentHandle {
                    player: player_id,
                    index: crate::action::space::BREEDING_TARGET as u8,
                };
                let top = perm.top_card();
                sources.push((
                    top.data_index,
                    top.handle(),
                    Some(handle),
                    player_id,
                    false,
                ));
            }

            // Track H §5 — security-zone-sourced auras. Face-up security
            // cards can carry `kind: aura, scope: security` declarative
            // clauses that grant DP/keyword/modifier to filter-matched
            // battle-area permanents while the source remains face-up in
            // the security stack. Source-permanent is `None` because
            // security entries have no battle-area handle; the install
            // closures still target battle-area handles for the matches.
            // Cleanup is automatic — each tick clears materialized
            // declaratives, then re-installs from active sources, so a
            // card leaving security simply stops re-installing on the
            // next tick. Mirrors DCGO `BT21_095.cs:CanUseCondition` →
            // `IsExistInSecurity(card, false)`.
            for card in &player.security {
                if !player.face_up_security.contains(&card.card_index) {
                    continue;
                }
                sources.push((
                    card.data_index,
                    card.handle(),
                    None,
                    player_id,
                    false,
                ));
            }
        }

        for (data_index, source_card, source_permanent, controller, inherited_source) in sources {
            let Some(effects) = self.effects_for_card(&self.card_data[data_index].card_id, source_card)
            else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.declarative || effect.inherited != inherited_source {
                    continue;
                }
                if !effect.materializes_declarative_state || effect.process.is_none() {
                    continue;
                }
                // De-dup: a `grant_keyword` whose keyword is already a PRINTED
                // (metadata) keyword on this source is redundant with `card_data`,
                // which `security_attack_keyword_bonus` (and `has_keyword`) already
                // count from the printed-text parse — materializing it too would
                // double-count. This applies to BOTH scopes, keyed to the matching
                // printed-text field:
                //   * face-up source  → `face_keywords`      (`effect_text`)
                //     e.g. BT21-029's `<Security A. +1>` is both a printed keyword
                //     and a face-up `grant_keyword` clause.
                //   * inherited source → `inherited_keywords` (`inherited_text`)
                //     e.g. ST1-07's inherited `<Security A. +1>` is both a printed
                //     inherited keyword and a `scope: inherited` `grant_keyword`.
                // The embedded DSL pack leaves these text fields empty (so the
                // grant still materializes there), but the real game loads them
                // from `cards.json`, where the double-count would otherwise show.
                if let Some(kw) = effect.granted_keyword {
                    let already_printed =
                        self.card_data_by_id(&self.card_data[data_index].card_id).is_some_and(|cd| {
                            if inherited_source {
                                inherited_keywords(cd).contains(&kw)
                            } else {
                                face_keywords(cd).contains(&kw)
                            }
                        });
                    if already_printed {
                        continue;
                    }
                }
                if let Some(condition) = &effect.condition {
                    let rctx = crate::effect_context::EffectReadContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    if !condition(&rctx) {
                        continue;
                    }
                }
                if let Some(process) = effect.process.as_ref() {
                    let mut ctx = crate::effect_context::EffectContext::new(
                        self,
                        source_card,
                        source_permanent,
                        controller,
                    );
                    process(&mut ctx);
                }
            }
        }

        // DigiLink Shape-B (task 6.2 / design D7): a linked card's `.linked()`
        // declarative grants materialize onto its HOST. The main loop above
        // scans top cards + under-stack digivolution sources but never
        // `linked_cards`, so a linked Digimon's continuous ESS (e.g. a `Raid`
        // keyword or DP buff) would never reach the host. This additive pass
        // runs each linked card's `.linked()` materializing declaratives with
        // `source_permanent = host` and `controller = host owner`, routing the
        // grant through the same modifier registry as inherited ESS — so
        // `Game::has_keyword` (via `modifiers.has_keyword`) and DP math see it.
        let mut linked_sources: Vec<(
            usize,
            crate::card_source::CardHandle,
            PermanentHandle,
            crate::enums::PlayerId,
        )> = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            let player_id = pid as crate::enums::PlayerId;
            for (index, perm) in player.battle_area.iter().enumerate() {
                let host = PermanentHandle {
                    player: player_id,
                    index: index as u8,
                };
                for linked in &perm.linked_cards {
                    linked_sources.push((
                        linked.data_index,
                        linked.handle(),
                        host,
                        player_id,
                    ));
                }
            }
        }
        for (data_index, source_card, host, controller) in linked_sources {
            let Some(effects) = self.effects_for_card(&self.card_data[data_index].card_id, source_card)
            else {
                continue;
            };
            for effect in effects.iter() {
                if !effect.linked || !effect.declarative {
                    continue;
                }
                if !effect.materializes_declarative_state || effect.process.is_none() {
                    continue;
                }
                if let Some(condition) = &effect.condition {
                    let rctx = crate::effect_context::EffectReadContext::new(
                        self,
                        source_card,
                        Some(host),
                        controller,
                    );
                    if !condition(&rctx) {
                        continue;
                    }
                }
                if let Some(process) = effect.process.as_ref() {
                    let mut ctx = crate::effect_context::EffectContext::new(
                        self,
                        source_card,
                        Some(host),
                        controller,
                    );
                    process(&mut ctx);
                }
            }
        }

        // Source-independent floating mass modifiers: re-scan the live candidate
        // set with each descriptor's predicate (relative to its `source_player`)
        // and install a materialized-declarative modifier on every current match
        // — so Digimon entering during the window receive the effect too
        // (G-CONTINUOUS-MASS-DP-DEBUFF). Cleared at the top of every tick along
        // with the source-bound declaratives; the descriptors themselves are
        // pruned at turn-end by `expire_floating_mass_modifiers`.
        if !self.floating_mass_modifiers.is_empty() {
            let floating = self.floating_mass_modifiers.clone();
            for fm in &floating {
                let mut ctx = crate::effect_context::EffectContext::new(
                    self,
                    fm.source_card,
                    None,
                    fm.source_player,
                );
                let matches = crate::dsl_cards::step::permanent_scan::scan(&ctx, &fm.filter, None);
                for h in matches {
                    // `Expiry::Permanent`: the per-permanent materialized entry is
                    // tick-ephemeral (cleared + re-installed every tick), so it must
                    // NOT be expired by the per-turn `expire_end_of_turn` pass — the
                    // floating DESCRIPTOR (pruned by `expire_floating_mass_modifiers`)
                    // is the sole lifetime authority. Using `fm.expiry` here would
                    // expire the entry one turn-end early relative to the descriptor
                    // (`*NextTurn` skips live on the descriptor, not the entry).
                    if let Some(payload) = &fm.payload {
                        // Continuous mass structured-payload aura (BT25-104:
                        // "all of your [Marcus Damon]s are also treated as 12000
                        // DP Digimon"): the materialized entry carries the
                        // `TreatAsDigimon` SynthIdentity. G-DSL-AURA-TREAT-AS-
                        // DIGIMON-SYNTH.
                        ctx.add_declarative_modifier_with_payload(
                            h,
                            fm.modifier,
                            fm.value,
                            Expiry::Permanent,
                            payload.clone(),
                        );
                    } else if let Some(imm) = fm.effect_immunity {
                        // Continuous controlled immunity (Q28 / BT20-059):
                        // the materialized entry carries the immunity filter.
                        ctx.add_declarative_modifier_with_immunity(
                            h,
                            fm.modifier,
                            fm.value,
                            Expiry::Permanent,
                            imm,
                        );
                    } else {
                        ctx.add_declarative_modifier(h, fm.modifier, fm.value, Expiry::Permanent);
                    }
                }
            }
        }
    }

    /// A cheap fingerprint of *all* declarative-relevant game state. When it
    /// is unchanged between two ticks, the materialized declarative modifier
    /// state is guaranteed identical, so the expensive full rebuild can be
    /// skipped. It deliberately over-captures (whole board + every zone's
    /// contents + memory/phase/turn + the non-materialized modifier digest):
    /// a false "changed" only costs an extra (correct) rebuild, while a missed
    /// input would be a stale-state bug — which the debug oracle in
    /// `tick_declarative_effects` catches across the test corpus.
    ///
    /// `HashSet`/`HashMap`-valued state (`face_up_security`,
    /// `effect_activations`) is folded in via order-independent digests so the
    /// fingerprint is stable across iteration-order changes; the modifier
    /// registry is folded in via [`ModifierRegistry::nonmaterialized_digest`].
    fn declarative_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.memory.hash(&mut h);
        self.current_phase.hash(&mut h);
        self.turn_player().hash(&mut h);
        self.turn_count.hash(&mut h);
        for player in &self.players {
            for (tag, zone) in [
                (0u8, &player.hand),
                (1u8, &player.deck),
                (2u8, &player.digitama_deck),
                (3u8, &player.security),
                (4u8, &player.trash),
            ] {
                tag.hash(&mut h);
                zone.len().hash(&mut h);
                for cs in zone {
                    hash_card_source_decl(cs, &mut h);
                }
            }
            // `face_up_security` is a `HashSet` — fold an order-independent
            // digest (len + scrambled sum) rather than iterating it directly.
            player.face_up_security.len().hash(&mut h);
            let fus: u64 = player
                .face_up_security
                .iter()
                .map(|&x| (x as u64).wrapping_mul(0x9E3779B97F4A7C15))
                .fold(0u64, u64::wrapping_add);
            fus.hash(&mut h);
            player.battle_area.len().hash(&mut h);
            for perm in &player.battle_area {
                hash_permanent_decl(perm, &mut h);
            }
            match &player.breeding_area {
                Some(perm) => {
                    1u8.hash(&mut h);
                    hash_permanent_decl(perm, &mut h);
                }
                None => 0u8.hash(&mut h),
            }
        }
        self.floating_mass_modifiers.len().hash(&mut h);
        for fm in &self.floating_mass_modifiers {
            format!("{fm:?}").hash(&mut h);
        }
        self.modifiers.nonmaterialized_digest().hash(&mut h);
        h.finish()
    }

    /// Track H Phase 4k — clear all per-permanent state when a
    /// permanent leaves the field, AND prune the corresponding
    /// granted-triggered-effect body registry entries. Wraps the
    /// narrower `ModifierRegistry::clear_permanent` so call sites
    /// don't have to remember the body-registry cleanup separately.
    /// Returns the count of body-registry entries removed (mostly for
    /// tests / instrumentation).
    pub fn clear_permanent_full(&mut self, handle: crate::permanent::PermanentHandle) -> usize {
        let body_ids = self
            .modifiers
            .drain_granted_triggered_ids_on_carrier(handle);
        self.modifiers.clear_permanent(handle);
        let mut removed = 0usize;
        for id in body_ids {
            if self
                .effect_queue
                .iter()
                .any(|queued| queued.granted_effect_id == Some(id))
            {
                continue;
            }
            if self.granted_effect_bodies.remove(id).is_some() {
                removed += 1;
            }
        }
        removed
    }

    /// Track H §3 — fire all granted triggered effects on `carrier`
    /// whose registered timing matches `timing`. Each body runs
    /// inline with `EffectContext::source_card` set to the grantor
    /// (mirroring DCGO `EffectSourceCard`) and `source_permanent` set
    /// to the carrier (mirroring DCGO `EffectSourcePermanent`).
    ///
    /// Inline-fire model (v1): the body runs synchronously, before
    /// `drain_effect_queue` resolves the rest of the trigger fan-out.
    /// Suitable for grants that mutate state without prompting (memory
    /// gain, modifier installs, etc.). Selection-driving granted
    /// bodies are not yet supported — they belong on the standard
    /// queue/drain path, which is a follow-up.
    pub fn fire_granted_triggered_effects(
        &mut self,
        carrier: crate::permanent::PermanentHandle,
        timing: crate::enums::EffectTiming,
    ) {
        let entries = self.modifiers.granted_triggered_for_timing(carrier, timing);
        for (source_card, source_player, body) in entries {
            // Immunity gate (mirrors the queue dispatcher): skip when the
            // carrier is unaffected by the grantor's effects — `<Progress>` /
            // attack-scoped immunity (Q2) or a general `CannotBeAffected`
            // opponent-effect immunity (Q17). The opponent-effect immunity
            // filter is source-kind-agnostic, so `Digimon` suffices on this
            // fallback path.
            if self.progress_excludes(carrier, Some(source_player))
                || self.permanent_is_unaffected_by_effect(
                    carrier,
                    source_player,
                    crate::enums::EffectSourceKind::Digimon,
                )
            {
                continue;
            }
            // D4 / DCGO: the granted body runs as the carrier's OWN effect
            // (effect_source_player = carrier.player), so a deletion it causes
            // is the carrier-controller's OwnEffect (judge-quiz Q16). The
            // grantor (`source_player`) is used only for the immunity gate.
            let mut ctx = crate::effect_context::EffectContext::new(
                self,
                source_card,
                Some(carrier),
                carrier.player,
            );
            body(&mut ctx);
        }
    }
}
