use sha2::{Digest, Sha256};

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
        let mut h = Sha256::new();
        token(&mut h, "digimon-engine.verification-digest.v1");

        field(&mut h, "rules.player_count", self.rules.player_count);
        field(&mut h, "rules.security_count", self.rules.security_count);
        field(&mut h, "rules.field_slots", self.rules.field_slots);
        field(&mut h, "turn_count", self.turn_count);
        debug_field(&mut h, "current_phase", &self.current_phase);
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
                if let Some(choices) = &selection.effect_choices {
                    for choice in choices {
                        token(&mut h, "effect_choice");
                        field(&mut h, "effect_choice.action_id", choice.action_id);
                        field(&mut h, "effect_choice.label", &choice.label);
                        debug_field(&mut h, "effect_choice.source_card", &choice.source_card);
                        debug_field(&mut h, "effect_choice.source_kind", &choice.source_kind);
                        debug_field(&mut h, "effect_choice.timing", &choice.timing);
                        field(&mut h, "effect_choice.is_optional", choice.is_optional);
                    }
                }
            }
            None => token(&mut h, "pending_selection=none"),
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
        debug_field(
            &mut h,
            "pending_overclock_attack",
            &self.pending_overclock_attack,
        );
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
