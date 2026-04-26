from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_021(CardScript):
    """EX11-021 Kokeshimon | Lv.4

    Alt digivolve: Lv.3 with [Puppet] trait for cost 2.

    [When Digivolving] If you have 1 or fewer Tamers, you may play 1
        [Mirai Kinosaki] from your hand without paying the cost.

    --- Inherited ---
    [Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks,
        by deleting 1 of your other Digimon, end the attack.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from Lv.3 [Puppet] for cost 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-021 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_trait = "Puppet"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Play [Mirai Kinosaki] if <=1 Tamers ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-021 Play 1 [Mirai Kinosaki] from your hand")
        effect1.set_effect_description("[When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Mirai Kinosaki] from your hand without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check tamer count <= 1
            player = card.owner if card else None
            if not player:
                return False
            tamer_count = sum(1 for p in player.battle_area if p.is_tamer)
            return tamer_count <= 1
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Mirai Kinosaki' in n for n in names)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="You may play 1 [Mirai Kinosaki] from hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [Opponent's Turn][Once Per Turn] End attack by deleting other Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name("EX11-021 End the attack by deleting 1 of your Digimon")
        effect2.set_effect_description("[Opponent's Turn][Once Per Turn] When an opponent's Digimon attacks, by deleting 1 of your other Digimon, end the attack.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("StopAttack_EX11-021")
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
