from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_086(CardScript):
    """BT13-086 Gizmon: XT | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ---------------------------------------------------------------
        # BeforePayCost: When you would play this card, by deleting 1 of
        # your level 4 Digimon, reduce the play cost by 6.
        # The cost_reduction=6 on effect0 is conditional on deleting a Lv.4.
        # ---------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT13-086 Delete your 1 level 4 Digimon to get Play Cost -6")
        effect0.set_effect_description("When you would play this card, by deleting 1 of your level 4 Digimon, reduce the play cost by 6.")
        effect0.is_optional = True
        effect0.set_hash_string("PlayCost-6_BT13_086")
        effect0.cost_reduction = 6

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Delete own Lv.4 Digimon as cost for -6 play cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def lv4_filter(p):
                if not p.is_digimon:
                    return False
                if getattr(p, 'level', None) != 4:
                    return False
                return True

            def on_lv4_selected(target_perm):
                player.delete_permanent(target_perm)

            game.effect_select_own_permanent(
                player, on_lv4_selected, filter_fn=lv4_filter, is_optional=True,
                prompt="Select 1 of your level 4 Digimon to delete for play cost -6.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # NOTE: The C# has a second static ChangeCostClass at EffectTiming.None
        # that shows the -6 availability in the UI. In the engine, the
        # BeforePayCost effect above handles the actual cost reduction.
        # We do NOT add a duplicate unconditional cost_reduction=6 static effect
        # as that would bypass the "by deleting Lv.4" cost requirement.

        # Factory effect: blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-086 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # On Play: Play 1 [Akihiro Kurata] from trash without paying cost.
        # ---------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-086 Play 1 [Akihiro Kurata] from trash")
        effect2.set_effect_description("[On Play] Play 1 [Akihiro Kurata] from your trash without paying the cost.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 Akihiro Kurata from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return 'Akihiro Kurata' in names or 'AkihiroKurata' in names

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # Can't Digivolve (static — register modifier when on field)
        # ---------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_effect_name("BT13-086 Can't digivolve")
        effect3.set_effect_description("This Digimon can't digivolve.")
        effect3.is_declarative = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Register CANNOT_DIGIVOLVE modifier on self."""
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if game and perm:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm,
                    ModifierType.CANNOT_DIGIVOLVE,
                    source_effect=effect3,
                    expiry='permanent',
                )
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ---------------------------------------------------------------
        # On Deletion: Play 1 [ProtoGizmon] from trash without paying cost.
        # ---------------------------------------------------------------
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT13-086 Play 1 [ProtoGizmon] from trash")
        effect4.set_effect_description("[On Deletion] You may play 1 [ProtoGizmon] from your trash without paying the cost.")
        effect4.is_optional = True
        effect4.is_on_deletion = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Play 1 ProtoGizmon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return 'ProtoGizmon' in names

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
