from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_068(CardScript):
    """BT17-068 Mephistomon | Lv.5 Purple | Fallen Angel, Virus | 7000 DP | Cost 8

    [Main] While this card is revealed from your deck, this card is also
        treated as level 6.
    When this card would be played from the hand, by returning 1 [Apocalymon]
        from your trash to the bottom of the deck, reduce the play cost by 3.
    [On Deletion] If deleted by an effect, you may play 1 [Gulfmon] or 1 level 6
        Digimon with the [Dark Masters] trait from your hand or trash without
        paying the cost.
    Inherited: [When Attacking] [Once Per Turn] By placing 1 level 5 or lower
        card with [Dark Masters] in its text from your trash as this Digimon's
        bottom digivolution card, this Digimon gets +2000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: While revealed, treated as level 6 ---
        # This is a static/passive effect the engine reads during reveal checks.
        # Modeled as descriptive; engine-level reveal handling uses card data.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-068 Treated as level 6 when revealed")
        effect0.set_effect_description(
            "While this card is revealed from your deck, this card is also "
            "treated as level 6."
        )
        effect0._revealed_level_override = 6

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: BeforePayCost - Return Apocalymon from trash to reduce cost by 3 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT17-068 Cost reduction via Apocalymon")
        effect1.set_effect_description(
            "When this card would be played from the hand, by returning 1 "
            "[Apocalymon] from your trash to the bottom of the deck, reduce "
            "the play cost by 3."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Must have an Apocalymon in trash
            for tc in player.trash_cards:
                names = getattr(tc, 'card_names', []) or []
                if any('Apocalymon' in n for n in names):
                    return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Return 1 Apocalymon from trash to deck bottom, reduce play cost by 3."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Find an Apocalymon in trash
            target = None
            for tc in player.trash_cards:
                names = getattr(tc, 'card_names', []) or []
                if any('Apocalymon' in n for n in names):
                    target = tc
                    break
            if target:
                player.trash_cards.remove(target)
                player.library_cards.append(target)
                # Apply cost reduction
                if hasattr(player, '_temp_play_cost_reduction'):
                    player._temp_play_cost_reduction += 3
                else:
                    player._temp_play_cost_reduction = 3
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Deletion] If deleted by effect, play Gulfmon or
        #     Lv.6 [Dark Masters] trait Digimon from hand or trash free ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("BT17-068 On Deletion: Play Gulfmon or Dark Masters Lv.6")
        effect2.set_effect_description(
            "[On Deletion] If deleted by an effect, you may play 1 [Gulfmon] "
            "or 1 level 6 Digimon with the [Dark Masters] trait from your hand "
            "or trash without paying the cost."
        )
        effect2.is_on_deletion = True
        effect2.is_optional = True

        def _is_valid_play_target(c) -> bool:
            """Check if card is Gulfmon or a Lv.6 Digimon with Dark Masters trait."""
            names = getattr(c, 'card_names', []) or []
            if any('Gulfmon' in n for n in names):
                return True
            is_digimon = getattr(c, 'is_digimon', False)
            level = getattr(c, 'level', None)
            if is_digimon and level == 6:
                traits = getattr(c, 'card_traits', []) or []
                if any('Dark Masters' in t for t in traits):
                    return True
            return False

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be deleted by an effect (not by battle)
            if not context.get('is_by_effect', False):
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check hand or trash for valid targets
            for c in player.hand_cards:
                if _is_valid_play_target(c):
                    return True
            for c in player.trash_cards:
                if _is_valid_play_target(c):
                    return True
            return False
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 1 qualifying Digimon from hand or trash without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_play_from_zone(
                player, 'hand_or_trash', _is_valid_play_target,
                free=True, is_optional=True)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [When Attacking] [Once Per Turn] ---
        # Place 1 Lv.5 or lower card with "Dark Masters" in its text from
        # trash as bottom digi card -> +2000 DP for the turn.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT17-068 Inherited: Place Dark Masters from trash +2000 DP")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] By placing 1 level 5 or lower "
            "card with [Dark Masters] in its text from your trash as this "
            "Digimon's bottom digivolution card, this Digimon gets +2000 DP "
            "for the turn."
        )
        effect3.is_inherited_effect = True
        effect3.is_on_attack = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WhenAttacking_BT17_068")

        def _is_dark_masters_text_lv5_or_lower(c) -> bool:
            level = getattr(c, 'level', None)
            if level is None or level > 5:
                return False
            text = getattr(c, 'card_text', '') or ''
            if 'Dark Masters' in text:
                return True
            return False

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return any(_is_dark_masters_text_lv5_or_lower(c) for c in player.trash_cards)
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Place 1 qualifying card from trash as bottom digi card, +2000 DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Find qualifying card in trash
            target = None
            for tc in player.trash_cards:
                if _is_dark_masters_text_lv5_or_lower(tc):
                    target = tc
                    break
            if target:
                player.trash_cards.remove(target)
                perm.add_card_source_bottom(target)
                # Grant +2000 DP for the turn
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CHANGE_DP,
                    value_fn=lambda: 2000, expiry='end_of_turn')
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
