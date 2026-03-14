from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_080(CardScript):
    """BT24-080 Megidramon | Lv.6 Purple Digimon | Evil Dragon, Four Great Dragons

    This card is also treated as [ChaosGallantmon].

    [Trash] [End of Your Turn] If you have 4 or fewer cards in your hand,
        1 of your [Dark Dragon] or [Evil Dragon] Digimon may digivolve into
        this card without paying the cost.

    <Blocker>

    [On Play] [When Digivolving] [On Deletion] Delete all of your opponent's
        lowest level Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['ChaosGallantmon']

        # --- Effect 0: Also treated as [ChaosGallantmon] ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-080 Also treated as [ChaosGallantmon]")
        effect0.set_effect_description("Also Treated As [ChaosGallantmon]")
        # Name aliasing: engine checks contains_card_name which does substring.
        # Register alias so ChaosGallantmon's EOT can find this card.
        effect0._also_treated_as = "ChaosGallantmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # descriptive-tagged: also_treated_as_name
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Trash] [End of Your Turn] Digivolve from trash ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name(
            "BT24-080 1 [Dark Dragon] or [Evil Dragon] may digivolve into this")
        effect1.set_effect_description(
            "[Trash] [End of Your Turn] If you have 4 or fewer cards in your "
            "hand, 1 of your [Dark Dragon] or [Evil Dragon] trait Digimon may "
            "digivolve into this card without paying the cost.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            # Must be in trash
            if card not in player.trash_cards:
                return False
            # Must have 4 or fewer cards in hand
            if len(player.hand_cards) > 4:
                return False
            # Must have a Dark Dragon or Evil Dragon Digimon on field
            has_target = any(
                p.is_digimon and p.top_card and
                any('Dark Dragon' in t or 'Evil Dragon' in t
                    for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in player.battle_area
            )
            return has_target
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Digivolve a Dark Dragon/Evil Dragon Digimon into this card from trash."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if card not in player.trash_cards:
                return

            def perm_filter(p):
                if not p.is_digimon:
                    return False
                tc = p.top_card
                if not tc:
                    return False
                traits = getattr(tc, 'card_traits', []) or []
                return any('Dark Dragon' in t or 'Evil Dragon' in t
                           for t in traits)

            def on_select(target_perm):
                if target_perm is None:
                    return
                # Remove this card from trash and digivolve onto the target
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    target_perm.add_card_source(card)
                    target_perm.turn_digivolved = game.turn_count
                    top_name = target_perm.top_card.card_names[0] if (target_perm.top_card and target_perm.top_card.card_names) else '?'
                    game.logger.log(
                        f"[Effect Digivolve] Megidramon onto "
                        f"{top_name} (cost: 0, from trash)")
                    player.draw()
                    game.execute_effects(
                        EffectTiming.WhenDigivolving,
                        {"digivolved_permanent": target_perm})

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=perm_filter,
                is_optional=True,
                prompt="Select 1 [Dark Dragon]/[Evil Dragon] Digimon to "
                       "digivolve into Megidramon.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Blocker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-080 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Shared: Delete all opponent's lowest level Digimon ---
        def _delete_all_lowest_level(ctx: Dict[str, Any]):
            """Delete ALL of opponent's Digimon at the lowest level."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            # Find the minimum level
            levels = [p.level for p in opp_digimon
                      if p.level is not None]
            if not levels:
                return
            min_level = min(levels)
            # Delete all at that level
            to_delete = [p for p in opp_digimon
                         if p.level is not None and p.level == min_level]
            for perm in to_delete:
                enemy.delete_permanent(perm)

        # --- Effect 3: [On Play] Delete all lowest level ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-080 On Play: delete all lowest level")
        effect3.set_effect_description(
            "[On Play] Delete all of your opponent's lowest level Digimon.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_delete_all_lowest_level)
        effects.append(effect3)

        # --- Effect 4: [When Digivolving] Delete all lowest level ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT24-080 When Digivolving: delete all lowest level")
        effect4.set_effect_description(
            "[When Digivolving] Delete all of your opponent's lowest level Digimon.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_delete_all_lowest_level)
        effects.append(effect4)

        # --- Effect 5: [On Deletion] Delete all lowest level ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDestroyedAnyone)
        effect5.set_effect_name("BT24-080 On Deletion: delete all lowest level")
        effect5.set_effect_description(
            "[On Deletion] Delete all of your opponent's lowest level Digimon.")
        effect5.is_on_deletion = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_delete_all_lowest_level)
        effects.append(effect5)

        return effects
