from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_060(CardScript):
    """EX7-060 Nidhoggmon | Lv.6 Purple Dark Dragon

    [Trash] [Main] If you have 4 or fewer cards in your hand, you may play
        this card from your trash with the play cost reduced by 4.
    <Blocker>
    [On Deletion] You may play 1 level 5 or lower Digimon card with the
        [Dark Dragon] or [Evil Dragon] trait from your trash without paying
        the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Trash] [Main] Play self from trash with cost reduced by 4 ---
        # Uses OnDeclaration timing for hand_main-style trash activation
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("EX7-060 [Trash][Main] Play from trash cost -4")
        effect0.set_effect_description(
            "[Trash] [Main] If you have 4 or fewer cards in your hand, you may "
            "play this card from your trash with the play cost reduced by 4."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be own turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be in trash
            owner = card.owner
            if card not in owner.trash_cards:
                return False
            # Must have 4 or fewer cards in hand
            if len(owner.hand_cards) > 4:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Play self from trash with pay_cost=True; the BeforePayCost
            # effect (effect1) provides the -4 cost reduction
            if card in player.trash_cards:
                player.play_card_from_source(card, pay_cost=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Cost reduction for Trash Main ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("EX7-060 Trash Main cost -4")
        effect1.set_effect_description("Reduce play cost by 4 when played from trash.")
        effect1.cost_reduction = 4

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False  # LEAK GUARD
            owner = card.owner if card else None
            if not owner:
                return False
            # Only applies when hand <= 4
            if len(owner.hand_cards) > 4:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Blocker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX7-060 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [On Deletion] Play lv5- Dark Dragon/Evil Dragon from trash free ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX7-060 On Deletion: play Dark/Evil Dragon from trash")
        effect3.set_effect_description(
            "[On Deletion] You may play 1 level 5 or lower Digimon card with the "
            "[Dark Dragon] or [Evil Dragon] trait from your trash without paying "
            "the cost."
        )
        effect3.is_on_deletion = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Dark Dragon' in t or 'Evil Dragon' in t for t in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True,
                prompt="Play 1 level 5 or lower [Dark Dragon] or [Evil Dragon] Digimon from trash."
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
