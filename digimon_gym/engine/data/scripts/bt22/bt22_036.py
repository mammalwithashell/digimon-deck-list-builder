from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_036(CardScript):
    """BT22-036 Chaperomon | Lv.5

    <Overclock ([Puppet] Trait)>

    [Hand][Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon]
        from your trash as any of your [Shoemon]'s bottom digivolution card,
        it digivolves into this card for a digivolution cost of 3, ignoring
        digivolution requirements.

    --- Inherited ---
    [All Turns][Once Per Turn] When this Digimon would leave the battle area
        other than by your effects, by deleting 1 of your Tokens or other
        [Puppet] trait Digimon, it doesn't leave.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: Overclock ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-036 Overclock")
        effect0.set_effect_description("Overclock")
        effect0._is_overclock = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Hand][Main] Place ShoeShoemon from trash under Shoemon,
        #     digivolve Shoemon into this card for cost 3, ignoring requirements ---
        effect1 = ICardEffect()
        effect1._is_hand_main = True
        effect1.set_effect_name("BT22-036 [Hand][Main] Place ShoeShoemon, digivolve onto Shoemon")
        effect1.set_effect_description(
            "[Hand][Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon] "
            "from your trash as any of your [Shoemon]'s bottom digivolution card, "
            "it digivolves into this card for a digivolution cost of 3, ignoring "
            "digivolution requirements.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card.permanent_of_this_card() is not None:
                return False  # must be in hand
            player = card.owner
            if not player or not player.is_my_turn:
                return False
            # Must have Arisa Kinosaki on field
            has_arisa = any(
                p.is_tamer and p.top_card and
                any('Arisa Kinosaki' in n for n in getattr(p.top_card, 'card_names', []))
                for p in player.battle_area
            )
            if not has_arisa:
                return False
            # Must have Shoemon on field (not ShoeShoemon)
            has_shoemon = any(
                p.is_digimon and p.top_card and
                any('Shoemon' in n and 'ShoeShoemon' not in n
                    for n in getattr(p.top_card, 'card_names', []))
                for p in player.battle_area
            )
            if not has_shoemon:
                return False
            # Must have ShoeShoemon in trash
            has_shoeshoemon = any(
                any('ShoeShoemon' in n for n in getattr(c, 'card_names', []))
                for c in player.trash_cards
            )
            return has_shoeshoemon
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            game = ctx.get('game')
            player = ctx.get('player')
            hand_card = ctx.get('card')
            if not (game and player and hand_card):
                return

            def _is_shoeshoemon(c):
                names = getattr(c, 'card_names', []) or []
                return any('ShoeShoemon' in (n or '') for n in names)

            # Let agent choose which ShoeShoemon from trash
            def on_shoeshoemon_selected(trash_idx):
                if trash_idx >= len(player.trash_cards):
                    return
                shoeshoemon = player.trash_cards[trash_idx]

                # Select a Shoemon on field to digivolve onto
                def on_shoemon_selected(target_perm):
                    # Place ShoeShoemon from trash as bottom digi card
                    if shoeshoemon in player.trash_cards:
                        player.trash_cards.remove(shoeshoemon)
                    target_perm.add_card_source_bottom(shoeshoemon)
                    game.logger.log(
                        f"[Hand][Main] Placed {game._card_ref(shoeshoemon)} "
                        f"under {game._perm_ref(target_perm)}")

                    # Remove Chaperomon from hand and digivolve onto Shoemon
                    if hand_card in player.hand_cards:
                        player.hand_cards.remove(hand_card)
                    target_perm.add_card_source(hand_card)
                    target_perm.turn_digivolved = game.turn_count
                    player.lose_memory(3)
                    game.logger.log(
                        f"[Hand][Main] Digivolved {game._card_ref(hand_card)} "
                        f"onto {game._perm_ref(target_perm)} (cost: 3)")
                    player.draw()
                    game.execute_effects(EffectTiming.WhenDigivolving,
                                         {"digivolved_permanent": target_perm})

                def _shoemon_filter(p):
                    if not p.is_digimon or not p.top_card:
                        return False
                    names = getattr(p.top_card, 'card_names', []) or []
                    return any('Shoemon' in n and 'ShoeShoemon' not in n
                               for n in names)

                game.effect_select_own_permanent(
                    player, on_shoemon_selected,
                    filter_fn=_shoemon_filter,
                    is_optional=False,
                    prompt="Select a [Shoemon] to digivolve into Chaperomon.")

            # Build valid trash indices for ShoeShoemon cards
            _SEL_TRASH_START = 130
            valid_trash = []
            for i, c in enumerate(player.trash_cards):
                if _is_shoeshoemon(c):
                    valid_trash.append(_SEL_TRASH_START + i)
            if not valid_trash:
                return

            def _on_trash_action(idx):
                # _decode_trash_selection already subtracts SEL_TRASH_START
                on_shoeshoemon_selected(idx)

            game.request_selection(
                GamePhase.SelectTrash, player, _on_trash_action,
                valid_trash, is_optional=False,
                prompt="Select a [ShoeShoemon] from your trash to place under [Shoemon].")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): WhenPermanentWouldBeDeleted ---
        # Delete Token/Puppet to prevent leaving
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name("BT22-036 Delete Token/Puppet to prevent leaving")
        effect2.set_effect_description(
            "[All Turns][Once Per Turn] When this Digimon would leave the battle "
            "area other than by your effects, by deleting 1 of your Tokens or "
            "other [Puppet] trait Digimon, it doesn't leave.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_036_Substitute")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            own_perm = card.permanent_of_this_card()
            # Only triggers for THIS permanent being deleted
            ctx_perm = context.get('permanent')
            if ctx_perm is not own_perm:
                return False
            player = card.owner if card else None
            if not player:
                return False

            def sub_filter(p):
                if not p.is_digimon:
                    return False
                if p is own_perm:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False

            return any(sub_filter(p) for p in player.battle_area)
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if not own_perm:
                return

            # Auto-select first matching Token/Puppet (no selection phase during deletion)
            for p in list(player.battle_area):
                if not p.is_digimon or p is own_perm:
                    continue
                if getattr(p, 'is_token', False) or (p.top_card and _is_puppet_trait(p.top_card)):
                    player.delete_permanent(p)
                    own_perm._will_not_be_removed = True
                    game.logger.log(
                        "[BT22-036] Deleted a Token/Puppet to prevent "
                        "this Digimon from leaving the battle area.")
                    break

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
