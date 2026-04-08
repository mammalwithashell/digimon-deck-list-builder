from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST22_11(CardScript):
    """ST22-11 Defense Plug-In F | Option (Black, Cost 3)

    While you have a Tamer, you can ignore this card's color requirements.
    [Security] De-Digivolve 2 1 of your opponent's Digimon. Then, add
    this card to the hand.
    [Main] You may link this card to 1 of your Digimon without paying
    the cost. Then, until your opponent's turn ends, 1 of your Digimon
    gains Reboot and +3000 DP.
    Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # --- Color requirement bypass: while you have a Tamer ---
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce
            has_tamer = any(getattr(p, 'is_tamer', False) for p in owner.battle_area)
            if has_tamer:
                return False  # bypass color requirement
            return True  # enforce color requirement
        card._match_color_requirement_fn = _check_color_req

        effects = []

        # [Security] De-Digivolve 2, then add to hand
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("ST22-11 Security: De-Digivolve 2, add to hand")
        effect0.set_effect_description(
            "[Security] De-Digivolve 2 1 of your opponent's Digimon. "
            "Then, add this card to the hand."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon and len(p.card_sources) > 1,
                is_optional=False,
                prompt="Select 1 opponent Digimon to De-Digivolve 2.")

            # Add this card to hand
            if card and player:
                if card in player.security_cards:
                    player.security_cards.remove(card)
                player.hand_cards.append(card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Main] Link this card to 1 of your Digimon, then grant Reboot + 3000 DP
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("ST22-11 Link and grant Reboot + 3000 DP")
        effect1.set_effect_description(
            "[Main] You may link this card to 1 of your Digimon without paying "
            "the cost. Then, until your opponent's turn ends, 1 of your "
            "Digimon gains Reboot and +3000 DP."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Link this card to a Digimon
            if card:
                game.effect_link_to_permanent(
                    player, card,
                    filter_fn=lambda p: p.is_digimon and (p.level or 0) >= 3,
                    is_optional=True,
                    prompt="Link Defense Plug-In F to 1 of your Digimon.")

            # Grant Reboot and +3000 DP to 1 of your Digimon
            def on_target(target_perm):
                game.register_modifier(
                    target_perm, ModifierType.GRANT_REBOOT,
                    condition=lambda p, c, fp=target_perm: p is fp,
                    expiry='end_of_opponent_turn',
                )
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda current, target, c: current + 3000,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_target,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your Digimon to gain Reboot and +3000 DP.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2
        effect2 = ICardEffect()
        effect2.set_effect_name("ST22-11 Link Requirements")
        effect2.set_effect_description("Link Requirements [Link] Lv.3 or higher: Cost 2")
        effect2.is_inherited_effect = True
        effect2._is_link = True
        effect2._link_cost = 2
        effect2._link_min_level = 3

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
