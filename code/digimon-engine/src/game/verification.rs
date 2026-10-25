use sha2::{Digest, Sha256};
#[path = "verification_cost_receipts.rs"]
mod cost_receipt_hash;
#[path = "verification_selected_pins.rs"]
mod selected_pin_hash;
#[path = "verification_security_play.rs"]
mod security_play_hash;
#[path = "verification_link_pins.rs"]
mod link_pin_projection_tests;

use crate::card_source::CardSource;
use crate::game::Game;
use crate::permanent::Permanent;
use crate::selection::QueuedEffect;

impl Game {
    /// Canonical gameplay-state digest used by replay/golden verification.
    ///
    /// This intentionally hashes observable gameplay state and deterministic
    /// resolver state, while excluding caches, loggers, event buffers, and RNG
    /// internals. Every `HashMap`/`HashSet` contribution is sorted first.
    pub fn verification_digest(&self) -> u64 {
        self.verification_digest_with_version(crate::recorder::VerificationDigestVersion::CURRENT)
    }

    /// Explicit legacy verification is for archival recordings, never an
    /// inferred fallback after a current digest mismatch.
    pub fn verification_digest_with_version(&self, version: crate::recorder::VerificationDigestVersion) -> u64 {
        use crate::recorder::VerificationDigestVersion;
        let mut h = Sha256::new();
        token(&mut h, match version {
            VerificationDigestVersion::LegacyV2 => "digimon-engine.verification-digest.v2",
            VerificationDigestVersion::V3 => "digimon-engine.verification-digest.v3",
            VerificationDigestVersion::V4 => "digimon-engine.verification-digest.v4",
        });
        if version != VerificationDigestVersion::LegacyV2 {
            // Explicit credit amount, paid status and owner after a parked payment. Receipt
            // references alone do not distinguish them. Preserve hook order,
            // including position relative to other continuation kinds.
            let credits: Vec<_> = self.after_selection_resume_hooks.0.iter().enumerate()
                .filter_map(|(index, hook)| match hook {
                    crate::resume::AfterSelectionHook::DigivolveReducerChain { state, amount, paid } =>
                        Some((index, state.acting_player, state.result_card(), *amount, *paid)),
                    _ => None,
                }).collect();
            debug_field(&mut h, "digivolve_reducer_hook_credits", &credits);
        }

        if version != VerificationDigestVersion::LegacyV2 {
            selected_pin_hash::write(&mut h, self, version==VerificationDigestVersion::V4);
            let dna = crate::game_actions::effect_dna::verification_rows(self, version==VerificationDigestVersion::V4);
            if !dna.is_empty() { debug_field(&mut h, "effect_dna_declarations", &dna); }
            let carriers = crate::replacement::verification_plural_carriers(self);
            if !carriers.is_empty() {
                debug_field(&mut h, "replacement_plural_carrier_identities", &carriers);
            }
        }

        field(&mut h, "rules.player_count", self.rules.player_count);
        field(&mut h, "rules.security_count", self.rules.security_count);
        field(&mut h, "rules.field_slots", self.rules.field_slots);
        field(&mut h, "turn_count", self.turn_count);
        debug_field(&mut h, "current_phase", &self.current_phase);
        if !self.effect_digivolve_scopes.is_empty() {
            debug_field(
                &mut h,
                "effect_digivolve_scopes",
                &self
                    .effect_digivolve_scopes
                    .iter()
                    .map(|s| {
                        (
                            cost_receipt_hash::effect_declaration(&s.declaration),
                            s.identity.window_started,
                            s.identity.effective_cost,
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(resume) = &self.pending_would_play_resume {
            debug_field(&mut h, "pending_would_play_resume", resume);
        }
        if let Some(resume) = &self.pending_would_digivolve_resume {
            debug_field(
                &mut h,
                "pending_would_digivolve",
                &(
                    resume.player,
                    resume.permanent,
                    resume.host_identity,
                    resume.card,
                    resume.effective_cost,
                    resume.app_fusion,
                    resume.window_started,
                ),
            );
            // Hash the referenced entries' local positions rather than the
            // allocator token, so equivalent clones/transpositions agree.
            if let Some((identity_order, no_dp_order)) = resume.treated_identity_orders {
                let host =
                    self.resolve_digivolve_host_identity(resume.player, resume.host_identity);
                let identity_ref = host.and_then(|p| {
                    self.modifiers
                        .get(p, crate::enums::ModifierType::TreatAsDigimon)
                        .iter()
                        .position(|e| e.install_order == identity_order)
                });
                let no_dp_ref = no_dp_order.map(|order| {
                    host.and_then(|p| {
                        self.modifiers
                            .get(p, crate::enums::ModifierType::DontHaveDp)
                            .iter()
                            .position(|e| e.install_order == order)
                    })
                });
                debug_field(
                    &mut h,
                    "would_digivolve_identity_refs",
                    &(identity_ref, no_dp_ref),
                );
            }
        }
        if !self.pending_security_moves.is_empty() {
            debug_field(
                &mut h,
                "pending_security_moves",
                &self.pending_security_moves,
            );
        }
        if !self.security_placement_receipts.is_empty() {
            debug_field(
                &mut h,
                "security_placement_receipts",
                &self.security_placement_receipts,
            );
        }
        if !self.field_source_receipts.is_empty() {
            debug_field(&mut h, "field_source_receipts", &self.field_source_receipts);
        }
        if !self.pending_field_top_source_moves.is_empty() {
            debug_field(
                &mut h,
                "pending_field_top_source_moves",
                &self.pending_field_top_source_moves,
            );
        }
        field(&mut h, "memory", self.memory);
        field(&mut h, "memory_pair.0", self.memory_pair.0);
        field(&mut h, "memory_pair.1", self.memory_pair.1);
        field(&mut h, "turn_player_idx", self.turn_player_idx);
        debug_field(&mut h, "turn_order", &self.turn_order);
        field(&mut h, "game_over", self.game_over);
        debug_field(&mut h, "winner", &self.winner);
        debug_field(
            &mut h,
            "terminal_outcome_reason",
            &self.terminal_outcome_reason,
        );
        debug_field(&mut h, "n_digivolutions", &self.n_digivolutions);
        debug_field(&mut h, "n_dna_digivolutions", &self.n_dna_digivolutions);
        debug_field(
            &mut h,
            "n_digivolve_driven_attacks",
            &self.n_digivolve_driven_attacks,
        );
        debug_field(
            &mut h,
            "digimon_attacks_this_turn",
            &self.digimon_attacks_this_turn,
        );
        debug_field(&mut h, "mulligan_pending", &self.mulligan_pending);
        debug_field(&mut h, "mulligan_used", &self.mulligan_used);

        for (player_idx, player) in self.players.iter().enumerate() {
            section(&mut h, "player", player_idx);
            field(&mut h, "player.id", player.id);
            field(&mut h, "player.is_eliminated", player.is_eliminated);
            field(&mut h, "player.commander_tax", player.commander_tax);
            write_zone(&mut h, self, "hand", &player.hand);
            write_zone(&mut h, self, "deck", &player.deck);
            write_zone(&mut h, self, "digitama_deck", &player.digitama_deck);
            write_zone(&mut h, self, "security", &player.security);
            write_zone(&mut h, self, "trash", &player.trash);
            write_permanent_zone(&mut h, self, "battle_area", &player.battle_area);
            match &player.breeding_area {
                Some(permanent) => {
                    token(&mut h, "breeding_area=some");
                    write_permanent(&mut h, self, permanent);
                }
                None => token(&mut h, "breeding_area=none"),
            }
            match &player.commander_zone {
                Some(card) => {
                    token(&mut h, "commander_zone=some");
                    write_card(&mut h, self, card);
                }
                None => token(&mut h, "commander_zone=none"),
            }
            let mut face_up: Vec<u16> = player.face_up_security.iter().copied().collect();
            face_up.sort_unstable();
            debug_field(&mut h, "face_up_security", &face_up);
            // `{Security}` [X Per Turn] counts (§15-14-1-2: the count is game
            // state), sorted like a permanent's `effect_activations`. Only
            // non-zero counts are hashed in V3 and later, so a state with no
            // counted `{Security}` activation (and every archival LegacyV2
            // recording) keeps its digest.
            if version != crate::recorder::VerificationDigestVersion::LegacyV2 {
                let mut counts: Vec<((u16, u8), u8)> = player
                    .security_effect_activations
                    .iter()
                    .filter(|(_, &count)| count > 0)
                    .map(|(&(card, key), &count)| ((card.0, key), count))
                    .collect();
                counts.sort_unstable();
                for ((card, key), count) in counts {
                    token(&mut h, "security_effect_activation");
                    field(&mut h, "security_effect_activation.card", card);
                    field(&mut h, "security_effect_activation.key", key);
                    field(&mut h, "security_effect_activation.count", count);
                }
            }
            debug_field(&mut h, "last_security_reveal", &player.last_security_reveal);
            debug_field(&mut h, "original_deck", &player.original_deck);
            debug_field(&mut h, "opaque_deck_state", &player.opaque_deck_state);
        }

        write_zone(&mut h, self, "revealed_cards", &self.revealed_cards);
        for entry in self.modifiers.verification_summary() {
            field(&mut h, "modifier", entry);
        }
        write_effect_queue(&mut h, &self.effect_queue);

        match self.pending_selection.as_ref() {
            Some(selection) => {
                token(&mut h, "pending_selection=some");
                debug_field(&mut h, "pending_selection.kind", &selection.kind);
                field(
                    &mut h,
                    "pending_selection.selecting_player",
                    selection.selecting_player,
                );
                debug_field(
                    &mut h,
                    "pending_selection.previous_phase",
                    &selection.previous_phase,
                );
                debug_field(
                    &mut h,
                    "pending_selection.valid_action_ids",
                    &selection.valid_action_ids,
                );
                field(
                    &mut h,
                    "pending_selection.is_optional",
                    selection.is_optional,
                );
                field(
                    &mut h,
                    "pending_selection.source_card",
                    selection.source_card.0,
                );
                debug_field(
                    &mut h,
                    "pending_selection.source_permanent",
                    &selection.source_permanent,
                );
                debug_field(
                    &mut h,
                    "pending_selection.source_kind",
                    &selection.source_kind,
                );
                debug_field(
                    &mut h,
                    "pending_selection.zone_owner",
                    &selection.zone_owner,
                );
                // Two consecutive prompts of one effect can agree on every
                // field above (same kind / chooser / candidates / source —
                // ST23-09's "Suspend 1 …" then "Return 1 … suspended …" with a
                // single opponent Digimon), so the prompt text and the
                // resume continuation below are what tell them apart.
                field(&mut h, "pending_selection.prompt", &selection.prompt);
                if let Some(choices) = &selection.effect_choices {
                    for choice in choices {
                        token(&mut h, "effect_choice");
                        field(&mut h, "effect_choice.action_id", choice.action_id);
                        field(&mut h, "effect_choice.label", &choice.label);
                        debug_field(&mut h, "effect_choice.source_card", &choice.source_card);
                        debug_field(&mut h, "effect_choice.source_kind", &choice.source_kind);
                        debug_field(&mut h, "effect_choice.timing", &choice.timing);
                        field(&mut h, "effect_choice.is_optional", choice.is_optional);
                        if let Some(count) = &choice.count_choice {
                            debug_field(&mut h, "effect_choice.count_choice", count);
                        }
                    }
                }
            }
            None => token(&mut h, "pending_selection=none"),
        }

        if version == VerificationDigestVersion::V4 {
            cost_receipt_hash::write_pending_play_credits(&mut h, self);
            security_play_hash::write(&mut h, self);
            debug_field(&mut h, "card_choice.next_id", &self.next_card_choice_id);
            debug_field(&mut h, "card_choice.security_shuffle_generations", &self.security_shuffle_generations);
            debug_field(&mut h, "card_choice.resolved", &self.resolved_card_choice);
            if let Some(owner) = self.pending_selection.as_ref().and_then(|s| s.card_choice.as_ref()) {
                debug_field(&mut h, "pending_selection.card_choice", owner);
            }
        }
        write_resume_stack(&mut h, self.pending_selection_resume.as_ref(), self, version);
        let windows: Vec<_> = self
            .parked_replacement
            .iter()
            .filter_map(|p| p.window.as_ref())
            .collect();
        if !windows.is_empty() {
            debug_field(&mut h, "parked_replacement_windows", &windows);
            for (index, window) in windows.iter().enumerate() {
                if let Some(owned) = window.verification_play_declaration(self.pending_would_play_resume.as_ref()) {
                    if version==VerificationDigestVersion::V4 || self.pending_would_play_resume.as_ref().is_none_or(|current|format!("{owned:?}")!=format!("{current:?}")) {
                        debug_field(&mut h, "parked_replacement_window.play_declaration", &(index, owned));
                    }
                }
            }
        }
        let native_tails: Vec<_> = self
            .parked_replacement
            .iter()
            .filter(|p| p.window.is_some())
            .map(|p| p.outer_conts.len())
            .collect();
        if native_tails.iter().any(|n| *n > 0) {
            debug_field(&mut h, "replacement_native_tail_lengths", &native_tails);
        }
        if !self.return_receipts.is_empty() {
            debug_field(&mut h,"return_receipts",&self.return_receipts);
        }
        if !self.deletion_receipts.is_empty() {
            debug_field(&mut h, "deletion_receipts", &self.deletion_receipts);
            debug_field(
                &mut h,
                "current_deletion_receipt",
                &self.current_deletion_receipt,
            );
            debug_field(
                &mut h,
                "deletion_receipt_batch_rest",
                &self.pending_deletion_batch_rest,
            );
            let parked: Vec<_> = self
                .parked_replacement
                .iter()
                .map(|p| p.deletion_receipt)
                .collect();
            debug_field(&mut h, "deletion_receipt_parked", &parked);
        }

        debug_field(&mut h, "pending_attack", &self.pending_attack);
        debug_field(&mut h, "pending_security", &self.pending_security);
        debug_field(
            &mut h,
            "pending_effect_security_removal",
            &self.pending_effect_security_removal,
        );
        debug_field(&mut h, "pending_option", &self.pending_option);
        debug_field(&mut h, "security_resolution", &self.security_resolution);
        debug_field(&mut h, "pending_granted_fires", &self.pending_granted_fires);
        field(
            &mut h,
            "next_granted_effect_id",
            self.next_granted_effect_id,
        );
        field(&mut h, "replacement_depth", self.replacement_depth);
        debug_field(
            &mut h,
            "replacement_pending_outcome",
            &self.replacement_pending_outcome,
        );
        debug_field(
            &mut h,
            "last_play_order_choice",
            &self.last_play_order_choice,
        );
        // Only outstanding source-play results affect future resolution. The
        // allocator counter is a private freshness token, not gameplay state;
        // empty/default extensions must preserve existing settled goldens.
        cost_receipt_hash::write(&mut h, self);
        if !self.prepared_source_plays.is_empty() {
            debug_field(&mut h, "prepared_source_plays", &self.prepared_source_plays);
        }
        if !self.source_play_receipts.is_empty() {
            debug_field(&mut h, "source_play_receipts", &self.source_play_receipts);
        }
        field(
            &mut h,
            "pending_player_digivolve_reduction",
            self.pending_player_digivolve_reduction,
        );
        field(
            &mut h,
            "pending_interactive_digivolve_reduction",
            self.pending_interactive_digivolve_reduction,
        );
        field(
            &mut h,
            "pending_interactive_option_use_reduction",
            self.pending_interactive_option_use_reduction,
        );
        field(
            &mut h,
            "interactive_option_use_reducer_prompted",
            self.interactive_option_use_reducer_prompted,
        );
        debug_field(
            &mut h,
            "pending_cost_reduction_amount_override",
            &self.pending_cost_reduction_amount_override,
        );
        debug_field(
            &mut h,
            "pending_digivolve_route_choice",
            &self.pending_digivolve_route_choice,
        );
        debug_field(
            &mut h,
            "current_trigger_context",
            &self.current_trigger_context,
        );
        debug_field(
            &mut h,
            "current_deletion_cause",
            &self.current_deletion_cause,
        );
        debug_field(
            &mut h,
            "current_deletion_event_cause_override",
            &self.current_deletion_event_cause_override,
        );
        // Hashed as the former single slot (`Option<tuple>`) while at most one
        // attack is queued, so recorded digests are unchanged; a deeper queue
        // (impossible before the FIFO) hashes the whole list.
        match self.parked_overclock_attacks.as_slice() {
            [] => debug_field(
                &mut h,
                "pending_overclock_attack",
                &None::<(crate::enums::PlayerId, crate::card_source::CardHandle, crate::enums::PlayerId, u64, u64)>,
            ),
            [one] => debug_field(&mut h, "pending_overclock_attack", &Some(one.digest_tuple())),
            many => {
                let tuples: Vec<_> = many.iter().map(|p| p.digest_tuple()).collect();
                debug_field(&mut h, "pending_overclock_attack", &tuples);
            }
        }
        let mut declined_overclock: Vec<u16> = self
            .declined_overclock_this_eot
            .iter()
            .map(|handle| handle.0)
            .collect();
        declined_overclock.sort_unstable();
        debug_field(&mut h, "declined_overclock_this_eot", &declined_overclock);
        debug_field(&mut h, "current_dna_origin", &self.current_dna_origin);
        debug_field(
            &mut h,
            "dsl_replacement_outcome",
            &self.dsl_replacement_outcome,
        );
        field(&mut h, "in_counter_window", self.in_counter_window);

        let digest = h.finalize();
        u64::from_le_bytes(
            digest[0..8]
                .try_into()
                .expect("sha256 digest has at least 8 bytes"),
        )
    }
}

fn write_zone(hasher: &mut Sha256, game: &Game, label: &str, cards: &[CardSource]) {
    section(hasher, label, cards.len());
    for card in cards {
        write_card(hasher, game, card);
    }
}

fn write_permanent_zone(hasher: &mut Sha256, game: &Game, label: &str, permanents: &[Permanent]) {
    section(hasher, label, permanents.len());
    for permanent in permanents {
        write_permanent(hasher, game, permanent);
    }
}

fn write_permanent(hasher: &mut Sha256, game: &Game, permanent: &Permanent) {
    token(hasher, "permanent");
    write_zone(hasher, game, "card_sources", &permanent.card_sources);
    write_zone(hasher, game, "linked_cards", &permanent.linked_cards);
    field(hasher, "is_suspended", permanent.is_suspended);
    field(hasher, "turn_played", permanent.turn_played);
    field(hasher, "turn_digivolved", permanent.turn_digivolved);
    field(hasher, "attacks_this_turn", permanent.attacks_this_turn);
    field(hasher, "is_attacking", permanent.is_attacking);
    debug_field(hasher, "option_state", &permanent.option_state);
    // Hashed only when set, so states without an effect-placed breeding
    // permanent keep their digest (§17-1-3-2-3 marker).
    if permanent.placed_by_effect {
        token(hasher, "placed_by_effect");
    }

    let mut activations: Vec<_> = permanent.effect_activations.iter().collect();
    activations.sort_by_key(|((card, slot), _)| (card.0, *slot));
    for ((card, slot), count) in activations {
        token(hasher, "effect_activation");
        field(hasher, "effect_activation.card", card.0);
        field(hasher, "effect_activation.slot", *slot);
        field(hasher, "effect_activation.count", *count);
    }
}

fn write_card(hasher: &mut Sha256, game: &Game, card: &CardSource) {
    token(hasher, "card");
    if card.is_opaque_placeholder {
        field(hasher, "card_id", "<opaque-security-placeholder>");
    } else {
        field(hasher, "card_id", card.card_id(&game.card_data));
    }
    field(hasher, "data_index", card.data_index);
    field(hasher, "owner", card.owner);
    field(hasher, "card_index", card.card_index);
    field(hasher, "is_token", card.is_token);
    debug_field(hasher, "also_treated_as", &card.also_treated_as);
    debug_field(hasher, "reveal_overlay", &card.reveal_overlay);
    field(hasher, "face_down", card.face_down);
    field(hasher, "is_opaque_placeholder", card.is_opaque_placeholder);
}

fn write_effect_queue(hasher: &mut Sha256, queue: &std::collections::VecDeque<QueuedEffect>) {
    section(hasher, "effect_queue", queue.len());
    for effect in queue {
        token(hasher, "queued_effect");
        field(hasher, "queued.source_card", effect.source_card.0);
        debug_field(hasher, "queued.source_permanent", &effect.source_permanent);
        debug_field(hasher, "queued.source_kind", &effect.source_kind);
        debug_field(
            hasher,
            "queued.attribution_source_card",
            &effect.attribution_source_card,
        );
        debug_field(
            hasher,
            "queued.attribution_source_kind",
            &effect.attribution_source_kind,
        );
        field(
            hasher,
            "queued.bypass_once_per_turn",
            effect.bypass_once_per_turn,
        );
        field(hasher, "queued.controller", effect.controller);
        debug_field(hasher, "queued.timing", &effect.timing);
        debug_field(hasher, "queued.trigger_context", &effect.trigger_context);
        field(hasher, "queued.effect_slot", effect.effect_slot);
        field(hasher, "queued.is_optional", effect.is_optional);
        field(hasher, "queued.is_turn_player", effect.is_turn_player);
        field(hasher, "queued.card_id", &effect.card_id);
        field(
            hasher,
            "queued.allow_below_top_liveness",
            effect.allow_below_top_liveness,
        );
        debug_field(
            hasher,
            "queued.dna_origin_context",
            &effect.dna_origin_context,
        );
        debug_field(
            hasher,
            "queued.granted_effect_id",
            &effect.granted_effect_id,
        );
        // Pending processing (§18-1-2). Written only when present, so the
        // digest of a queue without it is unchanged. Not the body's `Debug`
        // dump: its `Bindings` keep a `HashMap` (unstable order).
        match &effect.pending_processing {
            None => {}
            Some(crate::scheduled_effects::PendingProcessing::Scheduled(scheduled)) => {
                token(hasher, "queued.scheduled");
                debug_field(hasher, "queued.scheduled.when", &scheduled.when);
                field(
                    hasher,
                    "queued.scheduled.scheduled_at_turn",
                    scheduled.scheduled_at_turn,
                );
                field(hasher, "queued.scheduled.body_len", scheduled.body.len());
            }
            Some(crate::scheduled_effects::PendingProcessing::ProvenanceDeletion(entry)) => {
                token(hasher, "queued.provenance_deletion");
                field(hasher, "queued.provenance_deletion.token", entry.token.0);
                field(
                    hasher,
                    "queued.provenance_deletion.controller",
                    entry.controller,
                );
            }
        }
    }
}

/// Stable fingerprint of the parked selection continuation
/// (`Game::pending_selection_resume`): per frame its variant name and, for a
/// DSL `RunTail`, the slot it binds and how much body is left. Not the full
/// `Debug` dump — `Bindings` keeps a `HashMap`, whose order is not stable
/// across separately-built games (recording vs replay).
fn write_resume_stack(
    hasher: &mut Sha256,
    stack: Option<&crate::resume::ResumeStack>,
    game: &Game,
    version: crate::recorder::VerificationDigestVersion,
) {
    let Some(stack) = stack else {
        token(hasher, "pending_selection_resume=none");
        return;
    };
    section(hasher, "pending_selection_resume", stack.frames.len());
    if let Some(state) = crate::game_actions::play_cost_assignment::active_play_cost_order(stack) {
        debug_field(hasher, "resume.play_cost_order", &state.order);
        if game.cost_payment_receipts.is_empty() {
            debug_field(
                hasher,
                "resume.play_cost_adjustments",
                &state.accumulated_reduction,
            );
        } else {
            debug_field(
                hasher,
                "resume.play_cost_adjustments",
                &cost_receipt_hash::adjustment(game, &state.accumulated_reduction),
            );
        }
    }
    if let Some(pins) = crate::dsl_cards::step::source_prefix_payment::active_prefix_pins(stack) {
        debug_field(hasher, "resume.source_prefix_pins", &pins);
    }
    if let Some(page) = crate::count_choice::active_count_page(stack) {
        debug_field(hasher, "resume.active_count_choice", page);
    }
    for frame in &stack.frames {
        if let crate::resume::ResumeFrame::OptionalReplacement(state) = frame {
            if let Some(window) = state.window.as_ref() {
                debug_field(hasher, "replacement.window", window);
                if let Some(owned) = window.verification_play_declaration(game.pending_would_play_resume.as_ref()) {
                    if version==crate::recorder::VerificationDigestVersion::V4 || game.pending_would_play_resume.as_ref().is_none_or(|current|format!("{owned:?}")!=format!("{current:?}")) {
                        debug_field(hasher, "replacement.window.play_declaration", owned);
                    }
                }
                debug_field(
                    hasher,
                    "replacement.selected",
                    &(
                        state.source_card,
                        state.source_identity,
                        &state.candidate_kind,
                        state.controller,
                    ),
                );
                debug_field(hasher, "replacement.order", &state.order);
            }
        }
        if let crate::resume::ResumeFrame::ReplacementBody { inner, .. } = frame {
            write_resume_stack(hasher, Some(inner), game, version);
        }
        let dump = format!("{frame:?}");
        let variant = dump
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("");
        field(hasher, "resume.frame", variant);
        if let crate::resume::ResumeFrame::RunTail {
            bind_as,
            inner_tail,
            outer_conts,
            select_kind,
            ..
        } = frame
        {
            match select_kind {
                crate::resume::ResumeSelectKind::SourcePrefixHost(state) => {
                    debug_field(hasher, "resume.source_prefix_host", state)
                }
                crate::resume::ResumeSelectKind::SourcePrefixCount { state, .. } => {
                    debug_field(hasher, "resume.source_prefix_count", state)
                }
                crate::resume::ResumeSelectKind::DeDigivolveCount { page, state } => {
                    debug_field(hasher, "resume.de_digivolve_count", page);
                    debug_field(hasher, "resume.de_digivolve_count_state", state)
                }
                _ => {}
            }
            if let crate::resume::ResumeSelectKind::CountChoice(page) = select_kind {
                debug_field(hasher, "resume.count_choice", page);
            }
            debug_field(hasher, "resume.bind_as", bind_as);
            let length=if version==crate::recorder::VerificationDigestVersion::V4 {inner_tail.len()}
                else {inner_tail.iter().filter(|step|!security_play_hash::is_runtime_step(step)).count()};
            field(hasher, "resume.inner_tail", length);
            field(hasher, "resume.outer_conts", outer_conts.len());
        }
    }
}

fn section(hasher: &mut Sha256, label: &str, len: usize) {
    field(hasher, "section", label);
    field(hasher, "len", len);
}

fn field<T: std::fmt::Display>(hasher: &mut Sha256, name: &str, value: T) {
    token(hasher, name);
    token(hasher, &value.to_string());
}

fn debug_field<T: std::fmt::Debug>(hasher: &mut Sha256, name: &str, value: &T) {
    token(hasher, name);
    token(hasher, &format!("{value:?}"));
}

fn token(hasher: &mut Sha256, value: &str) {
    hasher.update(value.as_bytes());
    hasher.update([0]);
}

#[cfg(test)]
mod would_play_digest_tests {
    #[test]
    fn pending_would_play_controller_changes_digest_without_visible_state_change() {
        use crate::debug_runner::{make_test_card, DebugRunner};
        use crate::game::{PendingWouldPlayOrigin, PendingWouldPlayResume};
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("DIGEST-PLAY", "Digest play"))
            .hand(0, &["DIGEST-PLAY"])
            .deck(0, &["DIGEST-PLAY"; 10])
            .deck(1, &["DIGEST-PLAY"; 10])
            .start();
        r.skip_mulligan();
        let card = r.game.players[0].hand[0].handle();
        r.game.pending_would_play_resume = Some(PendingWouldPlayResume {
            player: 0,
            card,
            effective_cost: 0,
            origin: PendingWouldPlayOrigin::Hand,
            effect_initiated: true,
            play_controller: 0,
            suppress_on_play: false,
        });
        let mut other = r.game.clone();
        other
            .pending_would_play_resume
            .as_mut()
            .unwrap()
            .play_controller = 1;
        assert_eq!(r.game.memory, other.memory);
        assert_eq!(r.game.players[0].hand.len(), other.players[0].hand.len());
        assert_eq!(
            r.game.pending_selection.is_some(),
            other.pending_selection.is_some()
        );
        assert_ne!(
            r.game.verification_digest(),
            other.verification_digest(),
            "causing controller governs deferred play legality"
        );
        assert_eq!(
            other.verification_digest(),
            other.clone().verification_digest()
        );
    }
}

#[cfg(test)]
mod security_activation_digest_tests {
    use crate::card_source::CardHandle;
    use crate::recorder::VerificationDigestVersion;

    /// G-ENGINE-SECURITY-ZONE-OPT-UNTRACKED residual (final review I2): a
    /// counted `{Security}` [OPT] use is game state (§15-14-1-2), so two
    /// clones differing only in it must not be digest-equal; a refunded
    /// (zero) count and an archival LegacyV2 digest are unchanged.
    #[test]
    fn counted_security_activation_is_hashed_and_clone_stable() {
        let r = crate::debug_runner::DebugRunner::builder().start();
        let before = r.game.verification_digest();
        let legacy = r.game.verification_digest_with_version(VerificationDigestVersion::LegacyV2);
        let mut counted = r.game.clone();
        counted.players[0].record_security_activation(CardHandle(3), 0);
        assert_ne!(counted.verification_digest(), before);
        assert_eq!(
            counted.verification_digest(),
            counted.clone().verification_digest()
        );
        assert_eq!(
            counted.verification_digest_with_version(VerificationDigestVersion::LegacyV2),
            legacy
        );
        let mut other_key = r.game.clone();
        other_key.players[0].record_security_activation(CardHandle(3), 1);
        assert_ne!(other_key.verification_digest(), counted.verification_digest());
        counted.players[0].unrecord_security_activation(CardHandle(3), 0);
        assert_eq!(counted.verification_digest(), before, "a refunded count is no count");
    }

    #[test]
    fn counted_security_activation_changes_the_loop_signature() {
        let r = crate::debug_runner::DebugRunner::builder().start();
        let site = crate::game::infinite_loop::LoopSite::Queue;
        let before = r.game.loop_signature(site);
        let mut counted = r.game.clone();
        counted.players[1].record_security_activation(CardHandle(9), 2);
        assert_ne!(counted.loop_signature(site), before);
        assert_eq!(counted.loop_signature(site), counted.clone().loop_signature(site));
    }
}

#[cfg(test)]
mod source_receipt_digest_tests {
    #[test]
    fn settled_source_receipt_sequence_is_not_gameplay_state() {
        let r = crate::debug_runner::DebugRunner::builder().start();
        let before = r.game.verification_digest();
        let mut branch = r.game.clone();
        branch.next_source_play_receipt = 999;
        assert_eq!(
            branch.verification_digest(),
            before,
            "unused next receipt ID cannot change gameplay or settled replay digest"
        );
    }
    #[test]
    fn outstanding_source_play_commit_result_is_hashed_and_clone_stable() {
        let r = crate::debug_runner::DebugRunner::builder().start();
        let mut pending = r.game.clone();
        pending
            .source_play_receipts
            .push((7, crate::card_source::CardHandle(0), None));
        assert_ne!(pending.verification_digest(), r.game.verification_digest());
        let mut committed = pending.clone();
        committed.source_play_receipts[0].2 = Some((
            crate::permanent::PermanentHandle {
                player: 0,
                index: 0,
            },
            31,
        ));
        assert_ne!(
            committed.verification_digest(),
            pending.verification_digest()
        );
        assert_eq!(
            committed.verification_digest(),
            committed.clone().verification_digest()
        );
        committed.source_play_receipts.clear();
        committed.next_source_play_receipt = 8;
        assert_eq!(
            committed.verification_digest(),
            r.game.verification_digest(),
            "consumed private receipt leaves settled gameplay digest unchanged"
        );
    }
}

#[cfg(test)]
mod deletion_receipt_digest_tests {
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::enums::EffectTiming;
    use crate::selection::TriggerSource;
    struct Protect;
    impl crate::effect::CardEffect for Protect {
        fn effects(&self, c: crate::card_source::CardHandle) -> Vec<crate::effect::Effect> {
            vec![crate::effect::Effect::when_would_be_deleted(c)
                .optional()
                .name("Digest deletion protection")
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }
    fn parked() -> crate::game::Game {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(
                r#"
card: DELETION-DIGEST
name: Deletion digest driver
kind: digimon
color: [red]
cost: 1
level: 3
dp: 2000
effects:
  - when: on_play
    process:
      - delete_all_permanents:
          over: { of: opponent, zone: [battle_area], kind: digimon }
      - gain_memory: 1
"#,
            )
            .unwrap()
            .add_card(make_test_card("VICTIM", "Victim"))
            .deck(0, &["VICTIM"; 12])
            .deck(1, &["VICTIM"; 12])
            .memory(3)
            .start();
        r.skip_mulligan();
        let source = r.place_on_field(0, "DELETION-DIGEST", None);
        r.register_effect("VICTIM", std::sync::Arc::new(Protect));
        r.place_on_field(1, "VICTIM", None);
        r.game
            .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
        r.game.drain_effect_queue();
        assert!(r.game.pending_selection.is_some());
        assert_eq!(r.game.deletion_receipts.len(), 1);
        r.game
    }
    #[test]
    fn allocator_only_differences_do_not_change_parked_or_settled_semantics() {
        let r = DebugRunner::builder().start();
        let mut settled = r.game.clone();
        settled.next_deletion_receipt += 97;
        assert_eq!(r.game.verification_digest(), settled.verification_digest());
        let pending = parked();
        let mut branch = pending.clone();
        branch.next_deletion_receipt += 97;
        assert_eq!(
            pending.verification_digest(),
            branch.verification_digest(),
            "unused next operation ID cannot change the same live pending choice"
        );
    }
    #[test]
    fn actual_result_and_card_kind_affect_digest_and_clone_is_stable() {
        let pending = parked();
        let mut branch = pending.clone();
        branch.deletion_receipts[0].1.push((
            crate::permanent::PermanentHandle {
                player: 1,
                index: 0,
            },
            Some(3000),
            true,
            (0, 1),
        ));
        assert_ne!(pending.verification_digest(), branch.verification_digest());
        let mut non_digimon = branch.clone();
        non_digimon.deletion_receipts[0].1[0].2 = false;
        assert_ne!(
            non_digimon.verification_digest(),
            branch.verification_digest()
        );
        assert_eq!(
            branch.verification_digest(),
            branch.clone().verification_digest()
        );
    }
    #[test]
    fn consuming_real_replacement_receipt_leaves_no_private_digest_residue() {
        for action in [
            crate::action::space::PASS,
            crate::action::space::REPLACEMENT_ACCEPT,
        ] {
            let mut original = parked();
            let mut branch = original.clone();
            branch.next_deletion_receipt += 97;
            for game in [&mut original, &mut branch] {
                let player = game.pending_selection.as_ref().unwrap().selecting_player;
                assert!(crate::action::build_action_mask(game, player)[action as usize] > 0.0);
                game.resolve_selection(player, action).unwrap();
                assert!(game.pending_selection.is_none());
                assert!(game.deletion_receipts.is_empty());
                assert!(game.current_deletion_receipt.is_none());
                assert!(game.pending_deletion_batch_rest.is_empty());
            }
            assert_eq!(
                original.verification_digest(),
                branch.verification_digest(),
                "consumed allocator history is non-semantic"
            );
        }
    }
}

#[cfg(test)]
mod cost_receipt_hash_tests {
    use crate::{
        action::{
            build_action_mask,
            space::{PASS, PLAY_HAND_START},
        },
        debug_runner::{make_test_card, DebugRunner},
        dsl_cards::step::StepRuntime,
        game::Game,
        resume::{AfterSelectionHook, ResumeFrame},
    };
    use digimon_dsl::compiled::CompiledStep;
    fn fixture() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(
                r#"
card: HASH-SET
name: Hash setter
kind: digimon
level: 4
color: [white]
cost: 20
dp: 5000
effects:
  - kind: cost_assignment
    set_cost: 3
    reduction_timing: before_pay_cost
    when_playing_this: true
    optional: true
    pay_cost:
      - select_own_permanent:
          filter: { name_is: Payment ally }
          bind_as: fee
          prompt: Choose the actual payment ally
      - suspend: { target: fee }
"#,
            )
            .unwrap()
            .add_card(make_test_card("HASH-FEE", "Payment ally"))
            .hand(0, &["HASH-SET"])
            .deck(0, &["HASH-FEE"; 12])
            .deck(1, &["HASH-FEE"; 12])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.place_on_field(0, "HASH-FEE", None);
        r.game.enter_main_phase();
        r
    }
    fn choose(g: &mut Game, a: u16) {
        let p = g.pending_selection.as_ref().unwrap().selecting_player;
        assert_eq!(build_action_mask(g, p)[a as usize], 1.0);
        g.resolve_selection(p, a).unwrap();
    }
    fn park(r: &mut DebugRunner) {
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
        r.game.decode_action(PLAY_HAND_START, 0);
        choose(&mut r.game, 30);
        assert_eq!(r.game.cost_payment_receipts.len(), 1);
        assert!(r.game.pending_selection.is_some());
    }
    fn rewrite_ids(g: &mut Game, old: u64, new: u64) {
        for receipt in &mut g.cost_payment_receipts {
            if receipt.id == old {
                receipt.id = new;
            }
        }
        for hook in &mut g.after_selection_resume_hooks.0 {
            if let AfterSelectionHook::PlayCostContinuation {
                accumulated_reduction,
                ..
            } = hook
            {
                if accumulated_reduction.pending_payment == Some(old) {
                    accumulated_reduction.pending_payment = Some(new);
                }
            }
        }
        if let Some(stack) = g.pending_selection_resume.as_mut() {
            for frame in &mut stack.frames {
                if let ResumeFrame::RunTail { inner_tail, .. } = frame {
                    for step in std::sync::Arc::make_mut(inner_tail) {
                        if let CompiledStep::CompleteCostPayment { receipt } = step {
                            if *receipt == old {
                                *receipt = new;
                            }
                        }
                    }
                }
            }
        }
        if g.active_cost_payment_receipt == Some(old) {
            g.active_cost_payment_receipt = Some(new);
        }
    }
    #[test]
    fn receipt_token_history_is_not_gameplay_and_clones_resume_identically() {
        let mut a = fixture();
        let mut b = fixture();
        b.game = a.game.clone();
        b.game.next_cost_payment_receipt = 71;
        park(&mut a);
        park(&mut b);
        assert_eq!(
            a.game.verification_digest(),
            b.game.verification_digest(),
            "receipt allocator history is alpha-renamable"
        );
        let mut branch = b.game.clone();
        for g in [&mut a.game, &mut b.game, &mut branch] {
            choose(g, 100);
            assert!(g.pending_selection.is_none());
            assert!(g.cost_payment_receipts.is_empty());
            assert_eq!(g.memory, 7);
            assert!(g.players[0].battle_area[0].is_suspended);
        }
        assert_eq!(a.game.verification_digest(), b.game.verification_digest());
        assert_eq!(b.game.verification_digest(), branch.verification_digest());
    }
    #[test]
    fn paired_receipt_renaming_preserves_transposition_but_hook_misassociation_changes_hash() {
        let mut r = fixture();
        park(&mut r);
        let old = r.game.cost_payment_receipts[0].id;
        let original = r.game.verification_digest();
        let mut renamed = r.game.clone();
        rewrite_ids(&mut renamed, old, 901);
        assert_eq!(original, renamed.verification_digest());
        let mut wrong = r.game.clone();
        for hook in &mut wrong.after_selection_resume_hooks.0 {
            if let AfterSelectionHook::PlayCostContinuation {
                accumulated_reduction,
                ..
            } = hook
            {
                accumulated_reduction.pending_payment = Some(999);
            }
        }
        assert_ne!(
            original,
            wrong.verification_digest(),
            "different payment association must not collide"
        );
    }
    #[test]
    fn success_marker_reference_is_hashed_without_historical_token_identity() {
        let mut r = fixture();
        park(&mut r);
        let original = r.game.verification_digest();
        let mut wrong = r.game.clone();
        let mut changed = false;
        for frame in &mut wrong.pending_selection_resume.as_mut().unwrap().frames {
            if let ResumeFrame::RunTail { inner_tail, .. } = frame {
                for step in std::sync::Arc::make_mut(inner_tail) {
                    if let CompiledStep::CompleteCostPayment { receipt } = step {
                        *receipt = 999;
                        changed = true;
                    }
                }
            }
        }
        assert!(changed);
        assert_ne!(
            original,
            wrong.verification_digest(),
            "same tail length with a different completion receipt is distinct"
        );
    }
    #[test]
    fn cost_assignment_prompt_identifies_setting_the_base_instead_of_reduction() {
        let mut r = fixture();
        r.game.decode_action(PLAY_HAND_START, 0);
        let p = r.game.pending_selection.as_ref().unwrap();
        assert!(
            p.prompt.to_lowercase().contains("set play cost"),
            "{}",
            p.prompt
        );
        let row = &p.effect_choices.as_ref().unwrap()[0];
        assert!(row.label.contains('3'));
        assert!(!row.label.contains("(-0)"));
        assert!(!p.prompt.to_lowercase().contains("reduce"));
    }

    #[test]
    fn dangling_active_receipt_with_empty_table_is_hashed() {
        let r = fixture();
        assert!(r.game.cost_payment_receipts.is_empty());
        let original = r.game.verification_digest();
        let mut wrong = r.game.clone();
        wrong.active_cost_payment_receipt = Some(998);
        assert_ne!(
            original,
            wrong.verification_digest(),
            "dangling active receipt must remain observable with no table"
        );
        let mut another = wrong.clone();
        another.active_cost_payment_receipt = Some(999);
        assert_ne!(wrong.verification_digest(), another.verification_digest());
    }
    #[test]
    fn dangling_success_marker_with_empty_table_is_hashed() {
        let mut r = fixture();
        park(&mut r);
        r.game.cost_payment_receipts.clear();
        let original = r.game.verification_digest();
        let mut wrong = r.game.clone();
        let mut changed = false;
        for frame in &mut wrong.pending_selection_resume.as_mut().unwrap().frames {
            if let ResumeFrame::RunTail { inner_tail, .. } = frame {
                for step in std::sync::Arc::make_mut(inner_tail) {
                    if let CompiledStep::CompleteCostPayment { receipt } = step {
                        *receipt = 999;
                        changed = true;
                    }
                }
            }
        }
        assert!(changed);
        assert_ne!(
            original,
            wrong.verification_digest(),
            "empty table does not erase a dangling completion reference"
        );
    }
    #[test]
    fn dangling_pending_payment_with_empty_table_is_hashed() {
        let mut r = fixture();
        park(&mut r);
        r.game.cost_payment_receipts.clear();
        let original = r.game.verification_digest();
        let mut wrong = r.game.clone();
        let mut changed = false;
        for hook in &mut wrong.after_selection_resume_hooks.0 {
            if let AfterSelectionHook::PlayCostContinuation {
                accumulated_reduction,
                ..
            } = hook
            {
                accumulated_reduction.pending_payment = Some(999);
                changed = true;
            }
        }
        assert!(changed);
        assert_ne!(
            original,
            wrong.verification_digest(),
            "empty table does not erase a dangling payment continuation"
        );
    }

    fn with_effect_scope_receipt() -> DebugRunner {
        use crate::{game_actions::digivolve::{EffectDigivolveCostChoiceState,EffectDigivolveScope,DigivolveRequirementWaiver},enums::{CardSourceRef,CostDelta,PlaySource},permanent::PermanentHandle};
        let mut r=fixture();park(&mut r);
        let frame=r.game.pending_selection_resume.as_ref().unwrap().frames.last().unwrap();
        let ResumeFrame::RunTail{prov,inner_tail,bindings,runtime,trigger_context,..}=frame else {panic!("real cost body must be parked")};
        let cont=crate::resume::OuterContinuation{tail:inner_tail.clone(),bindings:bindings.clone(),runtime:runtime.clone(),trigger_context:trigger_context.clone(),source_card:prov.source_card,source_permanent:prov.source_permanent,player:prov.controller};
        let card=prov.source_card;let host=PermanentHandle{player:0,index:0};let top=r.game.players[0].battle_area[0].top_card().handle();
        let identity=r.game.capture_digivolve_host_identity(host);
        r.game.effect_digivolve_scopes.push(EffectDigivolveScope{
            declaration:EffectDigivolveCostChoiceState{acting_player:0,source_ref:CardSourceRef::Hand(0,0),result_card:card,source_carrier_top:None,source_carrier_identity:None,target_player:0,target_zone:crate::enums::Zone::BattleArea,target_top:top,target_identity:identity,dna_origin:None,cost_delta:CostDelta::Free,waiver:DigivolveRequirementWaiver::None,source:PlaySource::ByEffect,printed_costs:vec![],effect_source_player:Some(0),trigger_context:None,result_clause:None,outer_conts:vec![cont]},
            identity:crate::game::PendingWouldDigivolveResume{window_started:true,host_identity:identity,treated_identity_orders:None,player:0,permanent:host,card,effective_cost:0,app_fusion:false}});
        r
    }
    fn rewrite_scope_marker(g:&mut Game,new:u64){
        let mut changed=false;
        for c in &mut g.effect_digivolve_scopes[0].declaration.outer_conts {
            for step in std::sync::Arc::make_mut(&mut c.tail) {
                if let CompiledStep::CompleteCostPayment{receipt}=step {*receipt=new;changed=true;}
            }
        }
        assert!(changed);
    }
    #[test]
    fn effect_scope_receipt_alpha_renaming_preserves_digest(){
        let r=with_effect_scope_receipt();let mut renamed=r.game.clone();let old=renamed.cost_payment_receipts[0].id;
        rewrite_ids(&mut renamed,old,901);rewrite_scope_marker(&mut renamed,901);
        assert_eq!(r.game.verification_digest(),renamed.verification_digest(),"effect-owned tail receipt IDs must be canonical references, not allocation history");
    }
    #[test]
    fn effect_scope_wrong_receipt_reference_changes_digest(){
        let r=with_effect_scope_receipt();let mut wrong=r.game.clone();rewrite_scope_marker(&mut wrong,999);
        assert_ne!(r.game.verification_digest(),wrong.verification_digest());
    }
    #[test]
    fn effect_scope_dangling_receipt_stays_distinct_with_empty_table(){
        let mut r=with_effect_scope_receipt();r.game.cost_payment_receipts.clear();let mut wrong=r.game.clone();rewrite_scope_marker(&mut wrong,999);
        assert_ne!(r.game.verification_digest(),wrong.verification_digest());
    }
    #[test]
    fn effect_scope_nested_marker_alpha_renaming_preserves_digest() {
        let mut r = with_effect_scope_receipt();
        let tail = &mut r.game.effect_digivolve_scopes[0].declaration.outer_conts[0].tail;
        let nested = tail.to_vec();
        *tail = std::sync::Arc::from(vec![CompiledStep::AsSelectingPlayer {
            of: digimon_dsl::compiled::CompiledPlayerRef::You, body: nested,
        }]);
        let mut renamed = r.game.clone();
        let old = renamed.cost_payment_receipts[0].id;
        rewrite_ids(&mut renamed, old, 901);
        for cont in &mut renamed.effect_digivolve_scopes[0].declaration.outer_conts {
            for step in std::sync::Arc::make_mut(&mut cont.tail) {
                if let CompiledStep::AsSelectingPlayer { body, .. } = step {
                    for nested in body {
                        if let CompiledStep::CompleteCostPayment { receipt } = nested { *receipt = 901; }
                    }
                }
            }
        }
        assert_eq!(r.game.verification_digest(), renamed.verification_digest());
    }
    #[test]
    fn two_effect_scopes_preserve_alpha_renaming_and_reference_positions() {
        let mut r = with_effect_scope_receipt();
        let second = r.game.effect_digivolve_scopes[0].clone();
        r.game.effect_digivolve_scopes.push(second);
        let mut renamed = r.game.clone();
        let old = renamed.cost_payment_receipts[0].id;
        rewrite_ids(&mut renamed, old, 901);
        for scope in &mut renamed.effect_digivolve_scopes {
            for cont in &mut scope.declaration.outer_conts {
                for step in std::sync::Arc::make_mut(&mut cont.tail) {
                    if let CompiledStep::CompleteCostPayment { receipt } = step { *receipt = 901; }
                }
            }
        }
        assert_eq!(r.game.verification_digest(), renamed.verification_digest());
        for step in std::sync::Arc::make_mut(&mut renamed.effect_digivolve_scopes[1].declaration.outer_conts[0].tail) {
            if let CompiledStep::CompleteCostPayment { receipt } = step { *receipt = 999; }
        }
        assert_ne!(r.game.verification_digest(), renamed.verification_digest());
    }
    #[test]
    fn effect_scope_nonreceipt_declaration_changes_remain_hashed() {
        let r = with_effect_scope_receipt();
        let mut changed = r.game.clone();
        changed.effect_digivolve_scopes[0].declaration.dna_origin = Some(true);
        assert_ne!(r.game.verification_digest(), changed.verification_digest());
        let mut changed = r.game.clone();
        changed.effect_digivolve_scopes[0].declaration.outer_conts[0].tail = std::sync::Arc::from(vec![CompiledStep::GainMemory(42)]);
        assert_ne!(r.game.verification_digest(), changed.verification_digest());
    }

    #[test]
    fn pending_play_credit_changes_current_digest_but_not_archival_protocols() {
        use crate::recorder::VerificationDigestVersion;
        let mut r = fixture();
        park(&mut r);
        let mut branch = r.game.clone();
        let mut changed = false;
        for hook in &mut branch.after_selection_resume_hooks.0 {
            if let AfterSelectionHook::PlayCostContinuation { accumulated_reduction, .. } = hook {
                accumulated_reduction.pending_reduction = Some(7);
                changed = true;
            }
        }
        assert!(changed, "fixture must park a real play-cost continuation");
        for version in [VerificationDigestVersion::LegacyV2, VerificationDigestVersion::V3] {
            assert_eq!(r.game.verification_digest_with_version(version),
                branch.verification_digest_with_version(version), "archival bytes exclude the new credit field");
        }
        assert_ne!(r.game.verification_digest(), branch.verification_digest(),
            "different pending credit can change cost rollback and must be hashed");
        assert_eq!(branch.verification_digest(), branch.clone().verification_digest());
    }

    #[test]
    fn pending_play_order_credit_is_hashed_through_nested_resume_wrappers() {
        use crate::recorder::VerificationDigestVersion;
        for nested in [false, true] {
            let mut r = fixture();
            r.game.decode_action(PLAY_HAND_START, 0);
            assert!(r.game.pending_selection.is_some());
            if nested {
                let inner = r.game.pending_selection_resume.take().unwrap();
                r.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                    frames: vec![ResumeFrame::ReplacementBody { inner: Box::new(inner), outer_conts: Vec::new() }],
                });
            }
            let mut branch = r.game.clone();
            fn change(stack: &mut crate::resume::ResumeStack) -> bool {
                for frame in &mut stack.frames {
                    match frame {
                        ResumeFrame::PlayFromHandCostReductionPrompt(state) => {
                            state.accumulated_reduction.pending_reduction = Some(5);
                            return true;
                        }
                        ResumeFrame::ReplacementBody { inner, .. } => { if change(inner) { return true; } }
                        _ => {}
                    }
                }
                false
            }
            assert!(change(branch.pending_selection_resume.as_mut().unwrap()), "real play-order frame required");
            for version in [VerificationDigestVersion::LegacyV2, VerificationDigestVersion::V3] {
                assert_eq!(r.game.verification_digest_with_version(version), branch.verification_digest_with_version(version));
            }
            assert_ne!(r.game.verification_digest(), branch.verification_digest());
            assert_eq!(branch.verification_digest(), branch.clone().verification_digest());
        }
    }

}

#[cfg(test)]
#[path = "verification_reducer_hooks_tests.rs"]
mod reducer_hook_digest_tests;
