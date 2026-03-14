from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX3_057(CardScript):
    """EX3-057 Growlmon | Lv.4
    [When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP or less.
    If no Digimon is deleted by this effect, trash the top 2 cards of both
    players' decks.

    Inherited: [When Attacking][Once Per Turn] By deleting 1 of your other Digimon,
    this Digimon gains SA+1 for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [When Digivolving] Delete opponent 3000 DP or less, or mill 2 both
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX3-057 When Digivolving: Delete 3k or mill 2 both")
        effect0.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's Digimon with 3000 DP "
            "or less. If no Digimon is deleted, trash the top 2 cards of both "
            "players' decks."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Try to delete opponent Digimon with 3000 DP or less
            targets = [p for p in enemy.battle_area
                      if p.is_digimon and p.dp <= 3000]
            if targets:
                def target_filter(p):
                    return p.is_digimon and p.dp <= 3000

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            else:
                # No valid targets, mill 2 from both decks
                for p in [player, enemy]:
                    mill_count = min(2, len(p.library_cards))
                    trashed = p.library_cards[:mill_count]
                    p.library_cards = p.library_cards[mill_count:]
                    p.trash_cards.extend(trashed)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: [When Attacking][Once Per Turn] Delete own other Digimon for SA+1
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnTappedAnyone)
        effect1.set_effect_name("EX3-057 Inherited: Delete own for SA+1")
        effect1.set_effect_description(
            "Inherited: [When Attacking][Once Per Turn] By deleting 1 of your "
            "other Digimon, this Digimon gains SA+1 for the turn."
        )
        effect1.is_inherited_effect = True
        effect1.is_on_attack = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("SA+1_EX3_057")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.owner:
                my_perm = card.permanent_of_this_card()
                other_digimon = [p for p in card.owner.battle_area
                                if p.is_digimon and p is not my_perm]
                if not other_digimon:
                    return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            def other_filter(p):
                return p.is_digimon and p is not perm

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                # Grant SA+1
                perm.security_attack_modifier = getattr(perm, 'security_attack_modifier', 0) + 1

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=other_filter, is_optional=False,
                prompt="Delete 1 of your other Digimon to grant SA+1.")
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
