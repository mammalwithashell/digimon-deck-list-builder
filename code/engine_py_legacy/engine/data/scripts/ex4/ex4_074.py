from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....interfaces.modifiers import ModifierType

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_074(CardScript):
    """EX4-074 ShineGreymon: Ruin Mode | Lv.7 Purple/Yellow Digimon

    Alt digivolve: from [ShineGreymon] for cost 4.

    [When Digivolving] [On Deletion] Until the end of your opponent's next
        turn, all of your opponent's Digimon get -5000DP.

    [End of Attack] Delete this Digimon and 1 of your opponent's Digimon,
        and Recovery +1 (Deck). Then, if you have a Tamer in play, hatch 1
        Digi-Egg card to an empty space in your breeding area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [ShineGreymon] for cost 4 ---
        # Validated by digivolve_validator via `_alt_digi_name`; the
        # can_use_condition here is unconditional per the standard alt-digi
        # pattern (validator reads the `_alt_digi_*` attributes).
        effect0 = ICardEffect()
        effect0.set_effect_name("EX4-074 Alternate digivolution requirement")
        effect0.set_effect_description(
            "Alternate digivolution: from [ShineGreymon] for cost 4")
        effect0._alt_digi_cost = 4
        effect0._alt_digi_name = "ShineGreymon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _apply_minus_5000(ctx, source_effect):
            """Register -5000 DP CHANGE_DP modifier on every current opponent
            Digimon, expiring at the end of the opponent's (next) turn.

            Uses per-target registration with a target-equality guard so the
            debuff can never leak to unrelated permanents (checklist item 14).
            The modifier's source_permanent is the TARGET opp Digimon — so
            when EX4-074 itself leaves the field (e.g., after the [On Deletion]
            trigger) its cleanup_modifiers_for_permanent call will not affect
            these entries; they persist until the natural expiry.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                game.register_modifier(
                    opp_perm,
                    ModifierType.CHANGE_DP,
                    condition=lambda p, c, tp=opp_perm: p is tp,
                    value_fn=lambda cur, t, c: cur - 5000,
                    source_effect=source_effect,
                    expiry='end_of_opponent_turn',
                )

        # --- Effect 1: [When Digivolving] opponent Digimon get -5000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX4-074 When Digivolving: opponent -5000 DP")
        effect1.set_effect_description(
            "[When Digivolving] Until the end of your opponent's next turn, "
            "all of your opponent's Digimon get -5000DP."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(lambda ctx: _apply_minus_5000(ctx, effect1))
        effects.append(effect1)

        # --- Effect 2: [On Deletion] opponent Digimon get -5000 DP ---
        # No field-presence guard: [On Deletion] fires after the card has
        # already left the battle area (it's being destroyed).
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX4-074 On Deletion: opponent -5000 DP")
        effect2.set_effect_description(
            "[On Deletion] Until the end of your opponent's next turn, "
            "all of your opponent's Digimon get -5000DP."
        )
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(lambda ctx: _apply_minus_5000(ctx, effect2))
        effects.append(effect2)

        # --- Effect 3: [End of Attack] Delete self + 1 opponent + recovery + hatch ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndAttack)
        effect3.set_effect_name("EX4-074 End of Attack: delete self+opponent, recovery, hatch")
        effect3.set_effect_description(
            "[End of Attack] Delete this Digimon and 1 of your opponent's "
            "Digimon, and <Recovery +1 (Deck)>. Then, if you have a Tamer in "
            "play, hatch 1 Digi-Egg card to an empty space in your breeding "
            "area."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            # Field-presence check (checklist item 15).
            if card and card.permanent_of_this_card() is None:
                return False
            # OnEndAttack target-equality: only trigger for THIS card's attack.
            perm = card.permanent_of_this_card()
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm is not ctx_perm:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def _recovery_and_hatch(player):
            """Recovery +1 (Deck) then conditional hatch."""
            player.recovery(1)
            has_tamer = any(p.is_tamer for p in player.battle_area)
            if has_tamer and player.breeding_area is None:
                player.hatch()

        def process3(ctx: Dict[str, Any]):
            """Delete self, then select 1 opp Digimon to delete, then
            Recovery +1, then hatch if tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # 1. Delete this Digimon first (matches C# coroutine order).
            self_perm = card.permanent_of_this_card() if card else None
            if self_perm and self_perm in player.battle_area:
                player.delete_permanent(self_perm)

            # 2. Select 1 opp Digimon to delete (player agency — no
            #    auto-selection). If none available, skip directly to recovery.
            def _after_opp_delete(target_perm):
                if target_perm and target_perm in enemy.battle_area:
                    enemy.delete_permanent(target_perm)
                _recovery_and_hatch(player)

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                game.effect_select_opponent_permanent(
                    player, _after_opp_delete,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's Digimon.",
                )
            else:
                _recovery_and_hatch(player)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
