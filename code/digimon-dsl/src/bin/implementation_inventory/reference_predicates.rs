// Explicit typed patterns: all fields and variants, no rest/wildcard patterns.
use super::{Collector, Walk};
impl Walk for digimon_dsl::predicate::PredicateSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::PredicateSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::PredicateSpec {
            kind,
            level_eq,
            level_eq_binding,
            level_lte_binding,
            level_gte_binding,
            level_lte,
            level_gte,
            level_matches_aggregate,
            materials_count_matches_aggregate,
            color_is,
            color_only,
            color_matches_any_field_digimon,
            color_matches_binding,
            color_matches_returned_card,
            color_matches_own_source_stack,
            trait_has,
            trait_contains,
            form_is,
            attribute_is,
            name_is,
            name_contains,
            effect_text_contains,
            printed_keyword,
            in_text_contains,
            name_in,
            name_not_shared_by_field_digimon,
            name_not_shared_by_field_tamer,
            card_number_is,
            play_cost_lte,
            play_cost_gte,
            play_or_use_cost_lte,
            affordable_with_cost_reduce,
            can_digivolve_from_source,
            can_digivolve_onto,
            can_digivolve_onto_with_cost,
            dna_hand_candidate,
            can_move_from_breeding_by_effect,
            dp_eq,
            dp_lte,
            dp_gte,
            stack_size_lte,
            stack_size_gte,
            materials_count_lte,
            materials_count_gte,
            source_count,
            link_card_count,
            has_digivolve_candidate,
            has_inherited,
            is_suspended,
            is_unsuspended,
            can_change_orientation,
            controller_can_add_security,
            can_attack,
            has_keyword,
            has_security_attack_change,
            has_on_deletion_effect,
            self_color_count_gte,
            self_color_count_lte,
            has_face_down_source,
            distinct_tamer_colors_gte,
            distinct_named_count_gte,
            distinct_colors_gte,
            face_down_sources_under_tamers_gte,
            battle_opponent_no_sources,
            self_source_count,
            self_same_level_source_pairs_gte,
            own_source_stack_color_count_gte,
            zone,
            owner,
            other,
            is_source,
            of_permanent,
            not_in_binding,
            name_not_in_binding,
            binding_owner,
            binding_card_kind,
            binding_card_color,
            binding_card_name_is,
            play_cost_eq_binding,
            source_is_tamer,
            source_is_unsuspended,
            is_source_permanent,
            source_name_contains,
            source_permanent_trait_has,
            self_digivolution_contains_name,
            is_face_down,
            is_bottom_source,
            host_kind_is,
            self_digivolution_sources_contain_name,
            self_digivolution_sources_trait_has,
            rules_text_contains,
            memory_lte,
            memory_gte,
            own_memory_lte,
            own_memory_gte,
            security_count_lte,
            security_count_gte,
            opponent_security_count_lte,
            opponent_security_count_gte,
            total_security_count_lte,
            total_security_count_gte,
            total_security_count_eq,
            face_up_security_count_lte,
            face_up_security_count_gte,
            no_face_up_security_named,
            your_turn,
            opponents_turn,
            all_turns,
            can_hatch,
            digimon_attacked_this_turn,
            during_attack,
            can_gain_memory,
            in_breeding,
            on_field,
            dna_origin,
            event_target_kind,
            event_target_trait_has,
            event_target_trait_contains,
            event_target_has_digivolution_cards,
            event_target_stack_size_gte,
            event_target_level_eq,
            event_target_level_lte,
            event_target_level_gte,
            event_target_dp_eq,
            event_target_dp_lte,
            event_target_dp_gte,
            event_target_name_contains,
            event_target_in_text_contains,
            event_target_has_inherited,
            event_target_owner,
            event_target_color_any_of,
            event_permanent_is_source,
            source_deleted_battle_opponent,
            event_host_permanent_is_source,
            event_host_is_own_tamer,
            event_host_is_own_digimon,
            is_event_host,
            event_is_effect_initiated,
            event_add_to_hand_player,
            event_winner_owner,
            event_winner_trait_has,
            event_winner_is_source,
            event_battle_deleter,
            event_discard_player,
            event_caused_by_own_effect,
            event_added_card_any,
            event_discarded_card_any,
            in_event_discarded_cards,
            played_by_effect,
            event_target_is_player,
            event_target_is_source,
            event_target_was_self,
            attack_target_change_reason,
            attacker_trait_has,
            event_card_trait_has,
            event_card_name_contains,
            event_card_text_contains,
            event_card_in_text_contains,
            event_card_level_eq,
            event_card_level_gte,
            event_card_color_only,
            event_card_color_has,
            event_card_color_count,
            event_cause,
            event_target_same_level_as_previous,
            host_permanent_trait_has,
            trashed_source_trait_has,
            trashed_source_card_id_is,
            replacement_cause,
            replacement_source_is_opponent,
            replacement_subject_is_mine,
            would_link_card_trait_any_of,
            equals,
            not_equals,
            binding_exists,
            binding_present,
            binding_absent,
            binding_count_eq,
            effect_suspended_any_own_digimon,
            effect_suspended_any_opponent_digimon,
            effect_returned_any_card,
            returned_card_matching,
            effect_trashed_any_hand_card,
            trashed_hand_card_matching,
            effect_deleted_any_own_digimon,
            effect_deleted_any_opponent_digimon,
            effect_deleted_opponent_digimon_dp_gte,
            effect_played_any_digimon,
            effect_digivolved_any_digimon,
            effect_added_any_card_to_hand,
            count_lte,
            count_gte,
            level_sum_gte,
            any_permanent,
            any_field_permanent,
            no_permanent,
            all_permanents,
            all_of,
            any_of,
            none_of,
            not,
            has_alt_path,
            cost_target,
            cost_target_permanent,
            source_is_cost_target_permanent,
            cost_target_from_hand,
            dp,
            event_target_dp,
            play_cost,
            stack_size,
            materials_count,
            security_count,
            level,
            event_target_level,
            extra,
            can_play_from_effect_without_cost,
            can_play_to_breeding_from_hand,
            can_unsuspend,
            owner_can_add_security,
            source_name_is,
            event_battle_surviving_deleter,
            event_card_name_is,
            event_card_kind,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level_eq.walk(c, &format!("{}.level_eq", path), "UNCLASSIFIED_STRING");
        level_eq_binding.walk(
            c,
            &format!("{}.level_eq_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_lte_binding.walk(
            c,
            &format!("{}.level_lte_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_gte_binding.walk(
            c,
            &format!("{}.level_gte_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_lte.walk(c, &format!("{}.level_lte", path), "UNCLASSIFIED_STRING");
        level_gte.walk(c, &format!("{}.level_gte", path), "UNCLASSIFIED_STRING");
        level_matches_aggregate.walk(
            c,
            &format!("{}.level_matches_aggregate", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_matches_aggregate.walk(
            c,
            &format!("{}.materials_count_matches_aggregate", path),
            "UNCLASSIFIED_STRING",
        );
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
        color_only.walk(c, &format!("{}.color_only", path), "UNCLASSIFIED_STRING");
        color_matches_any_field_digimon.walk(
            c,
            &format!("{}.color_matches_any_field_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        color_matches_binding.walk(
            c,
            &format!("{}.color_matches_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        color_matches_returned_card.walk(
            c,
            &format!("{}.color_matches_returned_card", path),
            "UNCLASSIFIED_STRING",
        );
        color_matches_own_source_stack.walk(
            c,
            &format!("{}.color_matches_own_source_stack", path),
            "UNCLASSIFIED_STRING",
        );
        trait_has.walk(c, &format!("{}.trait_has", path), "MATCH_OR_DEFINE_TRAIT");
        trait_contains.walk(
            c,
            &format!("{}.trait_contains", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        form_is.walk(c, &format!("{}.form_is", path), "MATCH_OR_DEFINE_TRAIT");
        attribute_is.walk(
            c,
            &format!("{}.attribute_is", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        name_is.walk(c, &format!("{}.name_is", path), "MATCH_NAME_EXACT");
        name_contains.walk(
            c,
            &format!("{}.name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        effect_text_contains.walk(c, &format!("{}.effect_text_contains", path), "MATCH_TEXT");
        printed_keyword.walk(
            c,
            &format!("{}.printed_keyword", path),
            "UNCLASSIFIED_STRING",
        );
        in_text_contains.walk(c, &format!("{}.in_text_contains", path), "MATCH_TEXT");
        name_in.walk(c, &format!("{}.name_in", path), "MATCH_NAME_EXACT");
        name_not_shared_by_field_digimon.walk(
            c,
            &format!("{}.name_not_shared_by_field_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        name_not_shared_by_field_tamer.walk(
            c,
            &format!("{}.name_not_shared_by_field_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        card_number_is.walk(c, &format!("{}.card_number_is", path), "MATCH_CARD_ID");
        play_cost_lte.walk(c, &format!("{}.play_cost_lte", path), "UNCLASSIFIED_STRING");
        play_cost_gte.walk(c, &format!("{}.play_cost_gte", path), "UNCLASSIFIED_STRING");
        play_or_use_cost_lte.walk(
            c,
            &format!("{}.play_or_use_cost_lte", path),
            "UNCLASSIFIED_STRING",
        );
        affordable_with_cost_reduce.walk(
            c,
            &format!("{}.affordable_with_cost_reduce", path),
            "UNCLASSIFIED_STRING",
        );
        can_digivolve_from_source.walk(
            c,
            &format!("{}.can_digivolve_from_source", path),
            "UNCLASSIFIED_STRING",
        );
        can_digivolve_onto.walk(
            c,
            &format!("{}.can_digivolve_onto", path),
            "EXISTING_INSTANCE_BINDING",
        );
        can_digivolve_onto_with_cost.walk(c, &format!("{}.can_digivolve_onto_with_cost", path), role);
        dna_hand_candidate.walk(c, &format!("{}.dna_hand_candidate", path), role);
        can_move_from_breeding_by_effect.walk(c, &format!("{}.can_move_from_breeding_by_effect", path), role);
        dp_eq.walk(c, &format!("{}.dp_eq", path), "UNCLASSIFIED_STRING");
        dp_lte.walk(c, &format!("{}.dp_lte", path), "UNCLASSIFIED_STRING");
        dp_gte.walk(c, &format!("{}.dp_gte", path), "UNCLASSIFIED_STRING");
        stack_size_lte.walk(
            c,
            &format!("{}.stack_size_lte", path),
            "UNCLASSIFIED_STRING",
        );
        stack_size_gte.walk(
            c,
            &format!("{}.stack_size_gte", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_lte.walk(
            c,
            &format!("{}.materials_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_gte.walk(
            c,
            &format!("{}.materials_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        source_count.walk(c, &format!("{}.source_count", path), "UNCLASSIFIED_STRING");
        link_card_count.walk(
            c,
            &format!("{}.link_card_count", path),
            "UNCLASSIFIED_STRING",
        );
        has_digivolve_candidate.walk(
            c,
            &format!("{}.has_digivolve_candidate", path),
            "UNCLASSIFIED_STRING",
        );
        has_inherited.walk(c, &format!("{}.has_inherited", path), "UNCLASSIFIED_STRING");
        is_suspended.walk(c, &format!("{}.is_suspended", path), "UNCLASSIFIED_STRING");
        is_unsuspended.walk(
            c,
            &format!("{}.is_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        can_change_orientation.walk(
            c,
            &format!("{}.can_change_orientation", path),
            "UNCLASSIFIED_STRING",
        );
        can_attack.walk(c, &format!("{}.can_attack", path), "UNCLASSIFIED_STRING");
        controller_can_add_security.walk(c, &format!("{}.controller_can_add_security", path), "UNCLASSIFIED_STRING");
        has_keyword.walk(c, &format!("{}.has_keyword", path), "UNCLASSIFIED_STRING");
        has_security_attack_change.walk(
            c,
            &format!("{}.has_security_attack_change", path),
            "UNCLASSIFIED_STRING",
        );
        has_on_deletion_effect.walk(
            c,
            &format!("{}.has_on_deletion_effect", path),
            "UNCLASSIFIED_STRING",
        );
        self_color_count_gte.walk(
            c,
            &format!("{}.self_color_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        self_color_count_lte.walk(
            c,
            &format!("{}.self_color_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        has_face_down_source.walk(
            c,
            &format!("{}.has_face_down_source", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_tamer_colors_gte.walk(
            c,
            &format!("{}.distinct_tamer_colors_gte", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_named_count_gte.walk(
            c,
            &format!("{}.distinct_named_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_colors_gte.walk(
            c,
            &format!("{}.distinct_colors_gte", path),
            "UNCLASSIFIED_STRING",
        );
        face_down_sources_under_tamers_gte.walk(
            c,
            &format!("{}.face_down_sources_under_tamers_gte", path),
            "UNCLASSIFIED_STRING",
        );
        battle_opponent_no_sources.walk(
            c,
            &format!("{}.battle_opponent_no_sources", path),
            "UNCLASSIFIED_STRING",
        );
        self_source_count.walk(
            c,
            &format!("{}.self_source_count", path),
            "UNCLASSIFIED_STRING",
        );
        self_same_level_source_pairs_gte.walk(
            c,
            &format!("{}.self_same_level_source_pairs_gte", path),
            "UNCLASSIFIED_STRING",
        );
        own_source_stack_color_count_gte.walk(
            c,
            &format!("{}.own_source_stack_color_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        owner.walk(c, &format!("{}.owner", path), "UNCLASSIFIED_STRING");
        other.walk(c, &format!("{}.other", path), "UNCLASSIFIED_STRING");
        is_source.walk(c, &format!("{}.is_source", path), "UNCLASSIFIED_STRING");
        of_permanent.walk(
            c,
            &format!("{}.of_permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        not_in_binding.walk(
            c,
            &format!("{}.not_in_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        name_not_in_binding.walk(
            c,
            &format!("{}.name_not_in_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_owner.walk(c, &format!("{}.binding_owner", path), "UNCLASSIFIED_STRING");
        binding_card_kind.walk(
            c,
            &format!("{}.binding_card_kind", path),
            "UNCLASSIFIED_STRING",
        );
        binding_card_color.walk(
            c,
            &format!("{}.binding_card_color", path),
            "UNCLASSIFIED_STRING",
        );
        binding_card_name_is.walk(
            c,
            &format!("{}.binding_card_name_is", path),
            "MATCH_NAME_EXACT",
        );
        play_cost_eq_binding.walk(
            c,
            &format!("{}.play_cost_eq_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        source_is_tamer.walk(
            c,
            &format!("{}.source_is_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        source_is_unsuspended.walk(
            c,
            &format!("{}.source_is_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        is_source_permanent.walk(
            c,
            &format!("{}.is_source_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        source_name_contains.walk(
            c,
            &format!("{}.source_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        source_permanent_trait_has.walk(
            c,
            &format!("{}.source_permanent_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        self_digivolution_contains_name.walk(
            c,
            &format!("{}.self_digivolution_contains_name", path),
            "UNCLASSIFIED_STRING",
        );
        is_face_down.walk(c, &format!("{}.is_face_down", path), "UNCLASSIFIED_STRING");
        is_bottom_source.walk(
            c,
            &format!("{}.is_bottom_source", path),
            "UNCLASSIFIED_STRING",
        );
        host_kind_is.walk(c, &format!("{}.host_kind_is", path), "UNCLASSIFIED_STRING");
        self_digivolution_sources_contain_name.walk(
            c,
            &format!("{}.self_digivolution_sources_contain_name", path),
            "UNCLASSIFIED_STRING",
        );
        self_digivolution_sources_trait_has.walk(
            c,
            &format!("{}.self_digivolution_sources_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        rules_text_contains.walk(
            c,
            &format!("{}.rules_text_contains", path),
            "UNCLASSIFIED_STRING",
        );
        memory_lte.walk(c, &format!("{}.memory_lte", path), "UNCLASSIFIED_STRING");
        memory_gte.walk(c, &format!("{}.memory_gte", path), "UNCLASSIFIED_STRING");
        own_memory_lte.walk(
            c,
            &format!("{}.own_memory_lte", path),
            "UNCLASSIFIED_STRING",
        );
        own_memory_gte.walk(
            c,
            &format!("{}.own_memory_gte", path),
            "UNCLASSIFIED_STRING",
        );
        security_count_lte.walk(
            c,
            &format!("{}.security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        security_count_gte.walk(
            c,
            &format!("{}.security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        opponent_security_count_lte.walk(
            c,
            &format!("{}.opponent_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        opponent_security_count_gte.walk(
            c,
            &format!("{}.opponent_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_lte.walk(
            c,
            &format!("{}.total_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_gte.walk(
            c,
            &format!("{}.total_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_eq.walk(
            c,
            &format!("{}.total_security_count_eq", path),
            "UNCLASSIFIED_STRING",
        );
        face_up_security_count_lte.walk(
            c,
            &format!("{}.face_up_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        face_up_security_count_gte.walk(
            c,
            &format!("{}.face_up_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        no_face_up_security_named.walk(
            c,
            &format!("{}.no_face_up_security_named", path),
            "UNCLASSIFIED_STRING",
        );
        your_turn.walk(c, &format!("{}.your_turn", path), "UNCLASSIFIED_STRING");
        opponents_turn.walk(
            c,
            &format!("{}.opponents_turn", path),
            "UNCLASSIFIED_STRING",
        );
        all_turns.walk(c, &format!("{}.all_turns", path), "UNCLASSIFIED_STRING");
        can_hatch.walk(c, &format!("{}.can_hatch", path), "UNCLASSIFIED_STRING");
        digimon_attacked_this_turn.walk(
            c,
            &format!("{}.digimon_attacked_this_turn", path),
            "UNCLASSIFIED_STRING",
        );
        during_attack.walk(c, &format!("{}.during_attack", path), "UNCLASSIFIED_STRING");
        can_gain_memory.walk(c, &format!("{}.can_gain_memory", path), "UNCLASSIFIED_STRING");
        in_breeding.walk(c, &format!("{}.in_breeding", path), "UNCLASSIFIED_STRING");
        on_field.walk(c, &format!("{}.on_field", path), "UNCLASSIFIED_STRING");
        dna_origin.walk(c, &format!("{}.dna_origin", path), "UNCLASSIFIED_STRING");
        event_target_kind.walk(
            c,
            &format!("{}.event_target_kind", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_trait_has.walk(
            c,
            &format!("{}.event_target_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_target_trait_contains.walk(
            c,
            &format!("{}.event_target_trait_contains", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_target_has_digivolution_cards.walk(
            c,
            &format!("{}.event_target_has_digivolution_cards", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_stack_size_gte.walk(
            c,
            &format!("{}.event_target_stack_size_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_eq.walk(
            c,
            &format!("{}.event_target_level_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_lte.walk(
            c,
            &format!("{}.event_target_level_lte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_gte.walk(
            c,
            &format!("{}.event_target_level_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_eq.walk(
            c,
            &format!("{}.event_target_dp_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_lte.walk(
            c,
            &format!("{}.event_target_dp_lte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_gte.walk(
            c,
            &format!("{}.event_target_dp_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_name_contains.walk(
            c,
            &format!("{}.event_target_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        event_target_in_text_contains.walk(
            c,
            &format!("{}.event_target_in_text_contains", path),
            "MATCH_TEXT",
        );
        event_target_has_inherited.walk(
            c,
            &format!("{}.event_target_has_inherited", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_owner.walk(
            c,
            &format!("{}.event_target_owner", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_color_any_of.walk(
            c,
            &format!("{}.event_target_color_any_of", path),
            "UNCLASSIFIED_STRING",
        );
        event_permanent_is_source.walk(
            c,
            &format!("{}.event_permanent_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        source_deleted_battle_opponent.walk(
            c,
            &format!("{}.source_deleted_battle_opponent", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_permanent_is_source.walk(
            c,
            &format!("{}.event_host_permanent_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_is_own_tamer.walk(
            c,
            &format!("{}.event_host_is_own_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_is_own_digimon.walk(
            c,
            &format!("{}.event_host_is_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        is_event_host.walk(c, &format!("{}.is_event_host", path), "UNCLASSIFIED_STRING");
        event_is_effect_initiated.walk(
            c,
            &format!("{}.event_is_effect_initiated", path),
            "UNCLASSIFIED_STRING",
        );
        event_add_to_hand_player.walk(
            c,
            &format!("{}.event_add_to_hand_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_winner_owner.walk(
            c,
            &format!("{}.event_winner_owner", path),
            "UNCLASSIFIED_STRING",
        );
        event_winner_trait_has.walk(
            c,
            &format!("{}.event_winner_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_winner_is_source.walk(
            c,
            &format!("{}.event_winner_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_battle_deleter.walk(
            c,
            &format!("{}.event_battle_deleter", path),
            "UNCLASSIFIED_STRING",
        );
        event_discard_player.walk(
            c,
            &format!("{}.event_discard_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_caused_by_own_effect.walk(
            c,
            &format!("{}.event_caused_by_own_effect", path),
            "UNCLASSIFIED_STRING",
        );
        event_added_card_any.walk(
            c,
            &format!("{}.event_added_card_any", path),
            "UNCLASSIFIED_STRING",
        );
        event_discarded_card_any.walk(
            c,
            &format!("{}.event_discarded_card_any", path),
            "UNCLASSIFIED_STRING",
        );
        in_event_discarded_cards.walk(
            c,
            &format!("{}.in_event_discarded_cards", path),
            "UNCLASSIFIED_STRING",
        );
        played_by_effect.walk(
            c,
            &format!("{}.played_by_effect", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_is_player.walk(
            c,
            &format!("{}.event_target_is_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_is_source.walk(
            c,
            &format!("{}.event_target_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_was_self.walk(
            c,
            &format!("{}.event_target_was_self", path),
            "UNCLASSIFIED_STRING",
        );
        attack_target_change_reason.walk(
            c,
            &format!("{}.attack_target_change_reason", path),
            "UNCLASSIFIED_STRING",
        );
        attacker_trait_has.walk(
            c,
            &format!("{}.attacker_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_card_trait_has.walk(
            c,
            &format!("{}.event_card_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_card_name_contains.walk(
            c,
            &format!("{}.event_card_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        event_card_text_contains.walk(
            c,
            &format!("{}.event_card_text_contains", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_in_text_contains.walk(
            c,
            &format!("{}.event_card_in_text_contains", path),
            "MATCH_TEXT",
        );
        event_card_level_eq.walk(
            c,
            &format!("{}.event_card_level_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_level_gte.walk(
            c,
            &format!("{}.event_card_level_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_only.walk(
            c,
            &format!("{}.event_card_color_only", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_has.walk(
            c,
            &format!("{}.event_card_color_has", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_count.walk(
            c,
            &format!("{}.event_card_color_count", path),
            "UNCLASSIFIED_STRING",
        );
        event_cause.walk(c, &format!("{}.event_cause", path), "UNCLASSIFIED_STRING");
        event_target_same_level_as_previous.walk(
            c,
            &format!("{}.event_target_same_level_as_previous", path),
            "UNCLASSIFIED_STRING",
        );
        host_permanent_trait_has.walk(
            c,
            &format!("{}.host_permanent_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        trashed_source_trait_has.walk(
            c,
            &format!("{}.trashed_source_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        trashed_source_card_id_is.walk(
            c,
            &format!("{}.trashed_source_card_id_is", path),
            "MATCH_CARD_ID",
        );
        replacement_cause.walk(
            c,
            &format!("{}.replacement_cause", path),
            "UNCLASSIFIED_STRING",
        );
        replacement_source_is_opponent.walk(
            c,
            &format!("{}.replacement_source_is_opponent", path),
            "UNCLASSIFIED_STRING",
        );
        replacement_subject_is_mine.walk(
            c,
            &format!("{}.replacement_subject_is_mine", path),
            "UNCLASSIFIED_STRING",
        );
        would_link_card_trait_any_of.walk(
            c,
            &format!("{}.would_link_card_trait_any_of", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        equals.walk(c, &format!("{}.equals", path), "UNCLASSIFIED_STRING");
        not_equals.walk(c, &format!("{}.not_equals", path), "UNCLASSIFIED_STRING");
        binding_exists.walk(
            c,
            &format!("{}.binding_exists", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_present.walk(
            c,
            &format!("{}.binding_present", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_absent.walk(
            c,
            &format!("{}.binding_absent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_count_eq.walk(
            c,
            &format!("{}.binding_count_eq", path),
            "EXISTING_INSTANCE_BINDING",
        );
        effect_suspended_any_own_digimon.walk(
            c,
            &format!("{}.effect_suspended_any_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_suspended_any_opponent_digimon.walk(
            c,
            &format!("{}.effect_suspended_any_opponent_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_returned_any_card.walk(
            c,
            &format!("{}.effect_returned_any_card", path),
            "UNCLASSIFIED_STRING",
        );
        returned_card_matching.walk(
            c,
            &format!("{}.returned_card_matching", path),
            "UNCLASSIFIED_STRING",
        );
        effect_trashed_any_hand_card.walk(
            c,
            &format!("{}.effect_trashed_any_hand_card", path),
            "UNCLASSIFIED_STRING",
        );
        trashed_hand_card_matching.walk(
            c,
            &format!("{}.trashed_hand_card_matching", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_any_own_digimon.walk(
            c,
            &format!("{}.effect_deleted_any_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_any_opponent_digimon.walk(
            c,
            &format!("{}.effect_deleted_any_opponent_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_opponent_digimon_dp_gte.walk(
            c,
            &format!("{}.effect_deleted_opponent_digimon_dp_gte", path),
            "UNCLASSIFIED_STRING",
        );
        effect_played_any_digimon.walk(
            c,
            &format!("{}.effect_played_any_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_digivolved_any_digimon.walk(
            c,
            &format!("{}.effect_digivolved_any_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_added_any_card_to_hand.walk(
            c,
            &format!("{}.effect_added_any_card_to_hand", path),
            "UNCLASSIFIED_STRING",
        );
        count_lte.walk(c, &format!("{}.count_lte", path), "UNCLASSIFIED_STRING");
        count_gte.walk(c, &format!("{}.count_gte", path), "UNCLASSIFIED_STRING");
        level_sum_gte.walk(c, &format!("{}.level_sum_gte", path), "UNCLASSIFIED_STRING");
        any_permanent.walk(c, &format!("{}.any_permanent", path), "UNCLASSIFIED_STRING");
        any_field_permanent.walk(
            c,
            &format!("{}.any_field_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        no_permanent.walk(c, &format!("{}.no_permanent", path), "UNCLASSIFIED_STRING");
        all_permanents.walk(
            c,
            &format!("{}.all_permanents", path),
            "UNCLASSIFIED_STRING",
        );
        all_of.walk(c, &format!("{}.all_of", path), "UNCLASSIFIED_STRING");
        any_of.walk(c, &format!("{}.any_of", path), "UNCLASSIFIED_STRING");
        none_of.walk(c, &format!("{}.none_of", path), "UNCLASSIFIED_STRING");
        not.walk(c, &format!("{}.not", path), "UNCLASSIFIED_STRING");
        has_alt_path.walk(c, &format!("{}.has_alt_path", path), "UNCLASSIFIED_STRING");
        cost_target.walk(c, &format!("{}.cost_target", path), "UNCLASSIFIED_STRING");
        cost_target_permanent.walk(c, &format!("{}.cost_target_permanent", path), "UNCLASSIFIED_STRING");
        source_is_cost_target_permanent.walk(
            c,
            &format!("{}.source_is_cost_target_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        cost_target_from_hand.walk(
            c,
            &format!("{}.cost_target_from_hand", path),
            "UNCLASSIFIED_STRING",
        );
        dp.walk(c, &format!("{}.dp", path), "UNCLASSIFIED_STRING");
        event_target_dp.walk(
            c,
            &format!("{}.event_target_dp", path),
            "UNCLASSIFIED_STRING",
        );
        play_cost.walk(c, &format!("{}.play_cost", path), "UNCLASSIFIED_STRING");
        stack_size.walk(c, &format!("{}.stack_size", path), "UNCLASSIFIED_STRING");
        materials_count.walk(
            c,
            &format!("{}.materials_count", path),
            "UNCLASSIFIED_STRING",
        );
        security_count.walk(
            c,
            &format!("{}.security_count", path),
            "UNCLASSIFIED_STRING",
        );
        level.walk(c, &format!("{}.level", path), "UNCLASSIFIED_STRING");
        event_target_level.walk(
            c,
            &format!("{}.event_target_level", path),
            "UNCLASSIFIED_STRING",
        );
        extra.walk(c, &format!("{}.extra", path), "UNCLASSIFIED_STRING");
            can_play_from_effect_without_cost.walk(c, &format!("{}.can_play_from_effect_without_cost", path), "UNCLASSIFIED_STRING");
        can_play_to_breeding_from_hand.walk(c, &format!("{}.can_play_to_breeding_from_hand", path), "UNCLASSIFIED_STRING");
        can_unsuspend.walk(c, &format!("{}.can_unsuspend", path), "UNCLASSIFIED_STRING");
        owner_can_add_security.walk(c, &format!("{}.owner_can_add_security", path), "UNCLASSIFIED_STRING");
        source_name_is.walk(c, &format!("{}.source_name_is", path), "MATCH_NAME_EXACT");
        event_battle_surviving_deleter.walk(c, &format!("{}.event_battle_surviving_deleter", path), "UNCLASSIFIED_STRING");
        event_card_name_is.walk(c, &format!("{}.event_card_name_is", path), "MATCH_NAME_EXACT");
        event_card_kind.walk(c, &format!("{}.event_card_kind", path), "UNCLASSIFIED_STRING");
}
}
impl Walk for digimon_dsl::predicate::BindingOwnerPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::BindingOwnerPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::BindingOwnerPredicate { binding, of } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::BindingCardKindPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::BindingCardKindPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::BindingCardKindPredicate { binding, kind } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::BindingCardColorPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::BindingCardColorPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::BindingCardColorPredicate { binding, color_is } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::BindingCardNamePredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::BindingCardNamePredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::BindingCardNamePredicate { binding, name_is } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        name_is.walk(
            c,
            &format!("{}.name_is", path),
            "MATCH_BOUND_CARD_NAME_EXACT",
        );
    }
}
impl Walk for digimon_dsl::predicate::PlayCostBindingPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::PlayCostBindingPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::PlayCostBindingPredicate {
            binding,
            offset,
            op,
        } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        offset.walk(c, &format!("{}.offset", path), "UNCLASSIFIED_STRING");
        op.walk(c, &format!("{}.op", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::DistinctColorsCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::DistinctColorsCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::DistinctColorsCountPredicate { of, filter, n } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::DistinctNamedCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::DistinctNamedCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::DistinctNamedCountPredicate { of, filter, n } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::DpConstraint {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::DpConstraint",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::DpConstraint::Literal(f0) => {
                c.node(
                    path,
                    "digimon_dsl::predicate::DpConstraint",
                    "VARIANT",
                    serde_json::json!("Literal"),
                );
                f0.walk(c, &format!("{}@Literal.0", path), role);
            }
            digimon_dsl::predicate::DpConstraint::Formula(f0) => {
                c.node(
                    path,
                    "digimon_dsl::predicate::DpConstraint",
                    "VARIANT",
                    serde_json::json!("Formula"),
                );
                f0.walk(c, &format!("{}@Formula.0", path), role);
            }
        }
    }
}
impl Walk for digimon_dsl::predicate::ComparatorOp {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::ComparatorOp",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::ComparatorOp::Eq => c.node(
                path,
                "digimon_dsl::predicate::ComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Eq"),
            ),
            digimon_dsl::predicate::ComparatorOp::Gte => c.node(
                path,
                "digimon_dsl::predicate::ComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Gte"),
            ),
            digimon_dsl::predicate::ComparatorOp::Lte => c.node(
                path,
                "digimon_dsl::predicate::ComparatorOp",
                "ENUM_SCALAR",
                serde_json::json!("Lte"),
            ),
        }
    }
}
impl Walk for digimon_dsl::predicate::Comparator {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::Comparator",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::Comparator { op, value } = self;
        op.walk(c, &format!("{}.op", path), "UNCLASSIFIED_STRING");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::MetricComparators {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::MetricComparators",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::MetricComparators(f0) = self;
        f0.walk(c, &format!("{}.0", path), role);
    }
}
impl Walk for digimon_dsl::predicate::ReplacementCauseSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::ReplacementCauseSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::ReplacementCauseSpec::Battle => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Battle"),
            ),
            digimon_dsl::predicate::ReplacementCauseSpec::OwnEffect => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("OwnEffect"),
            ),
            digimon_dsl::predicate::ReplacementCauseSpec::OpponentEffect => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("OpponentEffect"),
            ),
            digimon_dsl::predicate::ReplacementCauseSpec::SecurityCheck => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("SecurityCheck"),
            ),
            digimon_dsl::predicate::ReplacementCauseSpec::Cost => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Cost"),
            ),
            digimon_dsl::predicate::ReplacementCauseSpec::Overclock => c.node(
                path,
                "digimon_dsl::predicate::ReplacementCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Overclock"),
            ),
        }
    }
}
impl Walk for digimon_dsl::predicate::EventCauseSpec {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::EventCauseSpec",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::EventCauseSpec::BattleDeletion => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("BattleDeletion"),
            ),
            digimon_dsl::predicate::EventCauseSpec::EffectDeletion => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("EffectDeletion"),
            ),
            digimon_dsl::predicate::EventCauseSpec::OwnEffect => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("OwnEffect"),
            ),
            digimon_dsl::predicate::EventCauseSpec::OpponentEffect => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("OpponentEffect"),
            ),
            digimon_dsl::predicate::EventCauseSpec::Overclock => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Overclock"),
            ),
            digimon_dsl::predicate::EventCauseSpec::Return => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Return"),
            ),
            digimon_dsl::predicate::EventCauseSpec::DeckBottom => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("DeckBottom"),
            ),
            digimon_dsl::predicate::EventCauseSpec::SecurityPlacement => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("SecurityPlacement"),
            ),
            digimon_dsl::predicate::EventCauseSpec::SecurityRemoval => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("SecurityRemoval"),
            ),
            digimon_dsl::predicate::EventCauseSpec::Cost => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Cost"),
            ),
            digimon_dsl::predicate::EventCauseSpec::Rule => c.node(
                path,
                "digimon_dsl::predicate::EventCauseSpec",
                "ENUM_SCALAR",
                serde_json::json!("Rule"),
            ),
        }
    }
}
impl Walk for digimon_dsl::predicate::PlayerRefSelector {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::PlayerRefSelector",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::PlayerRefSelector::Player(f0) => {
                c.node(
                    path,
                    "digimon_dsl::predicate::PlayerRefSelector",
                    "VARIANT",
                    serde_json::json!("Player"),
                );
                f0.walk(c, &format!("{}@Player.0", path), role);
            }
            digimon_dsl::predicate::PlayerRefSelector::Scoped { of } => {
                c.node(
                    path,
                    "digimon_dsl::predicate::PlayerRefSelector",
                    "VARIANT",
                    serde_json::json!("Scoped"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@Scoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::predicate::SourceStackScopeOf {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::SourceStackScopeOf",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::SourceStackScopeOf::SelfCarrier => c.node(
                path,
                "digimon_dsl::predicate::SourceStackScopeOf",
                "ENUM_SCALAR",
                serde_json::json!("SelfCarrier"),
            ),
        }
    }
}
impl Walk for digimon_dsl::predicate::SourceStackScope {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::SourceStackScope",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::SourceStackScope::Scoped { of } => {
                c.node(
                    path,
                    "digimon_dsl::predicate::SourceStackScope",
                    "VARIANT",
                    serde_json::json!("Scoped"),
                );
                of.walk(
                    c,
                    &format!("{}.of", &format!("{}@Scoped", path)),
                    "UNCLASSIFIED_STRING",
                );
            }
        }
    }
}
impl Walk for digimon_dsl::predicate::LevelAggregatePredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::LevelAggregatePredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::LevelAggregatePredicate { selector, of } = self;
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::MaterialCountAggregatePredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::MaterialCountAggregatePredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::MaterialCountAggregatePredicate { selector, of } = self;
        selector.walk(c, &format!("{}.selector", path), "UNCLASSIFIED_STRING");
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::SourceCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::SourceCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::SourceCountPredicate { filter, at_least } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        at_least.walk(c, &format!("{}.at_least", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::DigivolveCandidatePredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::DigivolveCandidatePredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::DigivolveCandidatePredicate { of, zone, filter, cost } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        cost.walk(c, &format!("{}.cost", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::DigivolveCostPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::predicate::DigivolveCostPredicate", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::predicate::DigivolveCostPredicate { target, cost } = self;
        target.walk(c, &format!("{path}.target"), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{path}.cost"), role);
    }
}
impl Walk for digimon_dsl::compiled::CompiledDigivolveCostPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(path, "digimon_dsl::compiled::CompiledDigivolveCostPredicate", "COMPOSITE", serde_json::Value::Null);
        let digimon_dsl::compiled::CompiledDigivolveCostPredicate { target, cost } = self;
        target.walk(c, &format!("{path}.target"), "EXISTING_INSTANCE_BINDING");
        cost.walk(c, &format!("{path}.cost"), role);
    }
}
impl Walk for digimon_dsl::predicate::CanAttackPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::CanAttackPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::CanAttackPredicate {
            targets,
            without_suspending,
            ignore_summoning_sickness,
        } = self;
        targets.walk(c, &format!("{}.targets", path), "UNCLASSIFIED_STRING");
        without_suspending.walk(
            c,
            &format!("{}.without_suspending", path),
            "UNCLASSIFIED_STRING",
        );
        ignore_summoning_sickness.walk(
            c,
            &format!("{}.ignore_summoning_sickness", path),
            "UNCLASSIFIED_STRING",
        );
    }
}
impl Walk for digimon_dsl::predicate::FaceUpSecurityNamedPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::FaceUpSecurityNamedPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::FaceUpSecurityNamedPredicate {
            of,
            card_number_is,
            name_is,
            color_is,
        } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        card_number_is.walk(c, &format!("{}.card_number_is", path), "MATCH_CARD_ID");
        name_is.walk(c, &format!("{}.name_is", path), "MATCH_NAME_EXACT");
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::BindingCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::BindingCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::BindingCountPredicate { binding, n } = self;
        binding.walk(c, &format!("{}.binding", path), "EXISTING_INSTANCE_BINDING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::SelfSourceCountPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::SelfSourceCountPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::SelfSourceCountPredicate { filter, op, value } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        op.walk(c, &format!("{}.op", path), "UNCLASSIFIED_STRING");
        value.walk(c, &format!("{}.value", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::CountAggregate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::CountAggregate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::CountAggregate { filter, n } = self;
        filter.walk(c, &format!("{}.filter", path), "UNCLASSIFIED_STRING");
        n.walk(c, &format!("{}.n", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::ExistentialPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::ExistentialPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::predicate::ExistentialPredicate { of, predicate } = self;
        of.walk(c, &format!("{}.of", path), "UNCLASSIFIED_STRING");
        predicate.walk(c, &format!("{}.predicate", path), "UNCLASSIFIED_STRING");
    }
}
impl Walk for digimon_dsl::predicate::Zone {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::predicate::Zone",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        match self {
            digimon_dsl::predicate::Zone::Hand => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Hand"),
            ),
            digimon_dsl::predicate::Zone::Deck => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Deck"),
            ),
            digimon_dsl::predicate::Zone::Trash => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Trash"),
            ),
            digimon_dsl::predicate::Zone::BattleArea => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("BattleArea"),
            ),
            digimon_dsl::predicate::Zone::Security => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Security"),
            ),
            digimon_dsl::predicate::Zone::Breeding => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Breeding"),
            ),
            digimon_dsl::predicate::Zone::Reveal => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Reveal"),
            ),
            digimon_dsl::predicate::Zone::DigiEggDeck => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("DigiEggDeck"),
            ),
            digimon_dsl::predicate::Zone::Material => c.node(
                path,
                "digimon_dsl::predicate::Zone",
                "ENUM_SCALAR",
                serde_json::json!("Material"),
            ),
        }
    }
}
impl Walk for digimon_dsl::compiled::CompiledPredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        c.node(
            path,
            "digimon_dsl::compiled::CompiledPredicate",
            "COMPOSITE",
            serde_json::Value::Null,
        );
        let digimon_dsl::compiled::CompiledPredicate {
            kind,
            level_eq,
            level_eq_binding,
            level_lte_binding,
            level_gte_binding,
            level_lte,
            level_gte,
            color_is,
            color_only,
            color_matches_any_field_digimon,
            color_matches_binding,
            color_matches_returned_card,
            color_matches_own_source_stack,
            trait_has,
            trait_contains,
            form_is,
            attribute_is,
            name_is,
            name_contains,
            effect_text_contains,
            printed_keyword,
            in_text_contains,
            name_in,
            name_not_shared_by_field_digimon,
            name_not_shared_by_field_tamer,
            card_number_is,
            play_cost_lte,
            play_cost_gte,
            play_cost_eq,
            play_or_use_cost_lte,
            affordable_with_cost_reduce,
            can_digivolve_from_source,
            can_digivolve_onto,
            can_digivolve_onto_with_cost,
            dna_hand_candidate,
            can_move_from_breeding_by_effect,
            dp_eq,
            dp_lte,
            dp_lte_deletion_cap,
            dp_gte,
            stack_size_lte,
            stack_size_gte,
            stack_size_eq,
            materials_count_lte,
            materials_count_gte,
            materials_count_eq,
            source_count,
            link_card_count,
            has_digivolve_candidate,
            has_inherited,
            is_suspended,
            is_unsuspended,
            can_change_orientation,
            can_attack,
            has_keyword,
            has_security_attack_change,
            has_on_deletion_effect,
            self_color_count_gte,
            self_color_count_lte,
            has_face_down_source,
            distinct_tamer_colors_gte,
            distinct_named_count_gte,
            distinct_colors_gte,
            face_down_sources_under_tamers_gte,
            battle_opponent_no_sources,
            self_source_count,
            self_same_level_source_pairs_gte,
            own_source_stack_color_count_gte,
            zone,
            owner,
            other,
            is_source,
            of_permanent,
            not_in_binding,
            name_not_in_binding,
            binding_owner,
            binding_card_kind,
            binding_card_color,
            binding_card_name_is,
            play_cost_eq_binding,
            source_is_tamer,
            source_is_unsuspended,
            is_source_permanent,
            source_name_contains,
            source_permanent_trait_has,
            is_face_down,
            is_bottom_source,
            host_kind_is,
            rules_text_contains,
            memory_lte,
            memory_gte,
            own_memory_lte,
            own_memory_gte,
            security_count_lte,
            security_count_gte,
            security_count_eq,
            opponent_security_count_lte,
            opponent_security_count_gte,
            total_security_count_lte,
            total_security_count_gte,
            total_security_count_eq,
            face_up_security_count_lte,
            face_up_security_count_gte,
            no_face_up_security_named,
            binding_count_eq,
            your_turn,
            opponents_turn,
            all_turns,
            can_hatch,
            digimon_attacked_this_turn,
            during_attack,
            can_gain_memory,
            in_breeding,
            on_field,
            dna_origin,
            event_target_kind,
            event_target_trait_has,
            event_target_trait_contains,
            event_target_has_digivolution_cards,
            event_target_stack_size_gte,
            event_target_level_eq,
            event_target_level_lte,
            event_target_level_gte,
            event_target_dp_eq,
            event_target_dp_lte,
            event_target_dp_gte,
            event_target_name_contains,
            event_target_in_text_contains,
            event_target_has_inherited,
            event_target_is_player,
            event_target_is_source,
            event_target_was_self,
            attack_target_change_reason,
            attacker_trait_has,
            event_card_trait_has,
            event_card_name_contains,
            event_card_text_contains,
            event_card_in_text_contains,
            event_card_level_eq,
            event_card_level_gte,
            event_card_color_only,
            event_card_color_has,
            event_card_color_count,
            event_permanent_is_source,
            source_deleted_battle_opponent,
            event_host_permanent_is_source,
            event_host_is_own_tamer,
            event_host_is_own_digimon,
            is_event_host,
            event_is_effect_initiated,
            event_target_same_level_as_previous,
            event_cause,
            replacement_cause,
            replacement_source_is_opponent,
            replacement_subject_is_mine,
            would_link_card_trait_any_of,
            equals,
            not_equals,
            binding_exists,
            binding_present,
            binding_absent,
            effect_suspended_any_own_digimon,
            effect_suspended_any_opponent_digimon,
            effect_returned_any_card,
            returned_card_matching,
            effect_trashed_any_hand_card,
            trashed_hand_card_matching,
            effect_deleted_any_own_digimon,
            effect_deleted_any_opponent_digimon,
            effect_deleted_opponent_digimon_dp_gte,
            effect_played_any_digimon,
            effect_digivolved_any_digimon,
            effect_added_any_card_to_hand,
            count_lte,
            count_gte,
            level_sum_gte,
            any_permanent,
            any_field_permanent,
            no_permanent,
            all_permanents,
            all_of,
            any_of,
            none_of,
            not,
            has_alt_path,
            level_matches_aggregate,
            materials_count_matches_aggregate,
            self_digivolution_contains_name,
            self_digivolution_sources_contain_name,
            self_digivolution_sources_trait_has,
            event_target_owner,
            event_add_to_hand_player,
            event_winner_owner,
            event_winner_trait_has,
            event_winner_is_source,
            event_battle_deleter,
            event_discard_player,
            event_caused_by_own_effect,
            event_added_card_any,
            event_discarded_card_any,
            in_event_discarded_cards,
            played_by_effect,
            event_target_color_any_of,
            host_permanent_trait_has,
            trashed_source_trait_has,
            trashed_source_card_id_is,
            cost_target,
            cost_target_permanent,
            source_is_cost_target_permanent,
            cost_target_from_hand,
            can_play_from_effect_without_cost,
            can_play_to_breeding_from_hand,
            can_unsuspend,
            owner_can_add_security,
            controller_can_add_security,
            source_name_is,
            event_card_name_is,
            event_card_kind,
            event_battle_surviving_deleter,
        } = self;
        kind.walk(c, &format!("{}.kind", path), "UNCLASSIFIED_STRING");
        level_eq.walk(c, &format!("{}.level_eq", path), "UNCLASSIFIED_STRING");
        level_eq_binding.walk(
            c,
            &format!("{}.level_eq_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_lte_binding.walk(
            c,
            &format!("{}.level_lte_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_gte_binding.walk(
            c,
            &format!("{}.level_gte_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        level_lte.walk(c, &format!("{}.level_lte", path), "UNCLASSIFIED_STRING");
        level_gte.walk(c, &format!("{}.level_gte", path), "UNCLASSIFIED_STRING");
        color_is.walk(c, &format!("{}.color_is", path), "UNCLASSIFIED_STRING");
        color_only.walk(c, &format!("{}.color_only", path), "UNCLASSIFIED_STRING");
        color_matches_any_field_digimon.walk(
            c,
            &format!("{}.color_matches_any_field_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        color_matches_binding.walk(
            c,
            &format!("{}.color_matches_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        color_matches_returned_card.walk(
            c,
            &format!("{}.color_matches_returned_card", path),
            "UNCLASSIFIED_STRING",
        );
        color_matches_own_source_stack.walk(
            c,
            &format!("{}.color_matches_own_source_stack", path),
            "UNCLASSIFIED_STRING",
        );
        trait_has.walk(c, &format!("{}.trait_has", path), "MATCH_OR_DEFINE_TRAIT");
        trait_contains.walk(
            c,
            &format!("{}.trait_contains", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        form_is.walk(c, &format!("{}.form_is", path), "MATCH_OR_DEFINE_TRAIT");
        attribute_is.walk(
            c,
            &format!("{}.attribute_is", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        name_is.walk(c, &format!("{}.name_is", path), "MATCH_NAME_EXACT");
        name_contains.walk(
            c,
            &format!("{}.name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        effect_text_contains.walk(c, &format!("{}.effect_text_contains", path), "MATCH_TEXT");
        printed_keyword.walk(
            c,
            &format!("{}.printed_keyword", path),
            "UNCLASSIFIED_STRING",
        );
        in_text_contains.walk(c, &format!("{}.in_text_contains", path), "MATCH_TEXT");
        name_in.walk(c, &format!("{}.name_in", path), "MATCH_NAME_EXACT");
        name_not_shared_by_field_digimon.walk(
            c,
            &format!("{}.name_not_shared_by_field_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        name_not_shared_by_field_tamer.walk(
            c,
            &format!("{}.name_not_shared_by_field_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        card_number_is.walk(c, &format!("{}.card_number_is", path), "MATCH_CARD_ID");
        play_cost_lte.walk(c, &format!("{}.play_cost_lte", path), "UNCLASSIFIED_STRING");
        play_cost_gte.walk(c, &format!("{}.play_cost_gte", path), "UNCLASSIFIED_STRING");
        play_cost_eq.walk(c, &format!("{}.play_cost_eq", path), "UNCLASSIFIED_STRING");
        play_or_use_cost_lte.walk(
            c,
            &format!("{}.play_or_use_cost_lte", path),
            "UNCLASSIFIED_STRING",
        );
        affordable_with_cost_reduce.walk(
            c,
            &format!("{}.affordable_with_cost_reduce", path),
            "UNCLASSIFIED_STRING",
        );
        can_digivolve_from_source.walk(
            c,
            &format!("{}.can_digivolve_from_source", path),
            "UNCLASSIFIED_STRING",
        );
        can_digivolve_onto.walk(
            c,
            &format!("{}.can_digivolve_onto", path),
            "EXISTING_INSTANCE_BINDING",
        );
        dp_eq.walk(c, &format!("{}.dp_eq", path), "UNCLASSIFIED_STRING");
        dp_lte.walk(c, &format!("{}.dp_lte", path), "UNCLASSIFIED_STRING");
        can_digivolve_onto_with_cost.walk(c, &format!("{}.can_digivolve_onto_with_cost", path), role);
        dna_hand_candidate.walk(c, &format!("{}.dna_hand_candidate", path), role);
        can_move_from_breeding_by_effect.walk(c, &format!("{}.can_move_from_breeding_by_effect", path), role);
        dp_lte_deletion_cap.walk(
            c,
            &format!("{}.dp_lte_deletion_cap", path),
            "UNCLASSIFIED_STRING",
        );
        dp_gte.walk(c, &format!("{}.dp_gte", path), "UNCLASSIFIED_STRING");
        stack_size_lte.walk(
            c,
            &format!("{}.stack_size_lte", path),
            "UNCLASSIFIED_STRING",
        );
        stack_size_gte.walk(
            c,
            &format!("{}.stack_size_gte", path),
            "UNCLASSIFIED_STRING",
        );
        stack_size_eq.walk(c, &format!("{}.stack_size_eq", path), "UNCLASSIFIED_STRING");
        materials_count_lte.walk(
            c,
            &format!("{}.materials_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_gte.walk(
            c,
            &format!("{}.materials_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_eq.walk(
            c,
            &format!("{}.materials_count_eq", path),
            "UNCLASSIFIED_STRING",
        );
        source_count.walk(c, &format!("{}.source_count", path), "UNCLASSIFIED_STRING");
        link_card_count.walk(
            c,
            &format!("{}.link_card_count", path),
            "UNCLASSIFIED_STRING",
        );
        has_digivolve_candidate.walk(
            c,
            &format!("{}.has_digivolve_candidate", path),
            "UNCLASSIFIED_STRING",
        );
        has_inherited.walk(c, &format!("{}.has_inherited", path), "UNCLASSIFIED_STRING");
        is_suspended.walk(c, &format!("{}.is_suspended", path), "UNCLASSIFIED_STRING");
        is_unsuspended.walk(
            c,
            &format!("{}.is_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        can_change_orientation.walk(
            c,
            &format!("{}.can_change_orientation", path),
            "UNCLASSIFIED_STRING",
        );
        can_attack.walk(c, &format!("{}.can_attack", path), "UNCLASSIFIED_STRING");
        has_keyword.walk(c, &format!("{}.has_keyword", path), "UNCLASSIFIED_STRING");
        has_security_attack_change.walk(
            c,
            &format!("{}.has_security_attack_change", path),
            "UNCLASSIFIED_STRING",
        );
        has_on_deletion_effect.walk(
            c,
            &format!("{}.has_on_deletion_effect", path),
            "UNCLASSIFIED_STRING",
        );
        self_color_count_gte.walk(
            c,
            &format!("{}.self_color_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        self_color_count_lte.walk(
            c,
            &format!("{}.self_color_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        has_face_down_source.walk(
            c,
            &format!("{}.has_face_down_source", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_tamer_colors_gte.walk(
            c,
            &format!("{}.distinct_tamer_colors_gte", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_named_count_gte.walk(
            c,
            &format!("{}.distinct_named_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        distinct_colors_gte.walk(
            c,
            &format!("{}.distinct_colors_gte", path),
            "UNCLASSIFIED_STRING",
        );
        face_down_sources_under_tamers_gte.walk(
            c,
            &format!("{}.face_down_sources_under_tamers_gte", path),
            "UNCLASSIFIED_STRING",
        );
        battle_opponent_no_sources.walk(
            c,
            &format!("{}.battle_opponent_no_sources", path),
            "UNCLASSIFIED_STRING",
        );
        self_source_count.walk(
            c,
            &format!("{}.self_source_count", path),
            "UNCLASSIFIED_STRING",
        );
        self_same_level_source_pairs_gte.walk(
            c,
            &format!("{}.self_same_level_source_pairs_gte", path),
            "UNCLASSIFIED_STRING",
        );
        own_source_stack_color_count_gte.walk(
            c,
            &format!("{}.own_source_stack_color_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        zone.walk(c, &format!("{}.zone", path), "UNCLASSIFIED_STRING");
        owner.walk(c, &format!("{}.owner", path), "UNCLASSIFIED_STRING");
        other.walk(c, &format!("{}.other", path), "UNCLASSIFIED_STRING");
        is_source.walk(c, &format!("{}.is_source", path), "UNCLASSIFIED_STRING");
        of_permanent.walk(
            c,
            &format!("{}.of_permanent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        not_in_binding.walk(
            c,
            &format!("{}.not_in_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        name_not_in_binding.walk(
            c,
            &format!("{}.name_not_in_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_owner.walk(c, &format!("{}.binding_owner", path), "UNCLASSIFIED_STRING");
        binding_card_kind.walk(
            c,
            &format!("{}.binding_card_kind", path),
            "UNCLASSIFIED_STRING",
        );
        binding_card_color.walk(
            c,
            &format!("{}.binding_card_color", path),
            "UNCLASSIFIED_STRING",
        );
        match binding_card_name_is {
            Some((binding, name)) => {
                binding.walk(
                    c,
                    &format!("{}.binding_card_name_is.0", path),
                    "EXISTING_INSTANCE_BINDING",
                );
                name.walk(
                    c,
                    &format!("{}.binding_card_name_is.1", path),
                    "MATCH_BOUND_CARD_NAME_EXACT",
                );
            }
            None => c.node(
                &format!("{}.binding_card_name_is", path),
                "Option",
                "ABSENT",
                serde_json::Value::Null,
            ),
        }
        play_cost_eq_binding.walk(
            c,
            &format!("{}.play_cost_eq_binding", path),
            "EXISTING_INSTANCE_BINDING",
        );
        source_is_tamer.walk(
            c,
            &format!("{}.source_is_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        source_is_unsuspended.walk(
            c,
            &format!("{}.source_is_unsuspended", path),
            "UNCLASSIFIED_STRING",
        );
        is_source_permanent.walk(
            c,
            &format!("{}.is_source_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        source_name_contains.walk(
            c,
            &format!("{}.source_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        source_permanent_trait_has.walk(
            c,
            &format!("{}.source_permanent_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        is_face_down.walk(c, &format!("{}.is_face_down", path), "UNCLASSIFIED_STRING");
        is_bottom_source.walk(
            c,
            &format!("{}.is_bottom_source", path),
            "UNCLASSIFIED_STRING",
        );
        host_kind_is.walk(c, &format!("{}.host_kind_is", path), "UNCLASSIFIED_STRING");
        rules_text_contains.walk(
            c,
            &format!("{}.rules_text_contains", path),
            "UNCLASSIFIED_STRING",
        );
        memory_lte.walk(c, &format!("{}.memory_lte", path), "UNCLASSIFIED_STRING");
        memory_gte.walk(c, &format!("{}.memory_gte", path), "UNCLASSIFIED_STRING");
        own_memory_lte.walk(
            c,
            &format!("{}.own_memory_lte", path),
            "UNCLASSIFIED_STRING",
        );
        own_memory_gte.walk(
            c,
            &format!("{}.own_memory_gte", path),
            "UNCLASSIFIED_STRING",
        );
        security_count_lte.walk(
            c,
            &format!("{}.security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        security_count_gte.walk(
            c,
            &format!("{}.security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        security_count_eq.walk(
            c,
            &format!("{}.security_count_eq", path),
            "UNCLASSIFIED_STRING",
        );
        opponent_security_count_lte.walk(
            c,
            &format!("{}.opponent_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        opponent_security_count_gte.walk(
            c,
            &format!("{}.opponent_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_lte.walk(
            c,
            &format!("{}.total_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_gte.walk(
            c,
            &format!("{}.total_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        total_security_count_eq.walk(
            c,
            &format!("{}.total_security_count_eq", path),
            "UNCLASSIFIED_STRING",
        );
        face_up_security_count_lte.walk(
            c,
            &format!("{}.face_up_security_count_lte", path),
            "UNCLASSIFIED_STRING",
        );
        face_up_security_count_gte.walk(
            c,
            &format!("{}.face_up_security_count_gte", path),
            "UNCLASSIFIED_STRING",
        );
        no_face_up_security_named.walk(
            c,
            &format!("{}.no_face_up_security_named", path),
            "UNCLASSIFIED_STRING",
        );
        binding_count_eq.walk(
            c,
            &format!("{}.binding_count_eq", path),
            "EXISTING_INSTANCE_BINDING",
        );
        your_turn.walk(c, &format!("{}.your_turn", path), "UNCLASSIFIED_STRING");
        opponents_turn.walk(
            c,
            &format!("{}.opponents_turn", path),
            "UNCLASSIFIED_STRING",
        );
        all_turns.walk(c, &format!("{}.all_turns", path), "UNCLASSIFIED_STRING");
        can_hatch.walk(c, &format!("{}.can_hatch", path), "UNCLASSIFIED_STRING");
        digimon_attacked_this_turn.walk(
            c,
            &format!("{}.digimon_attacked_this_turn", path),
            "UNCLASSIFIED_STRING",
        );
        during_attack.walk(c, &format!("{}.during_attack", path), "UNCLASSIFIED_STRING");
        can_gain_memory.walk(c, &format!("{}.can_gain_memory", path), "UNCLASSIFIED_STRING");
        in_breeding.walk(c, &format!("{}.in_breeding", path), "UNCLASSIFIED_STRING");
        on_field.walk(c, &format!("{}.on_field", path), "UNCLASSIFIED_STRING");
        dna_origin.walk(c, &format!("{}.dna_origin", path), "UNCLASSIFIED_STRING");
        event_target_kind.walk(
            c,
            &format!("{}.event_target_kind", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_trait_has.walk(
            c,
            &format!("{}.event_target_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_target_trait_contains.walk(
            c,
            &format!("{}.event_target_trait_contains", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_target_has_digivolution_cards.walk(
            c,
            &format!("{}.event_target_has_digivolution_cards", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_stack_size_gte.walk(
            c,
            &format!("{}.event_target_stack_size_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_eq.walk(
            c,
            &format!("{}.event_target_level_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_lte.walk(
            c,
            &format!("{}.event_target_level_lte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_level_gte.walk(
            c,
            &format!("{}.event_target_level_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_eq.walk(
            c,
            &format!("{}.event_target_dp_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_lte.walk(
            c,
            &format!("{}.event_target_dp_lte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_dp_gte.walk(
            c,
            &format!("{}.event_target_dp_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_name_contains.walk(
            c,
            &format!("{}.event_target_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        event_target_in_text_contains.walk(
            c,
            &format!("{}.event_target_in_text_contains", path),
            "MATCH_TEXT",
        );
        event_target_has_inherited.walk(
            c,
            &format!("{}.event_target_has_inherited", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_is_player.walk(
            c,
            &format!("{}.event_target_is_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_is_source.walk(
            c,
            &format!("{}.event_target_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_was_self.walk(
            c,
            &format!("{}.event_target_was_self", path),
            "UNCLASSIFIED_STRING",
        );
        attack_target_change_reason.walk(
            c,
            &format!("{}.attack_target_change_reason", path),
            "UNCLASSIFIED_STRING",
        );
        attacker_trait_has.walk(
            c,
            &format!("{}.attacker_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_card_trait_has.walk(
            c,
            &format!("{}.event_card_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_card_name_contains.walk(
            c,
            &format!("{}.event_card_name_contains", path),
            "MATCH_NAME_SUBSTRING",
        );
        event_card_text_contains.walk(
            c,
            &format!("{}.event_card_text_contains", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_in_text_contains.walk(
            c,
            &format!("{}.event_card_in_text_contains", path),
            "MATCH_TEXT",
        );
        event_card_level_eq.walk(
            c,
            &format!("{}.event_card_level_eq", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_level_gte.walk(
            c,
            &format!("{}.event_card_level_gte", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_only.walk(
            c,
            &format!("{}.event_card_color_only", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_has.walk(
            c,
            &format!("{}.event_card_color_has", path),
            "UNCLASSIFIED_STRING",
        );
        event_card_color_count.walk(
            c,
            &format!("{}.event_card_color_count", path),
            "UNCLASSIFIED_STRING",
        );
        event_permanent_is_source.walk(
            c,
            &format!("{}.event_permanent_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        source_deleted_battle_opponent.walk(
            c,
            &format!("{}.source_deleted_battle_opponent", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_permanent_is_source.walk(
            c,
            &format!("{}.event_host_permanent_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_is_own_tamer.walk(
            c,
            &format!("{}.event_host_is_own_tamer", path),
            "UNCLASSIFIED_STRING",
        );
        event_host_is_own_digimon.walk(
            c,
            &format!("{}.event_host_is_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        is_event_host.walk(c, &format!("{}.is_event_host", path), "UNCLASSIFIED_STRING");
        event_is_effect_initiated.walk(
            c,
            &format!("{}.event_is_effect_initiated", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_same_level_as_previous.walk(
            c,
            &format!("{}.event_target_same_level_as_previous", path),
            "UNCLASSIFIED_STRING",
        );
        event_cause.walk(c, &format!("{}.event_cause", path), "UNCLASSIFIED_STRING");
        replacement_cause.walk(
            c,
            &format!("{}.replacement_cause", path),
            "UNCLASSIFIED_STRING",
        );
        replacement_source_is_opponent.walk(
            c,
            &format!("{}.replacement_source_is_opponent", path),
            "UNCLASSIFIED_STRING",
        );
        replacement_subject_is_mine.walk(
            c,
            &format!("{}.replacement_subject_is_mine", path),
            "UNCLASSIFIED_STRING",
        );
        would_link_card_trait_any_of.walk(
            c,
            &format!("{}.would_link_card_trait_any_of", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        equals.walk(c, &format!("{}.equals", path), "UNCLASSIFIED_STRING");
        not_equals.walk(c, &format!("{}.not_equals", path), "UNCLASSIFIED_STRING");
        binding_exists.walk(
            c,
            &format!("{}.binding_exists", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_present.walk(
            c,
            &format!("{}.binding_present", path),
            "EXISTING_INSTANCE_BINDING",
        );
        binding_absent.walk(
            c,
            &format!("{}.binding_absent", path),
            "EXISTING_INSTANCE_BINDING",
        );
        effect_suspended_any_own_digimon.walk(
            c,
            &format!("{}.effect_suspended_any_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_suspended_any_opponent_digimon.walk(
            c,
            &format!("{}.effect_suspended_any_opponent_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_returned_any_card.walk(
            c,
            &format!("{}.effect_returned_any_card", path),
            "UNCLASSIFIED_STRING",
        );
        returned_card_matching.walk(
            c,
            &format!("{}.returned_card_matching", path),
            "UNCLASSIFIED_STRING",
        );
        effect_trashed_any_hand_card.walk(
            c,
            &format!("{}.effect_trashed_any_hand_card", path),
            "UNCLASSIFIED_STRING",
        );
        trashed_hand_card_matching.walk(
            c,
            &format!("{}.trashed_hand_card_matching", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_any_own_digimon.walk(
            c,
            &format!("{}.effect_deleted_any_own_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_any_opponent_digimon.walk(
            c,
            &format!("{}.effect_deleted_any_opponent_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_deleted_opponent_digimon_dp_gte.walk(
            c,
            &format!("{}.effect_deleted_opponent_digimon_dp_gte", path),
            "UNCLASSIFIED_STRING",
        );
        effect_played_any_digimon.walk(
            c,
            &format!("{}.effect_played_any_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_digivolved_any_digimon.walk(
            c,
            &format!("{}.effect_digivolved_any_digimon", path),
            "UNCLASSIFIED_STRING",
        );
        effect_added_any_card_to_hand.walk(
            c,
            &format!("{}.effect_added_any_card_to_hand", path),
            "UNCLASSIFIED_STRING",
        );
        count_lte.walk(c, &format!("{}.count_lte", path), "UNCLASSIFIED_STRING");
        count_gte.walk(c, &format!("{}.count_gte", path), "UNCLASSIFIED_STRING");
        level_sum_gte.walk(c, &format!("{}.level_sum_gte", path), "UNCLASSIFIED_STRING");
        any_permanent.walk(c, &format!("{}.any_permanent", path), "UNCLASSIFIED_STRING");
        any_field_permanent.walk(
            c,
            &format!("{}.any_field_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        no_permanent.walk(c, &format!("{}.no_permanent", path), "UNCLASSIFIED_STRING");
        all_permanents.walk(
            c,
            &format!("{}.all_permanents", path),
            "UNCLASSIFIED_STRING",
        );
        all_of.walk(c, &format!("{}.all_of", path), "UNCLASSIFIED_STRING");
        any_of.walk(c, &format!("{}.any_of", path), "UNCLASSIFIED_STRING");
        none_of.walk(c, &format!("{}.none_of", path), "UNCLASSIFIED_STRING");
        not.walk(c, &format!("{}.not", path), "UNCLASSIFIED_STRING");
        has_alt_path.walk(c, &format!("{}.has_alt_path", path), "UNCLASSIFIED_STRING");
        level_matches_aggregate.walk(
            c,
            &format!("{}.level_matches_aggregate", path),
            "UNCLASSIFIED_STRING",
        );
        materials_count_matches_aggregate.walk(
            c,
            &format!("{}.materials_count_matches_aggregate", path),
            "UNCLASSIFIED_STRING",
        );
        self_digivolution_contains_name.walk(
            c,
            &format!("{}.self_digivolution_contains_name", path),
            "UNCLASSIFIED_STRING",
        );
        self_digivolution_sources_contain_name.walk(
            c,
            &format!("{}.self_digivolution_sources_contain_name", path),
            "UNCLASSIFIED_STRING",
        );
        self_digivolution_sources_trait_has.walk(
            c,
            &format!("{}.self_digivolution_sources_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_target_owner.walk(
            c,
            &format!("{}.event_target_owner", path),
            "UNCLASSIFIED_STRING",
        );
        event_add_to_hand_player.walk(
            c,
            &format!("{}.event_add_to_hand_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_winner_owner.walk(
            c,
            &format!("{}.event_winner_owner", path),
            "UNCLASSIFIED_STRING",
        );
        event_winner_trait_has.walk(
            c,
            &format!("{}.event_winner_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        event_winner_is_source.walk(
            c,
            &format!("{}.event_winner_is_source", path),
            "UNCLASSIFIED_STRING",
        );
        event_battle_deleter.walk(
            c,
            &format!("{}.event_battle_deleter", path),
            "UNCLASSIFIED_STRING",
        );
        event_discard_player.walk(
            c,
            &format!("{}.event_discard_player", path),
            "UNCLASSIFIED_STRING",
        );
        event_caused_by_own_effect.walk(
            c,
            &format!("{}.event_caused_by_own_effect", path),
            "UNCLASSIFIED_STRING",
        );
        event_added_card_any.walk(
            c,
            &format!("{}.event_added_card_any", path),
            "UNCLASSIFIED_STRING",
        );
        event_discarded_card_any.walk(
            c,
            &format!("{}.event_discarded_card_any", path),
            "UNCLASSIFIED_STRING",
        );
        in_event_discarded_cards.walk(
            c,
            &format!("{}.in_event_discarded_cards", path),
            "UNCLASSIFIED_STRING",
        );
        played_by_effect.walk(
            c,
            &format!("{}.played_by_effect", path),
            "UNCLASSIFIED_STRING",
        );
        event_target_color_any_of.walk(
            c,
            &format!("{}.event_target_color_any_of", path),
            "UNCLASSIFIED_STRING",
        );
        host_permanent_trait_has.walk(
            c,
            &format!("{}.host_permanent_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        trashed_source_trait_has.walk(
            c,
            &format!("{}.trashed_source_trait_has", path),
            "MATCH_OR_DEFINE_TRAIT",
        );
        trashed_source_card_id_is.walk(
            c,
            &format!("{}.trashed_source_card_id_is", path),
            "MATCH_CARD_ID",
        );
        cost_target.walk(c, &format!("{}.cost_target", path), "UNCLASSIFIED_STRING");
        cost_target_permanent.walk(c, &format!("{}.cost_target_permanent", path), "UNCLASSIFIED_STRING");
        source_is_cost_target_permanent.walk(
            c,
            &format!("{}.source_is_cost_target_permanent", path),
            "UNCLASSIFIED_STRING",
        );
        cost_target_from_hand.walk(
            c,
            &format!("{}.cost_target_from_hand", path),
            "UNCLASSIFIED_STRING",
        );
            can_play_from_effect_without_cost.walk(c, &format!("{}.can_play_from_effect_without_cost", path), "UNCLASSIFIED_STRING");
        can_play_to_breeding_from_hand.walk(c, &format!("{}.can_play_to_breeding_from_hand", path), "UNCLASSIFIED_STRING");
        can_unsuspend.walk(c, &format!("{}.can_unsuspend", path), "UNCLASSIFIED_STRING");
        owner_can_add_security.walk(c, &format!("{}.owner_can_add_security", path), "UNCLASSIFIED_STRING");
        controller_can_add_security.walk(c, &format!("{}.controller_can_add_security", path), "UNCLASSIFIED_STRING");
        source_name_is.walk(c, &format!("{}.source_name_is", path), "MATCH_NAME_EXACT");
        event_card_name_is.walk(c, &format!("{}.event_card_name_is", path), "MATCH_NAME_EXACT");
        event_card_kind.walk(c, &format!("{}.event_card_kind", path), "UNCLASSIFIED_STRING");
        event_battle_surviving_deleter.walk(c, &format!("{}.event_battle_surviving_deleter", path), "UNCLASSIFIED_STRING");
}
}

impl Walk for digimon_dsl::predicate::DnaHandCandidateRole {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        let name = match self {
            Self::Result => "Result", Self::Field => "Field", Self::Partner => "Partner",
        };
        c.node(path, "digimon_dsl::predicate::DnaHandCandidateRole", role, serde_json::json!(name));
    }
}
impl Walk for digimon_dsl::predicate::DnaHandCandidatePredicate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        let Self { role: stage, result, field, field_filter, partner_filter, cost, ignore_requirements } = self;
        stage.walk(c, &format!("{}.role", path), role);
        result.walk(c, &format!("{}.result", path), "EXISTING_INSTANCE_BINDING");
        field.walk(c, &format!("{}.field", path), "EXISTING_INSTANCE_BINDING");
        field_filter.walk(c, &format!("{}.field_filter", path), role);
        partner_filter.walk(c, &format!("{}.partner_filter", path), role);
        cost.walk(c, &format!("{}.cost", path), role);
        ignore_requirements.walk(c, &format!("{}.ignore_requirements", path), role);
    }
}
impl Walk for digimon_dsl::compiled::CompiledDnaHandCandidate {
    fn walk(&self, c: &mut Collector, path: &str, role: &str) {
        let Self { role: stage, result, field, field_filter, partner_filter, cost, ignore_requirements } = self;
        stage.walk(c, &format!("{}.role", path), role);
        result.walk(c, &format!("{}.result", path), "EXISTING_INSTANCE_BINDING");
        field.walk(c, &format!("{}.field", path), "EXISTING_INSTANCE_BINDING");
        field_filter.walk(c, &format!("{}.field_filter", path), role);
        partner_filter.walk(c, &format!("{}.partner_filter", path), role);
        cost.walk(c, &format!("{}.cost", path), role);
        ignore_requirements.walk(c, &format!("{}.ignore_requirements", path), role);
    }
}
