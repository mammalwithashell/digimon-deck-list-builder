from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_059(CardScript):
    """BT17-059 Diaboromon | Lv.6 Mega | White | Unidentified

    [When Digivolving] By placing 1 [Doomsday Clock] from your hand or trash
        as this Digimon's bottom digivolution card, you may play 2 [Diaboromon]
        Tokens without paying the cost.
    [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon
        attacks, you may switch the attack target to 1 of your Digimon with
        [Diaboromon] in its name.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Place Doomsday Clock, play 2 tokens ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-059 Place Doomsday Clock, play 2 tokens")
        effect0.set_effect_description(
            "[When Digivolving] By placing 1 [Doomsday Clock] from your hand or "
            "trash as this Digimon's bottom digivolution card, you may play 2 "
            "[Diaboromon] Tokens without paying the cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have a Doomsday Clock in hand or trash
            for c in list(owner.hand_cards) + list(owner.trash_cards):
                names = getattr(c, 'card_names', []) or []
                if any('Doomsday Clock' in n for n in names):
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Find a Doomsday Clock in hand or trash
            dc_card = None
            dc_zone = None
            for c in player.hand_cards:
                names = getattr(c, 'card_names', []) or []
                if any('Doomsday Clock' in n for n in names):
                    dc_card = c
                    dc_zone = 'hand'
                    break
            if dc_card is None:
                for c in player.trash_cards:
                    names = getattr(c, 'card_names', []) or []
                    if any('Doomsday Clock' in n for n in names):
                        dc_card = c
                        dc_zone = 'trash'
                        break
            if dc_card is None:
                return

            # Place Doomsday Clock as bottom digivolution card
            if dc_zone == 'hand':
                player.hand_cards.remove(dc_card)
            else:
                player.trash_cards.remove(dc_card)
            perm.add_card_source_bottom(dc_card)
            game.logger.log(
                f"[BT17-059] Placed Doomsday Clock from {dc_zone} as bottom "
                f"digivolution card."
            )

            # Play 2 Diaboromon Tokens
            game.effect_play_token(player, 'diaboromon', count=2)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Opponent's Turn] [OPT] Switch attack target ---
        # NOTE: Attack target redirection is not fully supported in the engine.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT17-059 Switch attack target to Diaboromon")
        effect1.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, you may switch the attack target to 1 of your "
            "Digimon with [Diaboromon] in its name."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT17_059_SwitchTarget")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must be opponent's turn
            if owner.is_my_turn:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # descriptive-tagged: redirect_attack
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
