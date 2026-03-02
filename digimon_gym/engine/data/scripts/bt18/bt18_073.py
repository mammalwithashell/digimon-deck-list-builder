from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_073(CardScript):
    """BT18-073 Machinedramon | Lv.6 Black/Purple Machine"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Play cost reduction by 4 when deleting a Composite trait Digimon ---
        # NOTE: Play cost reduction by deleting own Digimon as cost is not
        # directly modelable as a static cost_reduction. We register the
        # reduction but the delete-as-cost enforcement is partial.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT18-073 Reduce play cost by 4 (delete Composite)")
        effect0.set_effect_description(
            "When you would play this card, by deleting 1 of your Digimon "
            "with the [Composite] trait, reduce the play cost by 4."
        )
        effect0.cost_reduction = 4

        def condition0(context: Dict[str, Any]) -> bool:
            # Check if player has a Composite Digimon to delete
            if card and card.owner:
                return any(
                    p.is_digimon and p.has_trait('Composite')
                    for p in card.owner.battle_area
                )
            return False
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] [When Digivolving] <De-Digivolve 1> all
        #     opponent's Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT18-073 De-Digivolve 1 all opponent Digimon")
        effect1.set_effect_description(
            "[On Play] [When Digivolving] <De-Digivolve 1> all of your "
            "opponent's Digimon."
        )
        effect1.is_on_play = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De-Digivolve 1 all opponent's Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # De-Digivolve 1 each opponent Digimon
            for perm in list(enemy.battle_area):
                if perm.is_digimon:
                    removed = perm.de_digivolve(1)
                    enemy.trash_cards.extend(removed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Deletion] DNA digivolve Kimeramon in play +
        #     Machinedramon in trash into Millenniummon in hand ---
        # NOTE: DNA digivolution from trash is complex. Marked PARTIAL.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT18-073 On Deletion: DNA digivolve into Millenniummon")
        effect2.set_effect_description(
            "[On Deletion] You may DNA digivolve 1 of your [Kimeramon] in "
            "play and 1 [Machinedramon] in the trash into [Millenniummon] in the hand."
        )
        effect2.is_on_deletion = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner):
                return False
            player = card.owner
            has_kimeramon = any(
                p.is_digimon and p.contains_card_name('Kimeramon')
                for p in player.battle_area
            )
            has_millenniummon_in_hand = any(
                c.contains_card_name('Millenniummon')
                for c in player.hand_cards
            )
            has_machinedramon_in_trash = any(
                c.contains_card_name('Machinedramon')
                for c in player.trash_cards
            )
            return has_kimeramon and has_millenniummon_in_hand and has_machinedramon_in_trash
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DNA digivolve Kimeramon + Machinedramon into Millenniummon"""
            # descriptive-tagged: DNA digivolution from trash into hand card
            # requires engine-level support. Partial implementation.
            pass

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [Opponent's Turn][Once Per Turn] When opponent
        #     attacks, change target to your Composite/Wicked God Digimon ---
        # NOTE: Attack redirect is a complex effect. Marked PARTIAL.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT18-073 Inherited: Redirect attack to Composite/Wicked God")
        effect3.set_effect_description(
            "Inherited: [Opponent's Turn][Once Per Turn] When any of your "
            "opponent's Digimon attack, you may change the attack target to "
            "1 of your Digimon with the [Composite] or [Wicked God] trait."
        )
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("redirect_attack_BT18_073")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.owner and not card.owner.is_my_turn:
                return True
            return False
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Redirect attack to Composite/Wicked God Digimon"""
            # descriptive-tagged: attack target redirection is not fully
            # supported in the engine
            pass

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
