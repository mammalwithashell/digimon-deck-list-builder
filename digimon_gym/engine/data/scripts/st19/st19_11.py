from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_11(CardScript):
    """ST19-11 Chaperomon | Lv.5 (Yellow, Puppet/LIBERATOR)

    [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
    for the turn. If there are 3 or more Digimon, increase the DP reduction
    of this effect by -3000.

    Inherited Effect:
    [All Turns] [Once Per Turn] When this Digimon would leave the battle area
    other than by your effects, by deleting 1 of your Tokens or other [Puppet]
    trait Digimon, prevent it from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _get_dp_reduction() -> int:
            """Calculate DP reduction: -3000, or -6000 if 3+ total Digimon."""
            owner = card.owner if card else None
            if not owner:
                return -3000
            own_digimon = sum(1 for p in owner.battle_area if p.is_digimon)
            enemy = owner.enemy
            opp_digimon = sum(1 for p in enemy.battle_area if p.is_digimon) if enemy else 0
            total = own_digimon + opp_digimon
            if total >= 3:
                return -6000
            return -3000

        # --- Effect 0: [On Play] Opponent's Digimon gets -3000/-6000 DP ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("ST19-11 On Play: DP reduction")
        effect0.set_effect_description(
            "[On Play] 1 of your opponent's Digimon gets -3000 DP for the "
            "turn. If there are 3 or more Digimon, increase the DP reduction "
            "of this effect by -3000."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon and apply DP reduction."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            reduction = _get_dp_reduction()

            def on_select(target_perm):
                target_perm.change_dp(reduction)

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt=f"Select 1 opponent Digimon to get {reduction} DP.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Same DP reduction ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST19-11 When Digivolving: DP reduction")
        effect1.set_effect_description(
            "[When Digivolving] 1 of your opponent's Digimon gets -3000 DP "
            "for the turn. If there are 3 or more Digimon, increase the DP "
            "reduction of this effect by -3000."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon and apply DP reduction."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            reduction = _get_dp_reduction()

            def on_select(target_perm):
                target_perm.change_dp(reduction)

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt=f"Select 1 opponent Digimon to get {reduction} DP.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [All Turns] [Once Per Turn] Prevent leaving ---
        # When this Digimon would leave the battle area other than by your
        # effects, by deleting 1 of your Tokens or other [Puppet] trait
        # Digimon, prevent it from leaving.
        # Uses WhenPermanentWouldBeDeleted timing + _will_not_be_removed flag
        # (matches EX7-027 canonical pattern)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name("ST19-11 Prevent leaving by deleting Token/Puppet")
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon would leave the "
            "battle area other than by your effects, by deleting 1 of your "
            "Tokens or other [Puppet] trait Digimon, prevent it from leaving."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_EX7_027")

        def _is_substitute(p, my_perm):
            """Check if a permanent can be sacrificed (Token or other Puppet Digimon)."""
            if p is my_perm or not p.is_digimon:
                return False
            if getattr(p, 'is_token', False):
                return True
            top = p.top_card
            if top:
                traits = getattr(top, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)
            return False

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            # Only triggers for THIS permanent being removed
            # In execute_effects context, event_permanent is the permanent being deleted
            event_perm = context.get('event_permanent') or context.get('permanent')
            if event_perm is not my_perm:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # "other than by your effects" — skip if removal is by own effect
            if context.get('is_own_effect', False):
                return False
            # Must have a Token or other Puppet Digimon to sacrifice
            return any(_is_substitute(p, my_perm) for p in owner.battle_area)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete a Token or Puppet Digimon to prevent this Digimon from leaving.

            Pattern: Set _will_not_be_removed = True IMMEDIATELY to prevent
            deletion (since delete_permanent checks the flag synchronously after
            firing WhenPermanentWouldBeDeleted), then start async selection.
            If the player declines, undo prevention by deleting the permanent.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            # Immediately prevent deletion — selection will confirm or undo
            my_perm._will_not_be_removed = True

            # Build valid target list
            from ....game.effects import SEL_MY_FIELD_START
            valid = []
            for i, p in enumerate(player.battle_area):
                if _is_substitute(p, my_perm):
                    valid.append(SEL_MY_FIELD_START + i)
            if not valid:
                # No valid targets — should not reach here (condition checks)
                my_perm._will_not_be_removed = False
                return

            def on_select(action_id: int):
                idx = action_id - SEL_MY_FIELD_START
                if 0 <= idx < len(player.battle_area):
                    target_perm = player.battle_area[idx]
                    player.delete_permanent(target_perm)

            def on_decline():
                # Player chose not to sacrifice — undo prevention, delete the permanent
                player.delete_permanent(my_perm)

            from ....data.enums import GamePhase as GP
            game.request_selection(
                GP.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 Token or [Puppet] Digimon to delete to prevent leaving.",
                on_decline=on_decline,
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
