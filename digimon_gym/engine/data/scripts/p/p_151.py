from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_151(CardScript):
    """P-151 Digimon Liberator | Option (White, Cost 2)

    While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this
    card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 card with the
    [LIBERATOR] trait among them to your hand. Return the rest to the bottom
    of the deck in any order. Then, you may play 1 card with the [LIBERATOR]
    trait and a play cost of 3 or less from your hand without paying the cost.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # --- Color requirement bypass: while you have LIBERATOR trait Digimon or Tamer ---
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce
            for p in owner.battle_area:
                if (p.is_digimon or p.is_tamer) and p.top_card:
                    traits = getattr(p.top_card, 'card_traits', []) or []
                    if any('LIBERATOR' in t for t in traits):
                        return False  # bypass color requirement
            return True  # enforce color requirement
        card._match_color_requirement_fn = _check_color_req

        effects = []

        def _has_liberator_trait(c):
            traits = getattr(c, 'card_traits', []) or []
            return any('LIBERATOR' in t for t in traits)

        def _main_effect_process(ctx: Dict[str, Any]):
            """Reveal top 3, add 1 [LIBERATOR] to hand, rest to deck bottom,
            then play 1 [LIBERATOR] cost<=3 free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def reveal_filter(c):
                return _has_liberator_trait(c)

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)

                # Then play 1 [LIBERATOR] trait card with cost <= 3 from hand free
                def play_filter(c):
                    if not _has_liberator_trait(c):
                        return False
                    cost = getattr(c, 'get_cost_itself', None)
                    if cost is None:
                        cost = getattr(c, 'play_cost', None)
                    if cost is None or cost > 3:
                        return False
                    return True

                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)

            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        # --- [Main] Reveal top 3, add 1 [LIBERATOR], play 1 cost<=3 free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-151 Reveal top 3, Add 1 with [LIBERATOR] trait. Then Play 1 with [LIBERATOR] trait")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 card with the "
            "[LIBERATOR] trait among them to your hand. Return the rest to "
            "the bottom of the deck. Then, you may play 1 card with the "
            "[LIBERATOR] trait and a play cost of 3 or less from your hand "
            "without paying the cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_main_effect_process)
        effects.append(effect1)

        # --- [Security] Activate this card's [Main] effects ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("P-151 Security: Activate Main effects")
        effect2.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Security: activate the Main effect (reveal + play)."""
            _main_effect_process(ctx)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
