from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_034(CardScript):
    """BT18-034 Lucemon | Lv.3

    [Start of Your Main Phase] [On Play] By trashing 1 card in your hand,
        your opponent may trash their top security card. If this effect didn't
        trash, Recovery +1 (Deck).
    [End of Your Turn] By placing 1 of your level 6 Digimon on top of your
        security stack, this Digimon may digivolve into [Lucemon: Chaos Mode]
        in your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] [On Play]
        # By trashing 1 card in hand, opponent may trash their top security.
        # If this effect didn't trash (opponent security), Recovery +1.
        def _build_trash_effect(is_on_play=False, is_start_main=False):
            effect = ICardEffect()
            if is_on_play:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_on_play = True
            else:
                effect.set_timing(EffectTiming.StartMainPhase)
            effect.set_effect_name("BT18-034 Trash hand card, opponent may trash security")
            effect.set_effect_description(
                "By trashing 1 card in your hand, your opponent may trash their "
                "top security card. If this effect didn't trash, Recovery +1 (Deck)."
            )
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                player = card.owner if card else None
                if not player or not player.hand_cards:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                if not player.hand_cards:
                    return

                # Cost: trash 1 card in hand
                def hand_filter(c):
                    return True

                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)

                    # Opponent may trash their top security
                    enemy = player.enemy
                    if enemy and enemy.security_cards:
                        # Simplified: opponent auto-decides (50/50 or always yes for AI)
                        # In engine, opponent decision is approximated as auto-trash
                        trashed_sec = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed_sec)
                    else:
                        # Opponent didn't trash (no security) -> Recovery +1
                        player.recovery(1)

                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_trash_effect(is_on_play=True))
        effects.append(_build_trash_effect(is_start_main=True))

        # [End of Your Turn] By placing 1 of your level 6 Digimon on top of your
        # security stack, this Digimon may digivolve into [Lucemon: Chaos Mode]
        # in your trash without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.EndOfTurn)
        effect2.set_effect_name("BT18-034 End of Turn: place Lv6, digivolve into Chaos Mode")
        effect2.set_effect_description(
            "[End of Your Turn] By placing 1 of your level 6 Digimon on top of "
            "your security stack, this Digimon may digivolve into "
            "[Lucemon: Chaos Mode] in your trash without paying the cost."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: place 1 of your level 6 Digimon on top of security
            def target_filter(p):
                return p.is_digimon and p is not perm and p.level is not None and p.level == 6

            def on_place_security(target_perm):
                if target_perm in player.battle_area:
                    player.put_permanent_to_security(target_perm)

                    # Digivolve into Lucemon: Chaos Mode from trash
                    cur_perm = card.permanent_of_this_card() if card else None
                    if not cur_perm:
                        return

                    def digi_filter(c):
                        if not getattr(c, 'is_digimon', False):
                            return False
                        names = getattr(c, 'card_names', []) or []
                        return any('Lucemon: Chaos Mode' in n or 'Lucemon Chaos Mode' in n for n in names)

                    # Play from trash as digivolve
                    chaos_cards = [c for c in player.trash_cards if digi_filter(c)]
                    if chaos_cards:
                        digi_card = chaos_cards[0]
                        player.trash_cards.remove(digi_card)
                        cur_perm.card_sources.append(digi_card)

            has_targets = any(target_filter(p) for p in player.battle_area)
            if has_targets:
                game.effect_select_own_permanent(
                    player, on_place_security, filter_fn=target_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
