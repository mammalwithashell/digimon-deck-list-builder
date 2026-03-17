from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_019(CardScript):
    """BT20-019 Jesmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-019 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Jesmon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Jesmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Jesmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alliance
        # Alliance
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-019 Alliance")
        effect1.set_effect_description("Alliance")
        effect1.is_on_attack = True
        effect1._is_alliance = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If [Jesmon] or [X Antibody] is in this Digimon's digivolution cards, for the turn, 1 of your Digimon isn't affected by your opponent's effects. Then, 1 of your Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-019 Become unaffected by opponents effects, then can attack")
        effect2.set_effect_description("[When Digivolving] If [Jesmon] or [X Antibody] is in this Digimon's digivolution cards, for the turn, 1 of your Digimon isn't affected by your opponent's effects. Then, 1 of your Digimon may attack.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Effect Immunity, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Check condition: Jesmon or X Antibody in digivolution cards
            has_jesmon_or_x = False
            for cs in perm.card_sources:
                if cs is not perm.top_card:
                    for n in getattr(cs, 'card_names', []):
                        if 'Jesmon' in n or 'X Antibody' in n:
                            has_jesmon_or_x = True
                            break
            if not has_jesmon_or_x:
                return
            # "1 of your Digimon isn't affected by your opponent's effects"
            def own_filter(p):
                return p.is_digimon
            def on_immunity_target(target_perm):
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_BE_SELECTED_BY_EFFECT,
                    value_fn=lambda: True, expiry='end_of_turn')
                # "Then, 1 of your Digimon may attack"
                def on_attack_target(atk_perm):
                    game.register_modifier(
                        atk_perm, ModifierType.FORCE_ATTACK,
                        value_fn=lambda: True, expiry='end_of_turn')
                game.effect_select_own_permanent(
                    player, on_attack_target, filter_fn=own_filter,
                    is_optional=True,
                    prompt="Select 1 of your Digimon that may attack.")
            game.effect_select_own_permanent(
                player, on_immunity_target, filter_fn=own_filter,
                is_optional=False,
                prompt="Select 1 of your Digimon to be unaffected by opponent's effects.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Your Turn] Your Digimon with [Sistermon] in their names or the
        # [Royal Knight] trait gain <Piercing>.
        # Aura pattern: _applies_to_all_own_digimon + _keyword_permanent_condition
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-019 Sistermon/Royal Knight gain Piercing")
        effect3.set_effect_description("[Your Turn] Your Digimon with [Sistermon] or [Royal Knight] gain <Piercing>.")
        effect3._is_piercing = True
        effect3._applies_to_all_own_digimon = True
        def _piercing_perm_filter(target_perm):
            """Check if target has Sistermon name or Royal Knight trait."""
            if target_perm.top_card:
                names = getattr(target_perm.top_card, 'card_names', []) or []
                if any('Sistermon' in n for n in names):
                    return True
                traits = getattr(target_perm.top_card, 'card_traits', []) or []
                if any('Royal Knight' in tr for tr in traits):
                    return True
            return False
        effect3._keyword_permanent_condition = _piercing_perm_filter

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # [Your Turn] Your Digimon with [Sistermon] or [Royal Knight] trait
        # can also attack your opponent's unsuspended Digimon.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-019 Sistermon/Royal Knight attack unsuspended")
        effect4.set_effect_description("[Your Turn] Sistermon/RK Digimon can attack unsuspended.")
        effect4._is_can_attack_unsuspended = True
        effect4._applies_to_all_own_digimon = True
        effect4._keyword_permanent_condition = _piercing_perm_filter

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # [Inherited] [Your Turn] While this Digimon is [Jesmon GX], all of your
        # Digimon gain <Piercing> and can also attack opponent's unsuspended Digimon.
        # Piercing aura
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-019 [Inherited] Jesmon GX: all Digimon gain Piercing")
        effect5.set_effect_description("[Your Turn] While Jesmon GX, all Digimon gain Piercing.")
        effect5.is_inherited_effect = True
        effect5._is_piercing = True
        effect5._applies_to_all_own_digimon = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = card.permanent_of_this_card()
            if not (permanent and (permanent.contains_card_name('Jesmon GX'))):
                return False
            return True

        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Attack Unsuspended aura (inherited)
        effect6 = ICardEffect()
        effect6.set_effect_name("BT20-019 [Inherited] Jesmon GX: all Digimon attack unsuspended")
        effect6.set_effect_description("[Your Turn] While Jesmon GX, all Digimon can attack unsuspended.")
        effect6.is_inherited_effect = True
        effect6._is_can_attack_unsuspended = True
        effect6._applies_to_all_own_digimon = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = card.permanent_of_this_card()
            if not (permanent and (permanent.contains_card_name('Jesmon GX'))):
                return False
            return True

        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
