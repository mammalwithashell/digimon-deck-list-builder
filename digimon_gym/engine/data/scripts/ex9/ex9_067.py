from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_067(CardScript):
    """EX9-067 Mirai Kinosaki | Tamer | Yellow | Cost 3

    [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [Puppet]
        or [LIBERATOR] trait among them to the hand. Return the rest to the
        bottom of the deck.

    [Your Turn] When any of your Digimon digivolve into a [Puppet] trait
        Digimon, by returning this Tamer to the bottom of the deck, you may
        play 1 [Arisa Kinosaki] or 1 [Puppet] trait Digimon card from your
        hand with the play cost reduced by 3.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        def _is_liberator_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('LIBERATOR' in t for t in traits)

        # --- Effect 0: [On Play] Reveal top 3, add 1 [Puppet] or [LIBERATOR] to hand ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-067 Reveal top 3, add Puppet/LIBERATOR")
        effect0.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with the "
            "[Puppet] or [LIBERATOR] trait among them to the hand. Return the rest "
            "to the bottom of the deck."
        )
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def puppet_or_liberator_filter(c):
                return _is_puppet_trait(c) or _is_liberator_trait(c)

            def on_selected(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3,
                filter_fn=puppet_or_liberator_filter,
                on_selected=on_selected,
                is_optional=False,
                prompt="Select 1 card with [Puppet] or [LIBERATOR] trait to add to hand."
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When your Digimon digivolves into [Puppet],
        #     return this Tamer to deck bottom, play Arisa Kinosaki or Puppet from hand cost -3 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-067 On Puppet digivolve: return Tamer, play from hand cost -3")
        effect1.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve into a [Puppet] trait "
            "Digimon, by returning this Tamer to the bottom of the deck, you may "
            "play 1 [Arisa Kinosaki] or 1 [Puppet] trait Digimon card from your "
            "hand with the play cost reduced by 3."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The digivolving Digimon must be one of our own and must have digivolved into [Puppet]
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != card.owner:
                return False
            if not trigger_perm.is_digimon:
                return False
            if trigger_perm.top_card and _is_puppet_trait(trigger_perm.top_card):
                return True
            return False
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Return this Tamer to the bottom of the deck
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return
            if my_perm in player.battle_area:
                player.battle_area.remove(my_perm)
            for cs in my_perm.card_sources:
                player.library_cards.append(cs)

            # Play 1 [Arisa Kinosaki] or [Puppet] Digimon from hand with cost -3
            def play_filter(c):
                # Arisa Kinosaki tamer
                if getattr(c, 'is_tamer', False):
                    names = getattr(c, 'card_names', []) or []
                    if any('Arisa Kinosaki' in n for n in names):
                        return True
                # Puppet Digimon
                if getattr(c, 'is_digimon', False) and _is_puppet_trait(c):
                    return True
                return False

            qualifying = [c for c in player.hand_cards if play_filter(c)]
            if not qualifying:
                return

            # Use simplified: play first qualifying card with cost -3
            chosen = qualifying[0]
            base_cost = getattr(chosen, 'get_cost_itself', 0)
            if callable(base_cost):
                base_cost = base_cost
            else:
                base_cost = int(base_cost)
            reduced_cost = max(0, base_cost - 3)

            player.hand_cards.remove(chosen)
            played_perm = player.play_card_from_source(chosen, pay_cost=False)
            player.lose_memory(reduced_cost)
            if played_perm:
                game.execute_effects(EffectTiming.OnEnterFieldAnyone,
                    {"played_card": chosen, "played_permanent": played_perm, "event_player": player})

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX9-067 Security: Play this card free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
