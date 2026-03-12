from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_064(CardScript):
    """EX7-064 Shoto Kazama | Tamer (Green, LIBERATOR, Cost 3)

    [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
    [End of Your Turn] By suspending this Tamer, 1 of your Digimon gets
        <Piercing> and <Blocker> until the end of your opponent's turn. If that
        Digimon has the [Vortex Warriors] trait, unsuspend that Digimon.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Main Phase] If opponent has a Digimon, +1 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX7-064 Gain 1 memory if opponent has Digimon")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            has_digimon = any(p.is_digimon for p in enemy.battle_area)
            if has_digimon:
                player.add_memory(1)
                game.logger.log(
                    f"[EX7-064] {player.player_name} gained 1 memory "
                    f"(opponent has Digimon)."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [End of Your Turn] Suspend this tamer → grant Piercing+Blocker ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name(
            "EX7-064 Suspend tamer to grant Piercing+Blocker until end of opponent's turn"
        )
        effect1.set_effect_description(
            "[End of Your Turn] By suspending this Tamer, 1 of your Digimon gets "
            "<Piercing> and <Blocker> until the end of your opponent's turn. "
            "If that Digimon has the [Vortex Warriors] trait, unsuspend that Digimon."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Tamer must not already be suspended to pay the cost
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Need at least one own Digimon to target
            owner = card.owner
            if owner and not any(p.is_digimon for p in owner.battle_area):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Pay cost: suspend this tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            def on_select_digimon(target_perm):
                # Grant Piercing and Blocker until end of opponent's turn.
                # turn_count + 1: active during current turn (T) and opponent's turn (T+1),
                # expires when our next turn begins (T+2 > T+1).
                grant_expiry = game.turn_count + 1
                target_perm.grant_keyword('_is_piercing', duration=grant_expiry)
                target_perm.grant_keyword('_is_blocker', duration=grant_expiry)
                game.logger.log(
                    f"[EX7-064] {target_perm.top_card.card_names[0] if target_perm.top_card else 'Digimon'} "
                    f"gained <Piercing> and <Blocker> until end of opponent's turn."
                )

                # Check for Vortex Warriors trait — if so, unsuspend
                top = target_perm.top_card
                traits = getattr(top, 'card_traits', []) or [] if top else []
                if any('Vortex Warriors' in t for t in traits):
                    target_perm.unsuspend()
                    game.logger.log(
                        f"[EX7-064] {top.card_names[0] if top and top.card_names else 'Digimon'} "
                        f"has [Vortex Warriors] — unsuspending."
                    )

            game.effect_select_own_permanent(
                player, on_select_digimon,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your Digimon to gain <Piercing> and <Blocker>.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX7-064 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
