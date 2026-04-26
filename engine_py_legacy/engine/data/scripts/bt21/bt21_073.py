from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_073(CardScript):
    """BT21-073 Charismon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: with [Sup.] trait for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_trait = "Sup."

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Sup.' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-073 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-073 Link from trash or sources")
        effect2.set_effect_description("[On Play] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT21-073 Link from trash or sources")
        effect3.set_effect_description("[When Digivolving] You may link 1 level 4 or lower Digimon card from your trash or this Digimon's digivolution cards to this Digimon without paying the cost.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenLinked
        # Effect
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenLinked)
        effect4.set_effect_name("BT21-073 Taunt when linked")
        effect4.set_effect_description("Effect")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT21_073_tauntOnLink")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.WhenLinked
        # [Start of Your Main Phase] Attack with this Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenLinked)
        effect5.set_effect_name("BT21-073 Attack with this Digimon")
        effect5.set_effect_description("[Start of Your Main Phase] Attack with this Digimon.")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Force Attack, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Grant effect immunity via modifier system
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns][Once Per Turn] When this Digimon would leave the battle area, by trashing 1 of this Digimon's link cards, it doesn't leave.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.WhenRemoveField)
        effect6.set_effect_name("BT21-073 Trash a linked card to prevent this digimon from leaving")
        effect6.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area, by trashing 1 of this Digimon's link cards, it doesn't leave.")
        effect6.is_optional = True
        effect6.set_max_count_per_turn(1)
        effect6.set_hash_string("AllTurns_BT21_073")

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
