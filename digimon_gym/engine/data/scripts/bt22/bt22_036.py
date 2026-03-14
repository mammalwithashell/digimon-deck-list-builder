from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

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

        # --- Effect 1: [Hand][Main] Digivolve from Shoemon via ShoeShoemon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1.set_effect_name("BT22-036 Place ShoeShoemon from trash into Shoemon, digivolve")
        effect1.set_effect_description("[Hand][Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon] from your trash as any of your [Shoemon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")
        effect1.is_optional = True

        effect = effect1
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            # Must have Arisa Kinosaki on field
            has_arisa = any(
                p.is_tamer and p.top_card and
                any('Arisa Kinosaki' in n for n in getattr(p.top_card, 'card_names', []))
                for p in player.battle_area
            )
            if not has_arisa:
                return False
            # Must have Shoemon on field
            has_shoemon = any(
                p.is_digimon and p.top_card and
                any('Shoemon' in n and 'ShoeShoemon' not in n for n in getattr(p.top_card, 'card_names', []))
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
            player = ctx.get('player')
            game = ctx.get('game')
            perm = ctx.get('permanent')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): WhenRemoveField - Delete Token/Puppet to prevent leaving ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT22-036 Delete Token/Puppet to prevent leaving")
        effect2.set_effect_description("[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, it doesn't leave.")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("BT22_036_Substitute")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            my_perm = card.permanent_of_this_card()

            def sub_filter(p):
                if not p.is_digimon:
                    return False
                if p is my_perm:
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
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def sub_filter(p):
                if not p.is_digimon:
                    return False
                if p is my_perm:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                if p.top_card and _is_puppet_trait(p.top_card):
                    return True
                return False

            def on_delete_substitute(target_perm):
                player.delete_permanent(target_perm)
                if my_perm and hasattr(my_perm, 'willBeRemoveField'):
                    my_perm.willBeRemoveField = False
                if my_perm and hasattr(my_perm, 'will_be_removed'):
                    my_perm.will_be_removed = False

            game.effect_select_own_permanent(
                player, on_delete_substitute, filter_fn=sub_filter,
                is_optional=True,
                prompt="Select 1 Token or [Puppet] Digimon to delete to prevent leaving."
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
