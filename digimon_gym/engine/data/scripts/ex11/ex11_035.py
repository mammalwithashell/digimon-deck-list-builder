from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_035(CardScript):
    """EX11-035 Zephagamon | Lv.6, Green, Magic Knight/Vortex Warriors/LIBERATOR, DP 12000, Cost 11

    <Piercing> <Vortex> <Blocker>
    [When Digivolving] You may unsuspend 1 Digimon. Then, you may suspend 1 Digimon.
    [All Turns][Once Per Turn] When any of your Digimon suspend, you may play 1 3000 DP or lower
    green Digimon card with [Avian] or [Bird] in any of its traits from your hand without paying
    the cost. For each suspended Digimon, add 2000 to this effect's DP maximum.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Keyword: <Piercing> ---
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("EX11-035 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def cond_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(cond_piercing)
        effects.append(effect_piercing)

        # --- Keyword: <Vortex> ---
        effect_vortex = ICardEffect()
        effect_vortex.set_effect_name("EX11-035 Vortex")
        effect_vortex.set_effect_description("Vortex")
        effect_vortex._is_vortex = True

        def cond_vortex(context: Dict[str, Any]) -> bool:
            return True
        effect_vortex.set_can_use_condition(cond_vortex)
        effects.append(effect_vortex)

        # --- Keyword: <Blocker> ---
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX11-035 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def cond_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(cond_blocker)
        effects.append(effect_blocker)

        # --- [When Digivolving] You may unsuspend 1 Digimon. Then, you may suspend 1 Digimon. ---
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX11-035 [When Digivolving] Unsuspend 1, then suspend 1")
        effect_wd.set_effect_description(
            "[When Digivolving] You may unsuspend 1 Digimon. Then, you may suspend 1 Digimon."
        )
        effect_wd.is_when_digivolving = True
        effect_wd.is_optional = True

        def cond_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(cond_wd)

        def process_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: You may unsuspend 1 Digimon (any field — offer own first, then opponent)
            unsuspend_happened = [False]

            def on_unsuspend_own(p):
                p.unsuspend()
                unsuspend_happened[0] = True

            def on_unsuspend_opp(p):
                p.unsuspend()
                unsuspend_happened[0] = True

            own_suspended = [p for p in player.battle_area if p.is_digimon and p.is_suspended]
            if own_suspended:
                game.effect_select_own_permanent(
                    player,
                    on_unsuspend_own,
                    filter_fn=lambda p: p.is_digimon and p.is_suspended,
                    is_optional=True,
                    prompt="You may unsuspend 1 of your Digimon.",
                )
            if not unsuspend_happened[0] and player.enemy:
                opp_suspended = [p for p in player.enemy.battle_area if p.is_digimon and p.is_suspended]
                if opp_suspended:
                    game.effect_select_opponent_permanent(
                        player,
                        on_unsuspend_opp,
                        filter_fn=lambda p: p.is_digimon and p.is_suspended,
                        is_optional=True,
                        prompt="You may unsuspend 1 of your opponent's Digimon.",
                    )
            # Step 2: You may suspend 1 Digimon (any field — offer own first, then opponent)
            suspend_happened = [False]

            def on_suspend_own(p):
                p.suspend()
                suspend_happened[0] = True

            def on_suspend_opp(p):
                p.suspend()
                suspend_happened[0] = True

            own_unsuspended = [p for p in player.battle_area if p.is_digimon and not p.is_suspended]
            if own_unsuspended:
                game.effect_select_own_permanent(
                    player,
                    on_suspend_own,
                    filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                    is_optional=True,
                    prompt="You may suspend 1 of your Digimon.",
                )
            if not suspend_happened[0] and player.enemy:
                opp_unsuspended = [p for p in player.enemy.battle_area if p.is_digimon and not p.is_suspended]
                if opp_unsuspended:
                    game.effect_select_opponent_permanent(
                        player,
                        on_suspend_opp,
                        filter_fn=lambda p: p.is_digimon and not p.is_suspended,
                        is_optional=True,
                        prompt="You may suspend 1 of your opponent's Digimon.",
                    )

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # --- [All Turns][Once Per Turn] When any of YOUR Digimon suspend, may play green Avian/Bird
        #     Digimon with DP <= (3000 + 2000 * suspended_count) from hand for free. ---
        effect_tap = ICardEffect()
        effect_tap.set_timing(EffectTiming.OnTappedAnyone)
        effect_tap.set_effect_name(
            "EX11-035 [All Turns][OPT] When your Digimon suspends, play green Avian/Bird Digimon"
        )
        effect_tap.set_effect_description(
            "[All Turns][Once Per Turn] When any of your Digimon suspend, you may play 1 "
            "green Digimon with [Avian] or [Bird] in traits, DP <= 3000 + 2000 per suspended "
            "Digimon, from hand for free."
        )
        effect_tap.is_optional = True
        effect_tap.set_max_count_per_turn(1)
        effect_tap.set_hash_string("EX11_035_AT")

        def cond_tap(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered only when ONE OF YOUR Digimon suspends
            suspended_perm = context.get('permanent')
            if not suspended_perm:
                return False
            if not suspended_perm.is_digimon:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # The suspended Digimon must belong to this card's owner
            if suspended_perm not in list(owner.battle_area):
                return False
            return True

        effect_tap.set_can_use_condition(cond_tap)

        def process_tap(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Count all suspended Digimon on both fields for the DP cap
            suspended_count = sum(
                1 for p in list(player.battle_area) + list(player.enemy.battle_area)
                if p.is_suspended and p.is_digimon
            ) if player.enemy else sum(
                1 for p in player.battle_area if p.is_suspended and p.is_digimon
            )
            max_dp = 3000 + 2000 * suspended_count

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if 'Green' not in [col.name for col in getattr(c, 'card_colors', [])]:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('Avian' in t or 'Bird' in t for t in traits):
                    return False
                card_dp = getattr(c, 'base_dp', None)
                if card_dp is None:
                    return False
                return card_dp <= max_dp

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt=f"You may play a green Avian/Bird Digimon with DP <= {max_dp} for free.",
            )

        effect_tap.set_on_process_callback(process_tap)
        effects.append(effect_tap)

        return effects
