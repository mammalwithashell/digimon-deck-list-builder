from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_073(CardScript):
    """EX11-073 ExMaquinamon | Lv.7

    DNA Digivolution: Green Lv.6 + Black Lv.6
    <Security Attack +1>
    <Blocker>
    <Link +2>

    [When Digivolving] If DNA Digivolving, you may link up to 3 [Maquinamon]
    from your hand, trash or this Digimon's digivolution cards to this
    Digimon without paying the cost.

    [End of Opponent's Turn] [Once Per Turn] For each of this Digimon's link
    cards, trash your opponent's top security card and return 1 of their
    Digimon to the bottom of the deck.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_maquinamon(c):
            names = getattr(c, 'card_names', []) or []
            return any('Maquinamon' in n for n in names)

        # --- Jogress Condition marker ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-073 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Security Attack +1 ---
        effect_sa = ICardEffect()
        effect_sa.set_effect_name("EX11-073 Security Attack +1")
        effect_sa.set_effect_description("Security Attack +1")
        effect_sa._security_attack_modifier = 1

        def condition_sa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_sa.set_can_use_condition(condition_sa)
        effects.append(effect_sa)

        # --- Blocker ---
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX11-073 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # --- Link +2 (up to 3 links total) ---
        for i in range(1, 4):
            link_effect = ICardEffect()
            link_effect.set_effect_name(f"EX11-073 Link slot {i} [Maquinamon]")
            link_effect.set_effect_description("Effect")

            def make_link_condition():
                def cond(context: Dict[str, Any]) -> bool:
                    if card and card.permanent_of_this_card() is None:
                        return False
                    return True
                return cond
            link_effect.set_can_use_condition(make_link_condition())

            def link_process(ctx: Dict[str, Any]):
                pass
            link_effect.set_on_process_callback(link_process)
            effects.append(link_effect)

        # --- When Digivolving: If DNA Digivolving, link up to 3 [Maquinamon] ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX11-073 You may link up to 3 [Maquinamon] from hand, trash or digivolution cards")
        effect4.set_effect_description(
            "[When Digivolving] If DNA digivolving, you may link up to 3 "
            "[Maquinamon] from your hand, trash or this Digimon's digivolution "
            "cards to this Digimon without paying the cost.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not context.get('is_dna_digivolve', False):
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Link up to 3 [Maquinamon] from hand, trash, or digi cards to THIS Digimon.
            Uses zone-choice selection like C# reference."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            from ....game.constants import SEL_TRASH_START, SEL_EFFECT_CHOICE_START
            from ....data.enums import GamePhase

            linked_so_far = []

            def link_next():
                if len(linked_so_far) >= 3:
                    return

                hand_candidates = [c for c in player.hand_cards
                                   if _is_maquinamon(c) and c not in linked_so_far]
                trash_candidates = [c for c in player.trash_cards
                                    if _is_maquinamon(c) and c not in linked_so_far]
                digi_candidates = []
                if perm and len(perm.card_sources) > 1:
                    digi_candidates = [c for c in perm.card_sources[:-1]
                                       if _is_maquinamon(c) and c not in linked_so_far]

                has_hand = len(hand_candidates) > 0
                has_trash = len(trash_candidates) > 0
                has_digi = len(digi_candidates) > 0

                if not (has_hand or has_trash or has_digi):
                    return

                # Count available zones
                zones = []
                zone_labels = []
                if has_hand:
                    zones.append('hand')
                    zone_labels.append("From hand")
                if has_trash:
                    zones.append('trash')
                    zone_labels.append("From trash")
                if has_digi:
                    zones.append('digi')
                    zone_labels.append("From digivolution cards")

                def on_zone_choice(branch: int):
                    if branch < 0 or branch >= len(zones):
                        return
                    zone = zones[branch]
                    if zone == 'hand':
                        _link_from_hand()
                    elif zone == 'trash':
                        _link_from_trash()
                    elif zone == 'digi':
                        _link_from_digi()

                game.effect_choose_branch(
                    player, len(zones), on_zone_choice,
                    prompt=f"Link a [Maquinamon] ({len(linked_so_far)+1}/3)? Choose zone.",
                    branch_labels=zone_labels)

            def _link_from_hand():
                def on_hand_selected(selected_card):
                    if selected_card and selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                        perm.link_card(selected_card)
                        linked_so_far.append(selected_card)
                        link_next()

                game.effect_select_hand_card(
                    player, lambda c: _is_maquinamon(c) and c not in linked_so_far,
                    on_hand_selected,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from hand to link.")

            def _link_from_trash():
                valid_trash = []
                for i, c in enumerate(player.trash_cards):
                    if _is_maquinamon(c) and c not in linked_so_far:
                        valid_trash.append(SEL_TRASH_START + i)
                if not valid_trash:
                    return

                def on_trash_action(action_id: int):
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        selected_card = player.trash_cards[idx]
                        player.trash_cards.remove(selected_card)
                        perm.link_card(selected_card)
                        linked_so_far.append(selected_card)
                        link_next()

                game.request_selection(
                    GamePhase.SelectTrash, player, on_trash_action,
                    valid_trash, is_optional=True,
                    prompt="Select a [Maquinamon] from trash to link.")

            def _link_from_digi():
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs is perm.top_card:
                        continue
                    if _is_maquinamon(cs) and cs not in linked_so_far:
                        valid.append(SEL_EFFECT_CHOICE_START + i)
                if not valid:
                    return

                def on_select_digi(action_id: int):
                    idx = action_id - SEL_EFFECT_CHOICE_START
                    if 0 <= idx < len(perm.card_sources):
                        cs = perm.card_sources[idx]
                        if cs in perm.card_sources:
                            perm.card_sources.remove(cs)
                        perm.link_card(cs)
                        linked_so_far.append(cs)
                        link_next()

                game.request_selection(
                    GamePhase.SelectEffectChoice, player, on_select_digi, valid,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from digivolution cards to link.")

            link_next()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- End of Opponent's Turn: trash security + bottom-deck Digimon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-073 Trash opponent security and bottom deck opponent digimon per link card")
        effect5.set_effect_description(
            "[End of Opponent's Turn] [Once Per Turn] For each of this "
            "Digimon's link cards, trash your opponent's top security card "
            "and return 1 of their Digimon to the bottom of the deck.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_073_EOOT_TRASH_BOUNCE")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """For each linked card, trash 1 opponent's security, then
            bottom-deck 1 opponent Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            link_count = len(perm.linked_cards) if perm else 0
            if link_count <= 0:
                return

            enemy = player.enemy if player else None
            if not enemy:
                return

            # Trash opponent's top security cards (1 per link card)
            for _ in range(link_count):
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(trashed)

            # Bottom-deck opponent Digimon (1 per link card)
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            bounce_count = min(link_count, len(opp_digimon))
            if bounce_count <= 0:
                return

            bounced_so_far = []

            def do_iteration():
                if len(bounced_so_far) >= bounce_count:
                    return
                remaining_opp = [p for p in enemy.battle_area if p.is_digimon]
                if not remaining_opp:
                    return

                def target_filter(p):
                    return p.is_digimon

                def on_bottom_deck(target_perm):
                    enemy.return_permanent_to_deck_bottom(target_perm)
                    bounced_so_far.append(target_perm)
                    do_iteration()

                game.effect_select_opponent_permanent(
                    player, on_bottom_deck, filter_fn=target_filter, is_optional=False,
                    prompt=f"Select opponent's Digimon to return to bottom of deck ({len(bounced_so_far)+1}/{bounce_count}).")

            do_iteration()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
