from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST18_14(CardScript):
    """ST18-14 Shoto Kazama | Tamer (Green, LIBERATOR, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [Your Turn] When one of your Digimon attacks your opponent's Digimon, by
        suspending this Tamer, you may change the attack target to another of
        your opponent's Digimon or the player.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] If memory <= 2, set to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("ST18-14 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
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
            if player and game and game.memory <= 2:
                game.memory = 3

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When own Digimon attacks opponent's Digimon,
        #     by suspending this tamer, change attack target ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name(
            "ST18-14 Suspend tamer to change attack target"
        )
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon attacks your opponent's Digimon, "
            "by suspending this Tamer, you may change the attack target to another "
            "of your opponent's Digimon or the player."
        )
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # This tamer must not be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # Attacker must be our Digimon attacking an opponent's Digimon
            game = context.get('game')
            attacker = context.get('attacker')
            if not (game and attacker):
                return False
            # Verify attacker is one of our Digimon
            owner = card.owner
            if not owner:
                return False
            if attacker not in owner.battle_area:
                return False
            if not attacker.is_digimon:
                return False
            # Original target must be an opponent's Digimon (not the player)
            pa = game.pending_attack
            if not pa:
                return False
            from ....core.permanent import Permanent
            if not isinstance(pa.original_target, Permanent):
                return False
            # Must be another valid target (another opponent Digimon or the player)
            enemy = owner.enemy
            if not enemy:
                return False
            other_digimon = [p for p in enemy.battle_area if p is not pa.original_target]
            # Player is always a valid alternate target
            has_alternatives = len(other_digimon) > 0 or True  # player always valid
            return has_alternatives

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            pa = game.pending_attack
            if not pa:
                return

            # Pay cost: suspend this tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            enemy = player.enemy
            if not enemy:
                return

            # Other opponent Digimon (excluding the original target)
            orig = pa.original_target
            other_digimon = [
                p for p in enemy.battle_area
                if p is not orig and p.is_digimon
            ]

            def apply_new_target(new_target):
                pa.effective_target = new_target
                game.execute_effects(
                    EffectTiming.OnAttackTargetChanged,
                    {"attacker": pa.attacker, "new_target": new_target},
                )
                top = getattr(new_target, 'top_card', None)
                name = top.card_names[0] if top and top.card_names else 'player'
                game.logger.log(f"[ST18-14] Attack target changed to {name}.")

            if len(other_digimon) == 0:
                # No other Digimon — redirect to player directly
                apply_new_target(enemy)
                return

            # Select another opponent Digimon; declining redirects to player
            def on_select_digimon(target_perm):
                apply_new_target(target_perm)

            game.effect_select_opponent_permanent(
                player, on_select_digimon,
                filter_fn=lambda p: p.is_digimon and p is not orig,
                is_optional=True,
                prompt="Select a new attack target (opponent's Digimon), or decline to target the player.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("ST18-14 Security: Play free")
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
