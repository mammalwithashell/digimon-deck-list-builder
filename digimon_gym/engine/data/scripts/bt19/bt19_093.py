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
        # DCGO: IgnoreColorConditionClass with condition checking that NO Queen Device
        # is currently on the owner's field.
        def _color_bypass_fn() -> bool:
            owner = card.owner if card else None
            if not owner:
                return True  # no owner context => bypass
            for perm in owner.battle_area:
                if perm.contains_card_name("Queen Device"):
                    return True  # Queen Device already on field => enforce color
            return False  # no Queen Device on field => bypass color

        card._match_color_requirement_fn = _color_bypass_fn

        # --- Effect 0: When this card is trashed from your battle area ---
        # "When this card is trashed from the battle area, until the end of your
        #  opponent's turn, 1 of their Digimon can't activate [When Digivolving]
        #  effects and gets -3000 DP."
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDestroyedAnyone)
        effect0.set_effect_name("BT19-093 On trashed: -3000 DP + disable When Digivolving")
        effect0.set_effect_description(
            "When this card is trashed from the battle area, until the end of your "
            "opponent's turn, 1 of their Digimon can't activate [When Digivolving] "
            "effects and gets -3000 DP."
        )
        effect0.is_on_deletion = True

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
                from ....interfaces.modifiers import ModifierType
                # -3000 DP until end of opponent's turn
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur - 3000,
                    expiry='end_of_opponent_turn')
                # Disable [When Digivolving] effects until end of opponent's turn
                game.register_modifier(
                    target_perm, ModifierType.DISABLE_EFFECT,
                    condition=lambda p, c, fp=target_perm: (
                        p is fp
                        and c.get('effect') is not None
                        and getattr(c['effect'], 'is_when_digivolving', False)
                    ),
                    expiry='end_of_opponent_turn')

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -3000 DP and disable [When Digivolving].")

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
        # _is_delay keeps the option card on the field after resolution
        effect1._is_delay = True

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
                from ....interfaces.modifiers import ModifierType
                # -3000 DP until end of opponent's turn
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda cur, t, c: cur - 3000,
                    expiry='end_of_opponent_turn')
                # Disable [When Digivolving] effects until end of opponent's turn
                game.register_modifier(
                    target_perm, ModifierType.DISABLE_EFFECT,
                    condition=lambda p, c, fp=target_perm: (
                        p is fp
                        and c.get('effect') is not None
                        and getattr(c['effect'], 'is_when_digivolving', False)
                    ),
                    expiry='end_of_opponent_turn')

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 opponent Digimon to give -3000 DP and disable [When Digivolving].")

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
                # No opponent => just add to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)
                return

            # Apply SA-2 to up to 2 opponent Digimon via selection
            digimon_targets = [p for p in enemy.battle_area if p.is_digimon]
            remaining_count = min(2, len(digimon_targets))
            if remaining_count > 0:
                selected_set = set()

                def _select_next():
                    nonlocal remaining_count
                    if remaining_count <= 0:
                        # All selections done, add card to hand
                        if card in player.trash_cards:
                            player.trash_cards.remove(card)
                            player.hand_cards.append(card)
                        return
                    def sa_filter(p):
                        return p.is_digimon and id(p) not in selected_set
                    if not any(sa_filter(p) for p in enemy.battle_area):
                        # No more valid targets, add card to hand
                        if card in player.trash_cards:
                            player.trash_cards.remove(card)
                            player.hand_cards.append(card)
                        return

                    def on_sa_select(target_perm):
                        nonlocal remaining_count
                        from ....interfaces.modifiers import ModifierType
                        # SA-2 via modifier with end_of_turn expiry
                        # condition restricts to this specific target permanent
                        game.register_modifier(
                            target_perm, ModifierType.CHANGE_SECURITY_ATTACK,
                            condition=lambda p, c, fp=target_perm: p is fp,
                            value_fn=lambda cur, t, c: cur - 2,
                            expiry='end_of_turn')
                        selected_set.add(id(target_perm))
                        remaining_count -= 1
                        _select_next()

                    game.effect_select_opponent_permanent(
                        player, on_sa_select, filter_fn=sa_filter,
                        is_optional=False,
                        prompt=f"Select opponent Digimon to give Security A. -2 ({remaining_count} remaining).")

                _select_next()
            else:
                # No Digimon targets, just add card to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
