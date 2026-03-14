from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_14(CardScript):
    """ST19-14 Arisa Kinosaki | Tamer (Yellow, LIBERATOR, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.

    [Your Turn] When effects play one of your Tokens or a Digimon with the
    [Puppet] trait, by suspending this Tamer, 1 of those Digimon gains <Rush>
    for the turn.

    Security Effect:
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("ST19-14 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When effects play Token/Puppet Digimon,
        #     suspend this Tamer to grant Rush ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST19-14 Grant Rush to played Token/Puppet")
        effect1.set_effect_description(
            "[Your Turn] When effects play one of your Tokens or a Digimon "
            "with the [Puppet] trait, by suspending this Tamer, 1 of those "
            "Digimon gains <Rush> for the turn."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # This tamer must not already be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The played permanent must be ours and played by an effect
            played_perm = context.get('played_permanent') or context.get('event_permanent')
            if played_perm is None:
                return False
            if played_perm not in owner.battle_area:
                return False
            # Must be a Token or a Digimon with Puppet trait
            if played_perm.is_token:
                return True
            if played_perm.is_digimon and played_perm.top_card:
                traits = getattr(played_perm.top_card, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, grant Rush to the played Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            # Pay cost: suspend this Tamer
            tamer_perm.suspend()
            # Grant Rush to the played permanent
            played_perm = ctx.get('played_permanent') or ctx.get('event_permanent')
            if played_perm:
                played_perm.grant_keyword('_is_rush')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("ST19-14 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this tamer from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
