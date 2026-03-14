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
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements ---
        # Condition: you have a [TS] Digimon or Tamer on the field
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-100 Ignore color requirements")
        effect0.set_effect_description(
            "While you have an [TS] trait Digimon or Tamer on the field, "
            "you can ignore this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            return any(
                (p.is_digimon or p.is_tamer)
                and any('TS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in owner.battle_area
                if p.top_card
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # Color requirement bypass — not modeled in engine

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Main] Reveal top 3, add 1 [TS] card to hand, bottom rest.
        #    Then place this card in battle area (Delay). ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT24-100 Reveal top 3, add 1 [TS] card to hand, bottom rest")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 [TS] trait card among them "
            "to the hand. Return the rest to the bottom of the deck. Then, place this card "
            "in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return any('TS' in t for t in (getattr(c, 'card_traits', []) or []))

            def on_revealed(selected, remaining):
                if selected:
                    player.hand_cards.append(selected)
                # Return rest to bottom of deck
                for c in remaining:
                    player.library_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True
            )

            # Place this option card in the battle area (Delay placement)
            # The engine's PlaceDelayOptionCards equivalent — place card in battle area
            if card and player:
                played = player.play_card_from_source(card, pay_cost=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay — Gain 2 memory ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT24-100 Delay: Gain 2 memory")
        effect2.set_effect_description(
            "[Main] <Delay> Gain 2 memory."
        )
        effect2._is_delay_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
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
