from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _has_dark_masters_trait(c) -> bool:
    """Check if a card has the [Dark Masters] trait (any card type)."""
    traits = getattr(c, 'card_traits', []) or []
    return any('Dark Masters' in t for t in traits)


def _has_dark_masters_trait_perm(perm) -> bool:
    """Check if a permanent's top card has the [Dark Masters] trait."""
    top = perm.top_card if perm else None
    if not top:
        return False
    return _has_dark_masters_trait(top)


def _get_distinct_dm_from_field_and_trash(player, max_count=3):
    """Get up to max_count distinct-named Dark Masters cards from battle area and trash.

    Card text: "cards with different names from your battle area or trash"
    - Any card type with [Dark Masters] trait (not just Digimon)
    - Battle area permanents: use top_card for trait check, collect the permanent
    - Trash cards: check trait directly
    """
    seen_names = set()
    result = []  # list of (source, card_or_perm) tuples

    # Check battle area permanents first
    for perm in player.battle_area:
        top = perm.top_card if perm else None
        if not top:
            continue
        if not _has_dark_masters_trait(top):
            continue
        names = getattr(top, 'card_names', []) or []
        name = names[0] if names else None
        if name and name not in seen_names:
            seen_names.add(name)
            result.append(('field', perm))
            if len(result) >= max_count:
                return result

    # Then check trash
    for c in player.trash_cards:
        if not _has_dark_masters_trait(c):
            continue
        names = getattr(c, 'card_names', []) or []
        name = names[0] if names else None
        if name and name not in seen_names:
            seen_names.add(name)
            result.append(('trash', c))
            if len(result) >= max_count:
                return result

    return result


