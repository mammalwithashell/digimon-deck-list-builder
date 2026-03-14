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
    Link: Up to 3 [Maquinamon]

    [When Digivolving] If DNA Digivolving, you may link up to 3 [Maquinamon]
    from your hand, trash, or this Digimon's digivolution cards to this
    Digimon without paying the cost.

    [End of Opponent's Turn] [Once Per Turn] For each of this Digimon's link
    cards, trash your opponent's top security card and return 1 of their
    Digimon to the bottom of the deck.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

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

        # --- Link slot markers (up to 3 links) ---
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
        effect4.set_effect_description("[When Digivolving] If DNA Digivolving, you may link up to 3 [Maquinamon] from your hand, trash or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be DNA digivolving
            if not context.get('is_dna_digivolve', False):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Link up to 3 [Maquinamon] cards from hand, trash, or digi cards to THIS Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def is_maquinamon(c):
                names = getattr(c, 'card_names', []) or []
                return any('Maquinamon' in n for n in names)

            links_remaining = [3]

            def link_next():
                if links_remaining[0] <= 0:
                    return
                # Gather candidates from hand
                hand_candidates = [
                    c for c in player.hand_cards if is_maquinamon(c)
                ]
                # Gather candidates from trash
                trash_candidates = [
                    c for c in player.trash_cards if is_maquinamon(c)
                ]
                # Gather candidates from this permanent's digivolution cards (excluding top)
                digi_candidates = []
                if perm and len(perm.card_sources) > 1:
                    digi_candidates = [
                        c for c in perm.card_sources[:-1] if is_maquinamon(c)
                    ]

                if not (hand_candidates or trash_candidates or digi_candidates):
                    return

                # Prefer hand selection since we have an API for it
                if hand_candidates:
                    def on_hand_select(selected_card):
                        if selected_card in player.hand_cards:
                            player.hand_cards.remove(selected_card)
                        perm.link_card(selected_card)
                        links_remaining[0] -= 1
                        link_next()

                    game.effect_select_hand_card(
                        player,
                        filter_fn=is_maquinamon,
                        callback=on_hand_select,
                        is_optional=True,
                        prompt="Select a [Maquinamon] card from hand to link.")
                elif trash_candidates:
                    tc = trash_candidates[0]
                    if tc in player.trash_cards:
                        player.trash_cards.remove(tc)
                        perm.link_card(tc)
                        links_remaining[0] -= 1
                        link_next()
                elif digi_candidates:
                    dc = digi_candidates[0]
                    if dc in perm.card_sources:
                        perm.card_sources.remove(dc)
                        perm.link_card(dc)
                        links_remaining[0] -= 1
                        link_next()

            link_next()

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- End of Opponent's Turn: trash OPPONENT's security + bottom-deck opponent Digimon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-073 Trash opponent security and bottom deck opponent digimon per link card")
        effect5.set_effect_description("[End of Opponent's Turn] [Once Per Turn] For each of this Digimon's link cards, trash your opponent's top security card and return 1 of their Digimon to the bottom of the deck.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_073_EOOT_TRASH_BOUNCE")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # End of opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """For each linked card, trash 1 OPPONENT's security, then bottom-deck 1 opponent Digimon."""
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
            security_trashed = 0
            for _ in range(link_count):
                if enemy.security_cards:
                    trashed = enemy.security_cards.pop()
                    enemy.trash_cards.append(trashed)
                    security_trashed += 1

            # Bottom-deck opponent Digimon (1 per link card)
            iterations_left = [min(link_count, len([p for p in enemy.battle_area if p.is_digimon]))]

            def do_iteration():
                if iterations_left[0] <= 0:
                    return
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not opp_digimon:
                    return
                iterations_left[0] -= 1

                def target_filter(p):
                    return p.is_digimon

                def on_bottom_deck(target_perm):
                    enemy.return_permanent_to_deck_bottom(target_perm)
                    do_iteration()

                game.effect_select_opponent_permanent(
                    player, on_bottom_deck, filter_fn=target_filter, is_optional=False)

            do_iteration()

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
