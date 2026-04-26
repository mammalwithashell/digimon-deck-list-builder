from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_059(CardScript):
    """BT20-059 Gankoomon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-059 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Gankoomon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Gankoomon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Gankoomon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if [Gankoomon] or [X Antibody] is in this Digimon's digivolution cards, until the end of your opponent's turn, none of your Digimon are affected by your opponent's Digimon effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT20-059 <De-Digivolve 2>, then your Digimon or unaffected by Digimon effects")
        effect1.set_effect_description("[When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon. Then, if [Gankoomon] or [X Antibody] is in this Digimon's digivolution cards, until the end of your opponent's turn, none of your Digimon are affected by your opponent's Digimon effects.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # De-Digivolve 2 on 1 opponent Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Check condition: [Gankoomon] or [X Antibody] in digi cards
            if perm:
                has_gankoomon_or_x = False
                for cs in perm.card_sources:
                    names = getattr(cs, 'card_names', []) or []
                    if any('Gankoomon' in n or 'X Antibody' in n for n in names):
                        has_gankoomon_or_x = True
                        break
                if has_gankoomon_or_x:
                    # Grant CANNOT_BE_AFFECTED to ALL own Digimon until end of opponent's turn
                    from ....interfaces.modifiers import ModifierType
                    for own_perm in player.battle_area:
                        if own_perm.is_digimon:
                            game.register_modifier(
                                own_perm, ModifierType.CANNOT_BE_AFFECTED,
                                expiry='end_of_opponent_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Opponent's Turn] Your Digimon with [Sistermon] or [Huckmon] in name,
        # or [Royal Knight] trait gain <Blocker> and <Reboot>.
        # Implemented as aura: _applies_to_all_own_digimon + _keyword_permanent_condition
        def _gankoomon_aura_filter(target_perm):
            """Check if target has Sistermon/Huckmon name or Royal Knight trait."""
            if target_perm.top_card:
                names = getattr(target_perm.top_card, 'card_names', []) or []
                if any('Sistermon' in n or 'Huckmon' in n for n in names):
                    return True
                traits = getattr(target_perm.top_card, 'card_traits', []) or []
                if any('Royal Knight' in tr for tr in traits):
                    return True
            return False

        # Blocker aura (opponent's turn)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-059 Blocker aura (Sistermon/Huckmon/RK)")
        effect2.set_effect_description("[Opponent's Turn] Sistermon/Huckmon/RK Digimon gain Blocker.")
        effect2._is_blocker = True
        effect2._applies_to_all_own_digimon = True
        effect2._keyword_permanent_condition = _gankoomon_aura_filter

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Active during opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Reboot aura (opponent's turn) — self
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-059 Reboot aura (Sistermon/Huckmon/RK)")
        effect3.set_effect_description("[Opponent's Turn] Sistermon/Huckmon/RK Digimon gain Reboot.")
        effect3._is_reboot = True
        effect3._applies_to_all_own_digimon = True
        effect3._keyword_permanent_condition = _gankoomon_aura_filter

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # [Inherited] [Opponent's Turn] While this Digimon is [Jesmon GX],
        # Sistermon/Huckmon/RK Digimon gain Blocker and Reboot.
        # Blocker aura (inherited, Jesmon GX host)
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-059 [Inherited] Blocker aura (Jesmon GX)")
        effect5.set_effect_description("[Inherited] While Jesmon GX, Sistermon/Huckmon/RK gain Blocker.")
        effect5.is_inherited_effect = True
        effect5._is_blocker = True
        effect5._applies_to_all_own_digimon = True
        effect5._keyword_permanent_condition = _gankoomon_aura_filter

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card()
            if not (permanent and (permanent.contains_card_name('Jesmon GX'))):
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Reboot aura (inherited, Jesmon GX host)
        effect6 = ICardEffect()
        effect6.set_effect_name("BT20-059 [Inherited] Reboot aura (Jesmon GX)")
        effect6.set_effect_description("[Inherited] While Jesmon GX, Sistermon/Huckmon/RK gain Reboot.")
        effect6.is_inherited_effect = True
        effect6._is_reboot = True
        effect6._applies_to_all_own_digimon = True
        effect6._keyword_permanent_condition = _gankoomon_aura_filter

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card()
            if not (permanent and (permanent.contains_card_name('Jesmon GX'))):
                return False
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
