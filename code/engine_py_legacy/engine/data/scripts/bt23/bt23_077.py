from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_077(CardScript):
    """BT23-077 Sistermon Ciel | Lv.4

    Also Treated As [Sistermon Noir].
    Blocker
    [On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.
    [All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.

    Inherited Effect: ALL of the above (Blocker, On Play delete, All Turns de-digivolve).
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['Sistermon Noir']

        # --- Effect 0: Also Treated As [Sistermon Noir] ---
        # Name aliasing handled by card.also_treated_as_names above.
        # No-op effect for UI/logging visibility.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.NoTiming)
        effect0.set_effect_name("BT23-077 Also treated as [Sistermon Noir]")
        effect0.set_effect_description("Also Treated As [Sistermon Noir]")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Blocker (main) ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.NoTiming)
        effect1.set_effect_name("BT23-077 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 1i: Blocker (inherited) ---
        effect1i = ICardEffect()
        effect1i.set_timing(EffectTiming.NoTiming)
        effect1i.set_effect_name("BT23-077 Blocker (inherited)")
        effect1i.set_effect_description("Blocker")
        effect1i.is_inherited_effect = True
        effect1i._is_blocker = True

        def condition1i(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1i.set_can_use_condition(condition1i)
        effects.append(effect1i)

        # --- Effect 2: [On Play] Delete 1 of opponent's Digimon with play cost 4 or less (main) ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-077 Delete 1 Digimon with a play cost of 4 or less")
        effect2.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def _make_on_play_delete_process():
            """Factory for on-play delete process callback (shared by main and inherited)."""
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def target_filter(p):
                    return (p.is_digimon and p.top_card and
                            p.top_card.get_cost_itself <= 4)

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            return process

        effect2.set_on_process_callback(_make_on_play_delete_process())
        effects.append(effect2)

        # --- Effect 2i: [On Play] Delete (inherited) ---
        effect2i = ICardEffect()
        effect2i.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2i.set_effect_name("BT23-077 Delete 1 Digimon with a play cost of 4 or less (inherited)")
        effect2i.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect2i.is_on_play = True
        effect2i.is_inherited_effect = True

        def condition2i(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2i.set_can_use_condition(condition2i)
        effect2i.set_on_process_callback(_make_on_play_delete_process())
        effects.append(effect2i)

        # --- Effect 3: [All Turns] When this Digimon suspends, <De-Digivolve 1> (main) ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT23-077 <De-Digivolve 1> 1 of your opponent's Digimon")
        effect3.set_effect_description(
            "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS Digimon suspends — use event_permanent (Pattern 5)
            event_perm = context.get('event_permanent', context.get('permanent'))
            owner_perm = card.permanent_of_this_card()
            if owner_perm and event_perm and event_perm is not owner_perm:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def _make_de_digivolve_process():
            """Factory for de-digivolve process callback (shared by main and inherited)."""
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def on_de_digivolve(target_perm):
                    removed = target_perm.de_digivolve(1)
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.trash_cards.extend(removed)

                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True)
            return process

        effect3.set_on_process_callback(_make_de_digivolve_process())
        effects.append(effect3)

        # --- Effect 3i: [All Turns] When this Digimon suspends, <De-Digivolve 1> (inherited) ---
        effect3i = ICardEffect()
        effect3i.set_timing(EffectTiming.OnTappedAnyone)
        effect3i.set_effect_name("BT23-077 <De-Digivolve 1> 1 of your opponent's Digimon (inherited)")
        effect3i.set_effect_description(
            "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect3i.is_inherited_effect = True

        def condition3i(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS Digimon suspends — use event_permanent (Pattern 5)
            event_perm = context.get('event_permanent', context.get('permanent'))
            owner_perm = card.permanent_of_this_card()
            if owner_perm and event_perm and event_perm is not owner_perm:
                return False
            return True

        effect3i.set_can_use_condition(condition3i)
        effect3i.set_on_process_callback(_make_de_digivolve_process())
        effects.append(effect3i)

        return effects