class BT15_102(CardScript):
    """BT15-102 Apocalymon | Lv.7

    When this card would be played, by placing up to 3 [Dark Masters] trait
    cards with different names from your battle area or trash under it, reduce
    the play cost by 4 for each one.

    [End of Your Turn] [Once Per Turn] By placing 1 level 6 or lower card from
    your trash as this Digimon's bottom digivolution card, activate 1 [On Play]
    effect on that card as an effect of this Digimon. Then, trash the top 2 cards
    of your opponent's deck for each of this Digimon's level 6 digivolution cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: BeforePayCost ---
        # Place up to 3 distinct-named Dark Masters cards from battle area or
        # trash under this card, reduce play cost by 4 per card placed.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT15-102 Placing 1 [Dark Masters] to get Play Cost -4")
        effect0.set_effect_description(
            "When this card would be played, by placing up to 3 [Dark Masters] "
            "trait cards with different names from your battle area or trash under "
            "it, reduce the play cost by 4 for each one."
        )
        effect0.set_hash_string("PlayCost-12_BT15_102")

        def _count_distinct_dm():
            """Count distinct Dark Masters card names in owner's battle area and trash."""
            player = card.owner if card else None
            if not player:
                return 0
            return len(_get_distinct_dm_from_field_and_trash(player, 3))

        def _cost_reduction_fn(context):
            return _count_distinct_dm() * 4

        effect0._cost_reduction_value_fn = _cost_reduction_fn

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check battle area
            for perm in player.battle_area:
                if _has_dark_masters_trait_perm(perm):
                    return True
            # Check trash
            for c in player.trash_cards:
                if _has_dark_masters_trait(c):
                    return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Place up to 3 distinct Dark Masters cards from field/trash under this card."""
            player = ctx.get('player')
            if not player:
                return
            to_place = _get_distinct_dm_from_field_and_trash(player, 3)
            collected_cards = []
            for source, item in to_place:
                if source == 'field':
                    # item is a Permanent on the battle area
                    perm = item
                    if perm in player.battle_area:
                        # Remove permanent from battle area, collect all its card sources
                        player.battle_area.remove(perm)
                        # Take the top card as the DM card to place under
                        top = perm.top_card
                        if top:
                            collected_cards.append(top)
                        # Remaining digivolution sources go to trash
                        for cs in perm.card_sources:
                            if cs is not top:
                                player.trash_cards.append(cs)
                elif source == 'trash':
                    # item is a CardSource in trash
                    c = item
                    if c in player.trash_cards:
                        player.trash_cards.remove(c)
                        collected_cards.append(c)

            # Store collected cards for attachment after the permanent is created
            if collected_cards:
                if not hasattr(card, '_pending_digi_sources'):
                    card._pending_digi_sources = []
                card._pending_digi_sources.extend(collected_cards)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: On Play — consume _pending_digi_sources ---
        # This fires after the permanent is created and attaches the
        # collected DM cards as digivolution sources.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT15-102 Attach pending digi sources")
        effect1.set_effect_description(
            "Attach Dark Masters cards placed during cost reduction as digivolution sources."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return bool(getattr(card, '_pending_digi_sources', None))

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Attach pending digi sources to the permanent."""
            perm = ctx.get('permanent') or ctx.get('played_permanent')
            if not perm:
                return
            pending = getattr(card, '_pending_digi_sources', None)
            if not pending:
                return
            for cs in pending:
                perm.add_card_source_bottom(cs)
            card._pending_digi_sources = []

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [End of Your Turn] [Once Per Turn] ---
        # Place 1 Lv.6 or lower card from trash as bottom digi card,
        # activate 1 [On Play] effect on that card, then mill 2 per Lv6 digi card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT15-102 Place 1 digimon from trash under this Digimon's digivolution cards")
        effect2.set_effect_description(
            "[End of Your Turn] [Once Per Turn] By placing 1 level 6 or lower card "
            "from your trash as this Digimon's bottom digivolution card, activate 1 "
            "[On Play] effect on that card as an effect of this Digimon. Then, trash "
            "the top 2 cards of your opponent's deck for each of this Digimon's "
            "level 6 digivolution cards."
        )
        effect2.set_max_count_per_turn(1)
        effect2.is_optional = True

        def _is_lv6_or_lower(c) -> bool:
            level = getattr(c, 'level', None)
            return level is not None and level <= 6

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            return any(_is_lv6_or_lower(c) for c in player.trash_cards)

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Place 1 Lv6- card from trash as bottom digi, activate [On Play], then mill."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            from ....game.constants import SEL_TRASH_START, ACTION_SPACE_SIZE
            from ....data.enums import GamePhase

            valid = []
            for i, tc in enumerate(player.trash_cards):
                if _is_lv6_or_lower(tc) and (SEL_TRASH_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def _on_select(action_id: int):
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    selected_card = player.trash_cards[idx]
                    # Remove from trash and place as bottom digivolution card
                    player.trash_cards.remove(selected_card)
                    perm.add_card_source_bottom(selected_card)

                    # Activate 1 [On Play] effect on the placed card
                    on_play_effects = []
                    for eff in selected_card.effect_list(EffectTiming.OnEnterFieldAnyone):
                        if getattr(eff, 'is_on_play', False):
                            on_play_effects.append(eff)

                    if on_play_effects:
                        if len(on_play_effects) == 1:
                            _activate_on_play_effect(on_play_effects[0], player, perm, game)
                        else:
                            def on_choose(branch_idx):
                                if 0 <= branch_idx < len(on_play_effects):
                                    _activate_on_play_effect(
                                        on_play_effects[branch_idx], player, perm, game)
                            game.effect_choose_branch(
                                player, len(on_play_effects), on_choose,
                                prompt="Select 1 [On Play] effect to activate.")

                    # Mill 2 per Lv6 digivolution card
                    lv6_count = sum(
                        1 for cs in perm.card_sources
                        if cs is not card and getattr(cs, 'level', None) == 6
                    )
                    mill_count = 2 * lv6_count
                    enemy = player.enemy
                    if enemy and mill_count > 0:
                        enemy.mill(mill_count)

            game.request_selection(
                GamePhase.SelectTrash, player, _on_select, valid, True,
                prompt="Select 1 Lv.6 or lower card from trash to place under Apocalymon.")

        def _activate_on_play_effect(effect, player, perm, game):
            """Activate a single [On Play] effect as this Digimon's effect."""
            effect_ctx = {
                'player': player,
                'permanent': perm,
                'game': game,
                'played_permanent': perm,
                'event_player': player,
            }
            cb = effect.on_process_callback
            if cb:
                cb(effect_ctx)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
