from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_081(CardScript):
    """BT17-081 Tai Kamiya & Matt Ishida | Tamer Red/Blue

    [All Turns] When one of your Digimon is played or digivolves, by
        suspending this Tamer: if you have a Digimon with [Greymon] in its
        name, gain 1 memory. If you have a Digimon with [Garurumon] in its
        name, gain 1 memory.
    [End of Your Turn][Once Per Turn] 1 of your Digimon with [Omnimon] in
        its name may attack a player.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] When own Digimon played/digivolves,
        #     suspend -> gain memory for Greymon/Garurumon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-081 Suspend to gain memory (Greymon/Garurumon)")
        effect0.set_effect_description(
            "[All Turns] When one of your Digimon is played or digivolves, "
            "by suspending this Tamer, if you have [Greymon] gain 1 memory, "
            "if you have [Garurumon] gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            # Suspend cost: must not already be suspended
            if perm.is_suspended:
                return False
            # Triggering permanent must be one of our Digimon (played or digivolved)
            trigger_perm = context.get('played_permanent')
            if trigger_perm is None:
                return False
            if not trigger_perm.is_digimon:
                return False
            if not (card and card.owner):
                return False
            # Must be on our battle area
            if trigger_perm not in card.owner.battle_area:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Pay cost: suspend this Tamer
            perm.suspend()

            # Check for [Greymon] named Digimon
            has_greymon = any(
                p.is_digimon and p.top_card and
                p.top_card.contains_card_name('Greymon')
                for p in player.battle_area
            )
            if has_greymon:
                player.add_memory(1)

            # Check for [Garurumon] named Digimon
            has_garurumon = any(
                p.is_digimon and p.top_card and
                p.top_card.contains_card_name('Garurumon')
                for p in player.battle_area
            )
            if has_garurumon:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [End of Your Turn][Once Per Turn] Omnimon attacks player ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT17-081 End of Turn: Omnimon attacks player")
        effect1.set_effect_description(
            "[End of Your Turn][Once Per Turn] 1 of your Digimon with "
            "[Omnimon] in its name may attack a player."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Attack_BT17_081")

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have an Omnimon that can attack
            has_omnimon = any(
                p.is_digimon and p.top_card and
                p.top_card.contains_card_name('Omnimon') and
                p.can_attack()
                for p in card.owner.battle_area
            )
            return has_omnimon

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def omnimon_filter(p):
                return (p.is_digimon and p.top_card and
                        p.top_card.contains_card_name('Omnimon') and
                        p.can_attack())

            def on_select_omnimon(target_perm):
                # Grant MAY_ATTACK (optional end-of-turn attack) + Rush + unsuspend
                # Card says "may attack a player" — player-only, so also restrict
                # Digimon targeting via _is_cannot_attack_digimon keyword grant
                from ....interfaces.modifiers import ModifierType
                if target_perm.is_suspended:
                    target_perm.unsuspend()
                target_perm.grant_keyword('_is_rush')
                target_perm.grant_keyword(
                    '_is_cannot_attack_digimon', game.turn_count)
                game.register_modifier(
                    target_perm, ModifierType.MAY_ATTACK,
                    expiry='end_of_turn')

            game.effect_select_own_permanent(
                player, on_select_omnimon,
                filter_fn=omnimon_filter,
                is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT17-081 Security: Play this Tamer")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
