from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_016(CardScript):
    """BT20-016 Paildramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Jogress Condition
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-016 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Jogress Condition"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DNA/Jogress digivolution condition — handled by engine
            pass

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] For the turn, 1 of your Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would) and gets +4000 DP. Then, this Digimon may attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-016 Piercing and Digivolve into Imperialdramon Dragon Mode")
        effect1.set_effect_description("[On Play] For the turn, 1 of your Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would) and gets +4000 DP. Then, this Digimon may attack.")
        effect1.is_on_play = True
        effect1.dp_modifier = 4000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(4000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] For the turn, 1 of your Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would) and gets +4000 DP. Then, this Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-016 1 Digimon gains Piercing and 4000DP, then this digimon may attack")
        effect2.set_effect_description("[When Digivolving] For the turn, 1 of your Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would) and gets +4000 DP. Then, this Digimon may attack.")
        effect2.is_when_digivolving = True
        effect2.dp_modifier = 4000

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DP +4000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(4000)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenPermanentWouldBeDeleted
        # [All Turns] When any of your [Paildramon]/[Dinobeemon] would be deleted, 2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-016 If would be deleted, DNA Digivolve")
        effect3.set_effect_description("[All Turns] When any of your [Paildramon]/[Dinobeemon] would be deleted, 2 of your Digimon may DNA digivolve into [Imperialdramon: Dragon Mode] in the hand.")
        effect3.is_optional = True
        effect3.set_hash_string("Dna Digivolve into Imperialdramon Dragon Mode")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Dinobeemon') or permanent.contains_card_name('Paildramon'))):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def hand_filter(c):
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: security_attack_plus
        # Security Attack +1
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-016 Security Attack +1")
        effect4.set_effect_description("Security Attack +1")
        effect4.is_inherited_effect = True
        effect4._security_attack_modifier = 1

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
