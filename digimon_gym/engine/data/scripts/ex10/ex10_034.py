from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_034(CardScript):
    """EX10-034 Blastmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Keyword: Collision
        effect_collision = ICardEffect()
        effect_collision.set_effect_name("EX10-034 Collision")
        effect_collision.set_effect_description("Collision")
        effect_collision._is_collision = True

        def condition_collision(context: Dict[str, Any]) -> bool:
            return True
        effect_collision.set_can_use_condition(condition_collision)
        effects.append(effect_collision)

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX10-034 Fragment")
        effect_fragment.set_effect_description("Fragment")
        effect_fragment._is_fragment = True
        effect_fragment._fragment_count = 3

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True
        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # Keyword: Blocker
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX10-034 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # [On Play] [When Digivolving] Give 1 opponent Digimon FORCE_ATTACK until opponent's turn ends
        def build_force_attack_effect(is_on_play: bool):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("EX10-034 Give opponent Digimon forced attack")
            effect.set_effect_description(
                "[On Play] [When Digivolving] Until your opponent's turn ends, give 1 of their "
                "Digimon '[Start of Your Main Phase] This Digimon attacks.'"
            )
            if is_on_play:
                effect.is_on_play = True
            else:
                effect.is_when_digivolving = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                from ....interfaces.modifiers import ModifierType

                def on_select(target_perm):
                    game.register_modifier(
                        target_perm,
                        ModifierType.FORCE_ATTACK,
                        condition=lambda p, c, tp=target_perm: p is tp,
                        expiry='end_of_opponent_turn',
                    )

                game.effect_select_opponent_permanent(
                    player, on_select,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 opponent Digimon to force attack at start of their main phase.",
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(build_force_attack_effect(is_on_play=True))
        effects.append(build_force_attack_effect(is_on_play=False))

        # [All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this
        # Digimon's digivolution cards, gain <Security A. +1> and +3000 DP until your turn ends.
        effect_at = ICardEffect()
        effect_at.set_timing(EffectTiming.OnDeclaration)
        effect_at.set_effect_name("EX10-034 Trash 2 sources, gain Security A. +1 and +3000 DP")
        effect_at.set_effect_description(
            "[All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this "
            "Digimon's digivolution cards, this Digimon gains <Security A. +1> and +3000 DP "
            "until your turn ends."
        )
        effect_at.is_optional = True
        effect_at.set_max_count_per_turn(1)
        effect_at.set_hash_string("AT_EX10_034")

        def condition_at(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            if perm.has_no_digivolution_cards:
                return False
            return len(perm.digivolution_cards) >= 2

        effect_at.set_can_use_condition(condition_at)

        def process_at(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            from ....interfaces.modifiers import ModifierType
            # Trash 2 digivolution cards
            trashed = perm.trash_digivolution_cards(2)
            player.trash_cards.extend(trashed)
            # Grant +3000 DP until end of turn
            game.register_modifier(
                perm,
                ModifierType.CHANGE_DP,
                value_fn=lambda: 3000,
                expiry='end_of_turn',
            )
            # Grant Security A. +1 until end of turn
            game.register_modifier(
                perm,
                ModifierType.CHANGE_SECURITY_ATTACK,
                value_fn=lambda: 1,
                expiry='end_of_turn',
            )

        effect_at.set_on_process_callback(process_at)
        effects.append(effect_at)

        return effects
