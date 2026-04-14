from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_020(CardScript):
    """EX11-020 Hanimon | Lv.3

    Alt digivolve: from [Kyaromon] for cost 0.

    [On Deletion] If deleted other than in battle, you may play 1 [Shoemon]
        from your hand without paying the cost.

    --- Inherited ---
    [Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks,
        by deleting 1 of your other Digimon, end that attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Kyaromon] for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-020 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Kyaromon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Kyaromon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Play 1 [Shoemon] from hand (if not in battle) ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX11-020 Play 1 [Shoemon] from hand for free.")
        effect1.set_effect_description("[On Deletion] If deleted other than in battle, you may play 1 [Shoemon] from your hand without paying the cost.")
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            # C# ref: CanTriggerOnDeletion(hashtable, card) && !IsByBattle(hashtable)
            if context.get('is_battle', False):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Shoemon' == n for n in names)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="You may play 1 [Shoemon] from hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [Opponent's Turn][Once Per Turn] End attack by deleting other Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("EX11-020 End the attack by deleting 1 of your Digimon")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, by deleting 1 of your other Digimon, end that attack.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("StopAttack_EX11-020")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on opponent's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            player = card.owner if card else None
            if not player:
                return False
            my_perm = card.permanent_of_this_card()
            # Must have another Digimon to delete
            has_other = any(
                p.is_digimon and p is not my_perm
                for p in player.battle_area
            )
            return has_other
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None

            def own_digimon_filter(p):
                return p.is_digimon and p is not my_perm

            def on_delete(target_perm):
                player.delete_permanent(target_perm)
                game.force_end_attack()

            game.effect_select_own_permanent(
                player, on_delete, filter_fn=own_digimon_filter,
                is_optional=False,
                prompt="Select 1 of your other Digimon to delete to end the attack."
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
