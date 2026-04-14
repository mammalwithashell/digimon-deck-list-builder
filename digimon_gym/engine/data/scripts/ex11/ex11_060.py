from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_060(CardScript):
    """EX11-060 Arisa Kinosaki (Tamer)

    [Start of Your Turn] If your memory is at 2 or less, it becomes 3.

    [All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted,
        by suspending this Tamer, <Draw 1>. If deleted by <Overclock>, you may
        play 1 level 4 or lower [Puppet] trait Digimon card from your hand
        without paying the cost.

    [Security] Play this card without paying its cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("EX11-060 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] Deletion trigger ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-060 Draw 1 on Token/Puppet deletion, Overclock play")
        effect1.set_effect_description("[All Turns] When Token/Puppet deleted, suspend to Draw 1. If Overclock, play Puppet Lv4-.")
        effect1.is_optional = True
        effect1._is_deletion_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check that this tamer is not already suspended (cost: suspend this)
            my_perm = card.permanent_of_this_card()
            if my_perm and my_perm.is_suspended:
                return False
            # Check that the deleted permanent belongs to this tamer's owner
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # In _fire_deletion_observers, 'player' is the owner of the deleted permanent.
            # Verify the deleted permanent's owner matches this card's owner.
            if context.get('player') is not player:
                return False
            # Check Token or Puppet trait
            is_token = getattr(deleted_perm, 'is_token', False)
            is_puppet = False
            if deleted_perm.top_card:
                is_puppet = _is_puppet_trait(deleted_perm.top_card)
            if not (is_token or is_puppet):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return
            # Cost: suspend this Tamer
            my_perm.suspend()
            # Draw 1
            player.draw_cards(1)
            # If deleted by Overclock, may play 1 level 4 or lower Puppet from hand
            removal_cause = ctx.get('removal_cause', '')
            is_overclock = (removal_cause == 'overclock')

            if is_overclock:
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    level = getattr(c, 'level', None)
                    if level is None or level > 4:
                        return False
                    return _is_puppet_trait(c)
                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True,
                    prompt="You may play 1 level 4 or lower [Puppet] Digimon from hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Security - Play this card ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX11-060 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
