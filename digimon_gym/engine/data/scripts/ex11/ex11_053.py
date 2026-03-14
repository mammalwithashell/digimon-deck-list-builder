from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_053(CardScript):
    """EX11-053 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['X Antibody']

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-053 Also treated as [X Antibody]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Draw 1
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-053 By placing a [Royal Knight] under any of your [King Drasil_7D6]s, Draw 1")
        effect1.set_effect_description("Draw 1")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check if player has any King Drasil_7D6 in battle area or breeding
            has_king_drasil = False
            for p in player.battle_area:
                if p.contains_card_name('King Drasil_7D6'):
                    has_king_drasil = True
                    break
            if not has_king_drasil and player.breeding_area:
                if player.breeding_area.contains_card_name('King Drasil_7D6'):
                    has_king_drasil = True
            if not has_king_drasil:
                return False
            # Check if player has a Royal Knight trait card in hand
            has_rk_in_hand = any(
                any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or []))
                for c in player.hand_cards
            )
            return has_rk_in_hand

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Place Royal Knight from hand under King Drasil, then Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: select a Royal Knight trait card from hand and place under King Drasil
            def hand_filter(c):
                return any('Royal Knight' in t for t in (getattr(c, 'card_traits', []) or []))
            def on_select(selected_card):
                # Find King Drasil permanent
                king_drasil = None
                for p in player.battle_area:
                    if p.contains_card_name('King Drasil_7D6'):
                        king_drasil = p
                        break
                if not king_drasil and player.breeding_area:
                    if player.breeding_area.contains_card_name('King Drasil_7D6'):
                        king_drasil = player.breeding_area
                if king_drasil and selected_card:
                    player.hand_cards.remove(selected_card)
                    king_drasil.add_card_source_bottom(selected_card)
                    # Reward: draw 1
                    player.draw_cards(1)
            game.effect_select_hand_card(
                player, hand_filter, on_select, is_optional=True,
                prompt="Select a [Royal Knight] card to place under King Drasil.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] If you have 1 or fewer security cards, you may play 1
        # [Omnimon (X Antibody)] from your hand or under your [King Drasil_7D6]s
        # on the field without paying the cost. Then, place this card as the
        # played Digimon's bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX11-053 Play 1 [Omnimon (X Antibody)] and place this card under it.")
        effect2.set_effect_description(
            "[On Deletion] If you have 1 or fewer security cards, you may play 1 "
            "[Omnimon (X Antibody)] from your hand or under your [King Drasil_7D6]s "
            "on the field without paying the cost. Then, place this card as the "
            "played Digimon's bottom digivolution card."
        )
        effect2.is_on_deletion = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            # Condition: 1 or fewer security cards
            if len(player.security_cards) > 1:
                return False
            # Must have an Omnimon (X Antibody) in hand OR under a King Drasil_7D6 on the field
            def is_omnimon_xab(c):
                return any('Omnimon (X Antibody)' in (n or '') for n in (getattr(c, 'card_names', []) or []))
            has_target = any(is_omnimon_xab(c) for c in player.hand_cards)
            if not has_target:
                for p in player.battle_area:
                    if p.contains_card_name('King Drasil_7D6'):
                        # Search digivolution cards (card_sources[:-1])
                        digi_sources = p.card_sources[:-1] if len(p.card_sources) > 1 else []
                        if any(is_omnimon_xab(s) for s in digi_sources):
                            has_target = True
                            break
            return has_target

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Omnimon (X Antibody) from hand or King Drasil sources, then place this card under it."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def is_omnimon_xab(c):
                return any('Omnimon (X Antibody)' in (n or '') for n in (getattr(c, 'card_names', []) or []))

            # Build candidate list: Omnimon (X Antibody) from hand
            candidates_hand = [c for c in player.hand_cards if is_omnimon_xab(c)]

            # Omnimon (X Antibody) from under King Drasil_7D6s on field
            candidates_drasil = []
            for p in player.battle_area:
                if p.contains_card_name('King Drasil_7D6'):
                    digi_sources = list(p.card_sources[:-1]) if len(p.card_sources) > 1 else []
                    for s in digi_sources:
                        if is_omnimon_xab(s):
                            candidates_drasil.append((p, s))

            if not candidates_hand and not candidates_drasil:
                return

            def on_played(played_perm):
                """After Omnimon is played, place this (deleted) Omekamon card under it."""
                if played_perm is not None:
                    played_perm.add_card_source_bottom(card)
                    # Remove from trash if it was sent there on deletion
                    if card in player.trash_cards:
                        player.trash_cards.remove(card)

            if candidates_hand:
                # Play from hand
                def hand_filter(c):
                    return is_omnimon_xab(c)

                def on_hand_select(selected_card):
                    if selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                    played_perm = player.play_card_from_source(selected_card, pay_cost=False)
                    if played_perm:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": selected_card, "played_permanent": played_perm,
                             "event_player": player},
                        )
                        on_played(played_perm)

                game.effect_select_hand_card(
                    player, hand_filter, on_hand_select, is_optional=True,
                    prompt="Select [Omnimon (X Antibody)] from your hand to play.")
            elif candidates_drasil:
                # Play from under King Drasil_7D6
                source_perm, source_card = candidates_drasil[0]
                source_perm.card_sources.remove(source_card)
                played_perm = player.play_card_from_source(source_card, pay_cost=False)
                if played_perm:
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": source_card, "played_permanent": played_perm,
                         "event_player": player},
                    )
                    on_played(played_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
