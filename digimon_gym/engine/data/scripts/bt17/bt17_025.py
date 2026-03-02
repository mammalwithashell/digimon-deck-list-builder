from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_025(CardScript):
    """BT17-025 Cerberusmon: Werewolf Mode | Lv.5 Blue/Purple Digimon

    [When Digivolving] You may play 1 level 3 blue or purple Digimon card
        from your trash or from one of your Digimon's digivolution cards
        without paying the cost. At the next end of your opponent's turn,
        return it to the hand.
    Inherited Effect [All Turns] [Once Per Turn] When an effect plays one
        of your Digimon, return 1 of your opponent's level 3 Digimon to
        the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Play Lv.3 blue/purple from trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-025 When Digivolving: play Lv.3 from trash")
        effect0.set_effect_description(
            "[When Digivolving] You may play 1 level 3 blue or purple Digimon "
            "card from your trash or from one of your Digimon's digivolution "
            "cards without paying the cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play 1 Lv.3 blue/purple Digimon from trash free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level != 3:
                    return False
                colors = getattr(c, 'card_colors', []) or []
                color_names = [col.name for col in colors]
                if 'Blue' not in color_names and 'Purple' not in color_names:
                    return False
                return True

            # Try to play from trash
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [All Turns] [Once Per Turn] Bounce opponent Lv.3 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT17-025 Inherited: bounce opponent Lv.3")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When an effect plays one of your "
            "Digimon, return 1 of your opponent's level 3 Digimon to the hand."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("AllTurns_BT17-025_BounceL3")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Return 1 opponent Lv.3 Digimon to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            targets = [
                p for p in enemy.battle_area
                if p.is_digimon and p.level is not None and p.level == 3
            ]
            if targets:
                target = targets[0]
                if target.top_card:
                    enemy.hand_cards.append(target.top_card)
                if target in enemy.battle_area:
                    enemy.battle_area.remove(target)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
