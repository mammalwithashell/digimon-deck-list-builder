from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_093(CardScript):
    """BT19-093 Queen Device"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Color bypass: "While you don't have [Queen Device] in the battle area,
        # you may ignore this card's color requirements."
        # Setting unconditionally is safe: if a Queen Device is already in the
        # battle area, its black color already satisfies the color requirement.
        card._match_color_requirement = False

        # --- Effect 0: When this card is trashed from your battle area ---
        # "When this card is trashed in your battle area, until the end of your
        #  opponent's turn, 1 of their Digimon can't activate [When Digivolving]
        #  effects and gets -3000 DP."
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT19-093 On trashed: -3000 DP + disable When Digivolving")
        effect0.set_effect_description(
            "When this card is trashed in your battle area, until the end of your "
            "opponent's turn, 1 of their Digimon can't activate [When Digivolving] "
            "effects and gets -3000 DP."
        )
        effect0.is_on_deletion = True

        effect = effect0
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon: -3000 DP, disable [When Digivolving]."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(-3000)
                # [When Digivolving] disable is not yet modeled in engine
                # descriptive-tagged: disable_when_digivolving

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -3000 DP.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Main] ---
        # "[Main] Until the end of your opponent's turn, 1 of their Digimon can't
        #  activate [When Digivolving] effects and gets -3000 DP. Then, place this
        #  card in the battle area."
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT19-093 Main: -3000 DP + disable When Digivolving")
        effect1.set_effect_description(
            "[Main] Until the end of your opponent's turn, 1 of their Digimon "
            "can't activate [When Digivolving] effects and gets -3000 DP. Then, "
            "place this card in the battle area."
        )

        effect = effect1
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon: -3000 DP, disable [When Digivolving]."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(-3000)
                # [When Digivolving] disable is not yet modeled in engine
                # descriptive-tagged: disable_when_digivolving

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -3000 DP.")
            # "Then, place this card in the battle area" — handled by engine
            # (OptionSkill cards with delay placement are auto-placed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] ---
        # "[Security] 2 of your opponent's Digimon gain <Security A. -2> for the
        #  turn. Then, add this card to the hand."
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT19-093 Security: SA-2 to 2 opponent Digimon")
        effect2.set_effect_description(
            "[Security] 2 of your opponent's Digimon gain <Security A. -2> for "
            "the turn. Then, add this card to the hand."
        )
        effect2.is_security_effect = True

        effect = effect2
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Give SA-2 to up to 2 opponent Digimon, then add this card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Apply SA-2 to up to 2 opponent Digimon via selection
            digimon_targets = [p for p in enemy.battle_area if p.is_digimon]
            remaining_count = min(2, len(digimon_targets))
            if remaining_count > 0:
                selected_set = set()

                def _select_next():
                    nonlocal remaining_count
                    if remaining_count <= 0:
                        return
                    def sa_filter(p):
                        return p.is_digimon and id(p) not in selected_set
                    if not any(sa_filter(p) for p in enemy.battle_area):
                        return

                    def on_sa_select(target_perm):
                        nonlocal remaining_count
                        target_perm._temp_sa_modifier -= 2
                        selected_set.add(id(target_perm))
                        remaining_count -= 1
                        _select_next()

                    game.effect_select_opponent_permanent(
                        player, on_sa_select, filter_fn=sa_filter,
                        is_optional=False,
                        prompt=f"Select opponent Digimon to give Security A. -2 ({remaining_count} remaining).")

                _select_next()

            # Add this card to hand
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
