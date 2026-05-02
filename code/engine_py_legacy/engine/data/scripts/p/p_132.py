from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Own-field selection indices: 100-113, opponent field: 114-127
_SEL_MY_FIELD_START = 100
_SEL_OPP_FIELD_START = 114


class P_132(CardScript):
    """P-132 Galemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: [When Digivolving] By suspending 1 Digimon, this Digimon gets +2000 DP
        # until the end of your opponent's turn.
        # The suspend is optional — if you don't suspend, no DP bonus.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "P-132 [When Digivolving] By suspending 1 Digimon, +2000 DP until end of opponent's turn"
        )
        effect0.set_effect_description(
            "[When Digivolving] By suspending 1 Digimon, this Digimon gets +2000 DP until the "
            "end of your opponent's turn."
        )
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Optionally suspend 1 Digimon (any). If you do, this Digimon gets +2000 DP."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            from ....data.enums import GamePhase

            opponent = player.enemy
            own_area = player.battle_area
            opp_area = opponent.battle_area if opponent else []

            # Build combined valid list: own + opponent Digimon
            valid = []
            for i, p in enumerate(own_area):
                if p.is_digimon:
                    valid.append(_SEL_MY_FIELD_START + i)
            for i, p in enumerate(opp_area):
                if p.is_digimon:
                    if game.modifiers.can_be_selected_by_effect(p):
                        valid.append(_SEL_OPP_FIELD_START + i)

            if not valid:
                return

            def on_select(action_id: int):
                target_perm = None
                if _SEL_MY_FIELD_START <= action_id < _SEL_MY_FIELD_START + len(own_area):
                    idx = action_id - _SEL_MY_FIELD_START
                    if 0 <= idx < len(own_area):
                        target_perm = own_area[idx]
                elif _SEL_OPP_FIELD_START <= action_id < _SEL_OPP_FIELD_START + len(opp_area):
                    idx = action_id - _SEL_OPP_FIELD_START
                    if 0 <= idx < len(opp_area):
                        target_perm = opp_area[idx]

                if target_perm:
                    target_perm.suspend()
                    # Grant +2000 DP to this Digimon until end of opponent's turn
                    if perm and game:
                        from ....interfaces.modifiers import ModifierType
                        game.register_modifier(
                            ModifierType.CHANGE_DP, perm,
                            value_fn=lambda: 2000,
                            expiry='end_of_opponent_turn'
                        )

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="You may suspend 1 Digimon to give this Digimon +2000 DP."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1: [Your Turn] While you have [Shoto Kazama], this Digimon gains <Piercing>.
        # This is a continuous/static effect — implemented as NoTiming with _is_piercing
        # conditioned on Shoto Kazama presence.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-132 [Your Turn] While you have Shoto Kazama, gains Piercing")
        effect1.set_effect_description(
            "[Your Turn] While you have [Shoto Kazama], this Digimon gains <Piercing>."
        )
        effect1._is_piercing = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have Shoto Kazama tamer on field
            player_owner = card.owner if card else None
            if player_owner is None:
                return False
            has_shoto = any(
                p.is_tamer and p.contains_card_name('Shoto Kazama')
                for p in player_owner.battle_area
            )
            return has_shoto

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Effect 2 (Inherited): [Your Turn] This Digimon gets +2000 DP.
        effect2 = ICardEffect()
        effect2.set_effect_name("P-132 Inherited: +2000 DP")
        effect2.set_effect_description("[Your Turn] This Digimon gets +2000 DP.")
        effect2.is_inherited_effect = True
        effect2.dp_modifier = 2000

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
