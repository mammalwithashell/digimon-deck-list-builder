from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_12(CardScript):
    """ST19-12 Cendrillmon | Lv.6 (Yellow, Puppet/LIBERATOR)

    <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of
    your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player
    without suspending.)

    <Blocker>

    [When Digivolving] You may play 2 [Familiar] Tokens.
    (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets
    -3000 DP for the turn.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Overclock ([Puppet] Trait)> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST19-12 Overclock")
        effect0.set_effect_description(
            "Overclock ([Puppet] Trait)"
        )
        effect0._is_overclock = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: <Blocker> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST19-12 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Play 2 Familiar Tokens ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("ST19-12 Play 2 Familiar Tokens")
        effect2.set_effect_description(
            "[When Digivolving] You may play 2 [Familiar] Tokens. "
            "(Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's "
            "Digimon gets -3000 DP for the turn.)"
        )
        effect2.is_optional = True
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Need at least 1 empty field slot
            from ....game.constants import FIELD_SLOTS
            return len(owner.battle_area) < FIELD_SLOTS

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play 2 Familiar Tokens."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_play_token(player, 'familiar', count=2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
