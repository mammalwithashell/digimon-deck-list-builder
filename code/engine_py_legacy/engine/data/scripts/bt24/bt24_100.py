from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_100(CardScript):
    """BT24-100 In-Between Theater | Option (White, Cost 3)

    While you have an [TS] trait Digimon or Tamer on the field, you can ignore
    this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 card with the [TS] trait
    among them to the hand. Return the rest to the bottom of the deck.
    Then, place this card in the battle area.
    [Main] <Delay> Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # --- Ignore color requirements ---
        # Per C#: while owner has a [TS] Digimon or Tamer on field
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce
            has_ts = any(
                (p.is_digimon or p.is_tamer)
                and any('TS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in owner.battle_area
                if p.top_card
            )
            return not has_ts  # True = enforce, False = bypass
        card._match_color_requirement_fn = _check_color_req

        effects = []

        # --- Effect 0: [Main] Reveal top 3, add 1 [TS] card to hand, bottom rest ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT24-100 Reveal top 3, add 1 [TS] card to hand, bottom rest")
        effect0.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 [TS] trait card among them "
            "to the hand. Return the rest to the bottom of the deck. Then, place this card "
            "in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return any('TS' in t for t in (getattr(c, 'card_traits', []) or []))

            def on_revealed(selected, remaining):
                if selected:
                    player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay factory marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-100 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Delay action — Gain 2 memory ---
        # Note: _execute_delay trashes the card before checking this condition,
        # so permanent_of_this_card() would return None. Condition must be True.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-100 Delay: Gain 2 memory")
        effect2.set_effect_description(
            "[Main] <Delay> Gain 2 memory."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Place this card in the battle area ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT24-100 Security: Place this card in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True
        effect3._is_delay = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
