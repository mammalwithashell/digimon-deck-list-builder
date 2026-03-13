from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_027(CardScript):
    """EX7-027 Chaperomon | Lv.5, Yellow, Puppet/LIBERATOR, DP 7000, Cost 7

    <Overclock ([Puppet] Trait)>
    [When Digivolving] You may play 1 level 3 Digimon card with the [Puppet]
        trait from your hand without paying the cost.
    [Inherited][All Turns][Once Per Turn] When this Digimon would leave the
        battle area other than by your effects, by deleting 1 of your Tokens
        or other [Puppet] trait Digimon, prevent it from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Overclock ([Puppet] Trait) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX7-027 Overclock")
        effect0.set_effect_description("Overclock ([Puppet] Trait)")
        effect0._is_overclock = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [When Digivolving] Play 1 Lv.3 [Puppet] Digimon from hand free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX7-027 Play Lv.3 Puppet Digimon from hand")
        effect1.set_effect_description(
            "[When Digivolving] You may play 1 level 3 Digimon card with the "
            "[Puppet] trait from your hand without paying the cost."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play 1 Lv.3 Puppet Digimon from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level != 3:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Inherited][All Turns][Once Per Turn] Prevent leaving by
        #     deleting 1 Token or other [Puppet] Digimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("EX7-027 Inherited: Prevent leaving by deleting Puppet/Token")
        effect2.set_effect_description(
            "[Inherited][All Turns][Once Per Turn] When this Digimon would leave "
            "the battle area other than by your effects, by deleting 1 of your "
            "Tokens or other [Puppet] trait Digimon, prevent it from leaving."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Substitute_EX7_027")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have a Token or other Puppet Digimon to delete
            def _is_substitute(p):
                if p is my_perm or not p.is_digimon:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                top = p.top_card
                if top:
                    traits = getattr(top, 'card_traits', []) or []
                    return any('Puppet' in t for t in traits)
                return False
            has_substitute = any(_is_substitute(p) for p in owner.battle_area)
            return has_substitute

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete 1 Token or other Puppet Digimon to prevent this Digimon from leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            def substitute_filter(p):
                if p is my_perm:
                    return False
                if not p.is_digimon:
                    return False
                if getattr(p, 'is_token', False):
                    return True
                top = p.top_card
                if top:
                    traits = getattr(top, 'card_traits', []) or []
                    return any('Puppet' in t for t in traits)
                return False

            def on_delete_substitute(target_perm):
                player.delete_permanent(target_perm)

            game.effect_select_own_permanent(
                player, on_delete_substitute,
                filter_fn=substitute_filter,
                is_optional=True,
                prompt="Select 1 Token or [Puppet] Digimon to delete to prevent leaving.",
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
