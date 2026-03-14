from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_073(CardScript):
    """BT18-073 Machinedramon | Lv.6 Black/Purple Machine

    Alt digi: Lv.5 [Composite] trait, cost 3.
    When you would play this card, by deleting 1 of your Digimon with the
    [Composite] trait, reduce the play cost by 4.
    [On Play] [When Digivolving] <De-Digivolve 1> all of your opponent's Digimon.
    [On Deletion] You may DNA digivolve 1 of your [Kimeramon] in play and
    1 [Machinedramon] in the trash into [Millenniummon] in the hand.
    Inherited: [Opponent's Turn][Once Per Turn] When any of your opponent's
    Digimon attack, you may change the attack target to 1 of your Digimon
    with the [Composite] or [Wicked God] trait.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Alt-Digi: From Lv.5 Composite trait, cost 3 ---
        effect_alt = ICardEffect()
        effect_alt.set_effect_name("BT18-073 Alt digi: Lv.5 Composite cost 3")
        effect_alt.set_effect_description("Alternate digivolution: Lv.5 with [Composite] trait, cost 3")
        effect_alt._alt_digi_cost = 3
        effect_alt._alt_digi_level = 5
        effect_alt._alt_digi_trait = "Composite"

        def condition_alt(context: Dict[str, Any]) -> bool:
            return True
        effect_alt.set_can_use_condition(condition_alt)
        effects.append(effect_alt)

        # --- Effect 0: Cost reduction by 4 when deleting a Composite Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT18-073 Reduce play cost by 4 (delete Composite)")
        effect0.set_effect_description(
            "When you would play this card, by deleting 1 of your Digimon "
            "with the [Composite] trait, reduce the play cost by 4."
        )
        effect0.cost_reduction = 4

        def condition0(context: Dict[str, Any]) -> bool:
            # Only applies when this specific card is being played
            if context.get('card_source') is not card:
                return False
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
        def _de_digivolve_all(ctx: Dict[str, Any]):
            """De-Digivolve 1 all opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            for perm in list(enemy.battle_area):
                if perm.is_digimon:
                    removed = perm.de_digivolve(1)
                    enemy.trash_cards.extend(removed)

        effect1a = ICardEffect()
        effect1a.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1a.set_effect_name("BT18-073 On Play: De-Digivolve 1 all opponent Digimon")
        effect1a.set_effect_description(
            "[On Play] <De-Digivolve 1> all of your opponent's Digimon."
        )
        effect1a.is_on_play = True

        def condition1a(context: Dict[str, Any]) -> bool:
            return True
        effect1a.set_can_use_condition(condition1a)
        effect1a.set_on_process_callback(_de_digivolve_all)
        effects.append(effect1a)

        effect1b = ICardEffect()
        effect1b.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1b.set_effect_name("BT18-073 When Digivolving: De-Digivolve 1 all opponent Digimon")
        effect1b.set_effect_description(
            "[When Digivolving] <De-Digivolve 1> all of your opponent's Digimon."
        )
        effect1b.is_when_digivolving = True

        def condition1b(context: Dict[str, Any]) -> bool:
            return True
        effect1b.set_can_use_condition(condition1b)
        effect1b.set_on_process_callback(_de_digivolve_all)
        effects.append(effect1b)

        # --- Effect 2: [On Deletion] DNA digivolve Kimeramon (field) +
        #     Machinedramon (trash) into Millenniummon (hand) ---
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
                c.contains_card_name('Millenniummon') and getattr(c, 'is_digimon', False)
                for c in player.hand_cards
            )
            has_machinedramon_in_trash = any(
                c.contains_card_name('Machinedramon')
                for c in player.trash_cards
            )
            return has_kimeramon and has_millenniummon_in_hand and has_machinedramon_in_trash
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """DNA digivolve: Kimeramon (field) + Machinedramon (trash) -> Millenniummon (hand)"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def millenniummon_filter(c):
                return c.contains_card_name('Millenniummon') and getattr(c, 'is_digimon', False)

            game.effect_dna_digivolve_from_hand(
                player, millenniummon_filter, is_optional=True,
                prompt="Select [Millenniummon] from hand to DNA digivolve.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [Opponent's Turn][Once Per Turn] When opponent
        #     attacks, change target to your Composite/Wicked God Digimon ---
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
            if not (card and card.owner and not card.owner.is_my_turn):
                return False
            if card.permanent_of_this_card() is None:
                return False
            # Must have a valid Composite or Wicked God Digimon to redirect to
            owner = card.owner
            return any(
                p.is_digimon and (p.has_trait('Composite') or p.has_trait('Wicked God'))
                for p in owner.battle_area
            )
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Redirect attack to Composite/Wicked God Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def redirect_filter(p):
                return p.is_digimon and (p.has_trait('Composite') or p.has_trait('Wicked God'))

            def on_redirect_selected(target_perm):
                # Change the attack target to the selected permanent
                if hasattr(game, 'pending_attack') and game.pending_attack:
                    game.pending_attack.target = target_perm

            game.effect_select_own_permanent(
                player, on_redirect_selected, filter_fn=redirect_filter,
                is_optional=False,
                prompt="Select a [Composite] or [Wicked God] Digimon to redirect the attack to.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
