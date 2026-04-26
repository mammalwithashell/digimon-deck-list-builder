from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_062(CardScript):
    """EX11-062 Shoto Kazama | Tamer, Green, LIBERATOR, Cost 4

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [All Turns] When any Digimon suspend, by suspending this Tamer, if effects
        suspended those Digimon, <Draw 1>. After, 1 of your Digimon with
        [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait
        gets +3000 DP until your opponent's turn ends.
    [Your Turn] While your opponent has no unsuspended Digimon, your <Vortex>
        can also attack players.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- [Start of Your Turn] If memory <= 2, set to 3 ---
        effect_mem = ICardEffect()
        effect_mem.set_timing(EffectTiming.OnStartMainPhase)
        effect_mem.set_effect_name("EX11-062 Set memory to 3")
        effect_mem.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3."
        )

        def cond_mem(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect_mem.set_can_use_condition(cond_mem)

        def process_mem(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect_mem.set_on_process_callback(process_mem)
        effects.append(effect_mem)

        # --- [All Turns] When any Digimon suspend, by suspending this Tamer,
        #     if effects suspended those Digimon: Draw 1. Then buff Avian/Bird/VW ---
        effect_tap = ICardEffect()
        effect_tap.set_effect_name(
            "EX11-062 By suspending this Tamer (if effects suspended): Draw 1, buff Avian/Bird/VW Digimon +3k DP"
        )
        effect_tap.set_effect_description(
            "[All Turns] When any Digimon suspend, by suspending this Tamer, "
            "if effects suspended those Digimon, <Draw 1>. After, 1 of your Digimon "
            "with [Avian] or [Bird] in traits or [Vortex Warriors] trait gets +3000 DP "
            "until your opponent's turn ends."
        )
        effect_tap.is_optional = True
        effect_tap._is_suspend_observer = True

        def cond_tap(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm or tamer_perm.is_suspended:
                return False
            suspended_perm = context.get('suspended_permanent')
            if not suspended_perm or not suspended_perm.is_digimon:
                return False
            if getattr(suspended_perm, 'is_attacking', False):
                return False
            return True

        effect_tap.set_can_use_condition(cond_tap)

        def process_tap(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()
            # Draw 1
            player.draw_cards(1)
            # 1 of your Digimon with [Avian]/[Bird] in traits or [Vortex Warriors] gets +3000 DP
            from ....interfaces.modifiers import ModifierType

            def buff_filter(p):
                if not p.is_digimon or not p.top_card:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return (
                    any('Avian' in t or 'Bird' in t for t in traits)
                    or any('Vortex Warriors' in t for t in traits)
                )

            def on_buff(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    value_fn=lambda: 3000,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_buff,
                filter_fn=buff_filter,
                is_optional=False,
                prompt="Choose 1 of your Avian/Bird or Vortex Warriors Digimon to get +3000 DP.",
            )

        effect_tap.set_on_process_callback(process_tap)
        effects.append(effect_tap)

        # --- [Your Turn] While opponent has no unsuspended Digimon,
        #     your <Vortex> can also attack players. ---
        effect_cont = ICardEffect()
        effect_cont.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_cont.set_effect_name(
            "EX11-062 [Your Turn] Vortex can attack players while opponent has no unsuspended Digimon"
        )
        effect_cont.set_effect_description(
            "[Your Turn] While your opponent has no unsuspended Digimon, "
            "your <Vortex> can also attack players."
        )
        effect_cont.is_on_play = True

        def cond_cont(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            played_perm = context.get('played_permanent') or context.get('permanent')
            this_perm = card.permanent_of_this_card() if card else None
            if played_perm is not this_perm:
                return False
            return True

        effect_cont.set_can_use_condition(cond_cont)

        def process_cont(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            tamer_perm = card.permanent_of_this_card() if card else None
            if not (player and game and tamer_perm):
                return
            from ....interfaces.modifiers import ModifierType

            def has_vortex(perm):
                return perm.is_digimon and perm.has_keyword('_is_vortex')

            def condition_fn(target_perm, context_inner):
                owner = card.owner if card else None
                if not (owner and owner.is_my_turn):
                    return False
                enemy = owner.enemy if owner else None
                if not enemy:
                    return True
                has_unsuspended_opp = any(
                    p.is_digimon and not p.is_suspended
                    for p in enemy.battle_area
                )
                return not has_unsuspended_opp

            for vortex_perm in list(player.battle_area):
                if has_vortex(vortex_perm):
                    game.register_modifier(
                        vortex_perm,
                        ModifierType.VORTEX_CAN_ATTACK_PLAYERS,
                        condition=condition_fn,
                        value_fn=lambda: True,
                        source_effect=effect_cont,
                        expiry='permanent',
                    )

        effect_cont.set_on_process_callback(process_cont)
        effects.append(effect_cont)

        # --- [Security] Play this card without paying the cost ---
        effect_sec = ICardEffect()
        effect_sec.set_effect_name("EX11-062 Security: Play this card")
        effect_sec.set_effect_description("Security: Play this card")
        effect_sec.is_security_effect = True

        def cond_sec(context: Dict[str, Any]) -> bool:
            return True
        effect_sec.set_can_use_condition(cond_sec)
        effects.append(effect_sec)

        return effects
