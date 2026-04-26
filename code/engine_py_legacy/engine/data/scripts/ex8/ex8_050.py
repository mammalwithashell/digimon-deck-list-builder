from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_050(CardScript):
    """EX8-050 Gogmamon | Lv.5

    Blocker
    [On Deletion] Reveal the top 3 cards of your deck. You may play 1
    Digimon card with the [Mineral] or [Rock] trait and a play cost of
    5 or less among them without paying the cost. Trash the rest.

    Inherited: [Opponent's Turn] [Once Per Turn] When one of your
    opponent's Digimon attacks, you may change the attack target to
    this Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-050 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Deletion] Reveal top 3, play 1 Mineral/Rock Digimon cost 5-,
        # trash the rest.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX8-050 On Deletion: Reveal 3, play 1 Mineral/Rock Digimon")
        effect1.set_effect_description(
            "[On Deletion] Reveal the top 3 cards of your deck. You may play "
            "1 Digimon card with the [Mineral] or [Rock] trait and a play cost "
            "of 5 or less among them without paying the cost. Trash the rest."
        )
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import GamePhase
            from ....game.constants import SEL_REVEALED_START

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not ('Mineral' in traits or 'Rock' in traits):
                    return False
                try:
                    cost = c.get_cost_itself
                except Exception:
                    cost = getattr(c, 'play_cost', None)
                return cost is not None and cost <= 5

            # Reveal top 3
            revealed = player.library_cards[:3]
            if not revealed:
                return

            game.revealed_cards = list(revealed)

            valid = []
            for i, c in enumerate(revealed):
                if play_filter(c):
                    valid.append(SEL_REVEALED_START + i)

            if not valid:
                # No valid plays: trash all revealed
                for c in revealed:
                    player.library_cards.remove(c)
                    player.trash_cards.append(c)
                game.revealed_cards = []
                return

            def on_select(action_id):
                idx = action_id - SEL_REVEALED_START
                if 0 <= idx < len(revealed):
                    selected = revealed[idx]
                    remaining = [c for c in revealed if c is not selected]
                    for c in revealed:
                        if c in player.library_cards:
                            player.library_cards.remove(c)
                    # Play selected card for free
                    played_perm = player.play_card_from_source(selected, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(selected)} from revealed cards")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": selected, "played_permanent": played_perm,
                         "event_player": player, "is_effect_play": True})
                    game._fire_play_observers(played_perm, player)
                    # Trash the rest
                    for c in remaining:
                        player.trash_cards.append(c)
                game.revealed_cards = []

            def on_decline():
                # Trash all revealed cards (card text: "Trash the rest")
                for c in revealed:
                    if c in player.library_cards:
                        player.library_cards.remove(c)
                    player.trash_cards.append(c)
                game.revealed_cards = []

            game.request_selection(
                GamePhase.SelectReveal, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 [Mineral]/[Rock] Digimon (cost 5-) to play free. Rest trashed.",
                on_decline=on_decline)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Inherited: [Opponent's Turn] [Once Per Turn] When opponent attacks,
        # may redirect attack to this Digimon (SwitchDefender pattern)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX8-050 Redirect attack to this Digimon")
        effect2.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's "
            "Digimon attacks, you may change the attack target to this Digimon."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Redirect_EX8_050")
        effect2._is_when_attacked_observer = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or owner.is_my_turn:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Redirect attack to this Digimon via game.redirect_attack."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (perm and game):
                return
            game.redirect_attack(perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
