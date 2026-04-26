from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_071(CardScript):
    """BT24-071 Raidramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-071 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # Shared On Play / When Digivolving logic:
        # Select 1 of your Digimon with [System], [Life], or [Transmutation]
        # trait and grant it Security Attack +1 for the turn.
        # ---------------------------------------------------------------

        def _sa_plus_1_process(ctx: Dict[str, Any]):
            """Grant SA+1 to 1 of your Digimon with System/Life/Transmutation trait."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType

            def sa_filter(p):
                if not p.is_digimon:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return any(t in tr for tr in traits for t in ('System', 'Life', 'Transmutation'))

            def on_select(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_SECURITY_ATTACK,
                    value_fn=lambda cur, t, c: cur + 1,
                    expiry='end_of_turn',
                )

            game.effect_select_own_permanent(
                player, on_select, filter_fn=sa_filter, is_optional=False,
                prompt="Select 1 of your Digimon with System/Life/Transmutation trait to give Security A. +1.")

        # On Play — SA+1
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-071 Give SA+1 to traited Digimon")
        effect1.set_effect_description("[On Play] 1 of your Digimon with the [System], [Life] or [Transmutation] trait gains <Security A. +1> for the turn.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_sa_plus_1_process)
        effects.append(effect1)

        # When Digivolving — SA+1
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-071 Give SA+1 to traited Digimon")
        effect2.set_effect_description("[When Digivolving] 1 of your Digimon with the [System], [Life] or [Transmutation] trait gains <Security A. +1> for the turn.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_sa_plus_1_process)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # Shared On Deletion / Linked On Deletion logic:
        # Play 1 level 3 [Appmon] trait Digimon from trash without paying cost.
        # ---------------------------------------------------------------

        def _on_deletion_play_process(ctx: Dict[str, Any]):
            """Play 1 level 3 Appmon trait Digimon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 3:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Appmon' in tr for tr in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        # On Deletion — play Lv.3 Appmon from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT24-071 Play 1 level 3 Appmon from trash")
        effect3.set_effect_description("[On Deletion] You may play 1 level 3 [Appmon] trait Digimon card from your trash without paying the cost.")
        effect3.is_on_deletion = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_on_deletion_play_process)
        effects.append(effect3)

        # Linked On Deletion — play Lv.3 Appmon from trash (inherited link effect)
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT24-071 Play 1 level 3 Appmon from trash")
        effect4.set_effect_description("[On Deletion] You may play 1 level 3 [Appmon] trait Digimon card from your trash without paying the cost.")
        effect4.is_on_deletion = True
        effect4.is_optional = True
        effect4.is_linked_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_on_deletion_play_process)
        effects.append(effect4)

        return effects
