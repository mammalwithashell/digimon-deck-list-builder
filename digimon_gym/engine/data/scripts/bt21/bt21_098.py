from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_098(CardScript):
    """BT21-098 Ragnarok Cannon | Option

    [Main] Delete 1 of your opponent's Digimon with the lowest play cost.
    Then, place this card in the battle area.

    [Your Turn] When one of your [Galacticmon] attacks, <Delay>.
    * Delete 1 of your opponent's Digimon with the lowest play cost. If this
      effect didn't delete, trash the top cards of your opponent's security
      stack so that it has 1 card left.

    [Security] You may play 1 card with [Vemmon] in its text and a play cost
    of 6 or less from your hand or trash without paying the cost. Then, add
    this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_vemmon_text(c) -> bool:
            return 'Vemmon' in getattr(c, 'card_text', '')

        def _delete_lowest_cost_digimon(player, game):
            """Delete 1 of opponent's Digimon with the lowest play cost. Returns True if deleted."""
            enemy = player.enemy if player else None
            if not enemy:
                return False
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return False
            # Find lowest play cost
            def get_play_cost(p):
                top = p.top_card
                if top is None:
                    return 999
                return getattr(top, 'get_cost_itself', 999)
            lowest_cost = min(get_play_cost(p) for p in opp_digimon)
            lowest_digimon = [p for p in opp_digimon if get_play_cost(p) == lowest_cost]
            if not lowest_digimon:
                return False

            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)

            def lowest_filter(p):
                return p.is_digimon and get_play_cost(p) == lowest_cost

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=lowest_filter, is_optional=False)
            return True

        # ─── Effect 0: [Main] Delete 1 opponent's Digimon with lowest play cost.
        #     Then place this card in the battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT21-098 Delete lowest cost Digimon")
        effect0.set_effect_description("[Main] Delete 1 of your opponent's Digimon with the lowest play cost. Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Delete lowest-cost opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _delete_lowest_cost_digimon(player, game)
            # Placement in battle area is handled by the engine for Option cards
            # with OptionSkill timing (they stay in battle area as delayed cards)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: Delay marker — triggers when [Galacticmon] attacks on your turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-098 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ─── Effect 2: Delay effect — [Your Turn] When [Galacticmon] attacks.
        #     Delete 1 lowest-cost opponent Digimon. If didn't delete, trash
        #     opponent's security until 1 left.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2.set_effect_name("BT21-098 Delay: Delete lowest cost or trash security")
        effect2.set_effect_description("[Your Turn] When one of your [Galacticmon] attacks, <Delay>. Delete 1 of your opponent's Digimon with the lowest play cost. If this effect didn't delete, trash the top cards of your opponent's security stack so that it has 1 card left.")
        effect2._is_delay_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The attacker must be [Galacticmon]
            attacker = context.get('permanent')
            if attacker is None or not attacker.is_digimon:
                return False
            if not attacker.contains_card_name('Galacticmon'):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete lowest-cost opponent Digimon. If no delete, trash security to 1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            enemy = player.enemy if player else None
            if not enemy:
                return

            # Check if opponent has Digimon to delete
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                _delete_lowest_cost_digimon(player, game)
            else:
                # Didn't delete — trash security until 1 left
                while len(enemy.security_cards) > 1:
                    sec_card = enemy.security_cards.pop(0)
                    enemy.trash_cards.append(sec_card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3: [Security] Play 1 Vemmon-text card with play cost <= 6
        #     from hand or trash free. Then add this card to hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT21-098 Security: Play Vemmon-text, add to hand")
        effect3.set_effect_description("[Security] You may play 1 card with [Vemmon] in its text and a play cost of 6 or less from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — always valid when revealed from security
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play 1 Vemmon-text card cost<=6 from hand/trash free, then add this card to hand."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not _has_vemmon_text(c):
                    return False
                cost = getattr(c, 'get_cost_itself', 99)
                return cost <= 6

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

            # Add this card (Ragnarok Cannon) to hand
            if card:
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
