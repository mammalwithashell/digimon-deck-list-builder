from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_055(CardScript):
    """BT17-055 Infermon | Lv.5 Ultimate | Black | Unidentified

    [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon.
        Then, 1 of your opponent's Digimon with a play cost 8 or less
        can't attack players until the end of their turn.
    Inherited: [All Turns] [Once Per Turn] When you play another Digimon
        with [Diaboromon] in its name, <De-Digivolve 1> 1 of your
        opponent's Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] De-Digivolve 1 + attack restriction ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-055 When Digivolving: De-Digivolve 1 + can't attack")
        effect0.set_effect_description(
            "[When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon. "
            "Then, 1 of your opponent's Digimon with a play cost 8 or less "
            "can't attack players until the end of their turn."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """De-Digivolve 1 opponent Digimon, then restrict attack on cost<=8."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: De-Digivolve 1 an opponent's Digimon
            def dedigivolve_filter(p):
                return p.is_digimon and len(p.card_sources) > 1

            def on_dedigivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                for rc in removed:
                    enemy.trash_cards.append(rc)

                # Step 2: Restrict attack on a cost<=8 opponent Digimon
                def restrict_filter(p):
                    if not p.is_digimon:
                        return False
                    if p.top_card:
                        return (p.top_card.get_cost_itself or 0) <= 8
                    return False

                def on_restrict(target_perm2):
                    target_perm2.grant_keyword('_is_cannot_attack_player')

                game.effect_select_opponent_permanent(
                    player, on_restrict, filter_fn=restrict_filter,
                    is_optional=True)

            game.effect_select_opponent_permanent(
                player, on_dedigivolve, filter_fn=dedigivolve_filter,
                is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [All Turns] [Once Per Turn] ---
        # When you play another Digimon with [Diaboromon] in its name,
        # De-Digivolve 1 an opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-055 Inherited: De-Digivolve on Diaboromon play")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When you play another Digimon with "
            "[Diaboromon] in its name, <De-Digivolve 1> 1 of your opponent's "
            "Digimon."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Inherited_BT17_055_DeDigi_OnDiaboromonPlay")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check if the played permanent is a Diaboromon
            trigger_perm = context.get('permanent')
            if trigger_perm and trigger_perm.top_card:
                name = trigger_perm.top_card.c_entity_base.card_name_eng if trigger_perm.top_card.c_entity_base else ''
                if 'Diaboromon' in name:
                    # Make sure it's not this permanent itself
                    own_perm = card.permanent_of_this_card() if card else None
                    if own_perm and trigger_perm != own_perm:
                        return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """De-Digivolve 1 an opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            def dedigivolve_filter(p):
                return p.is_digimon and len(p.card_sources) > 1

            def on_dedigivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                for rc in removed:
                    enemy.trash_cards.append(rc)

            game.effect_select_opponent_permanent(
                player, on_dedigivolve, filter_fn=dedigivolve_filter,
                is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
