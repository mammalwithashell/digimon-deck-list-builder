from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_018(CardScript):
    """EX6-018 Lucemon | Lv.3

    When you would play this card, if you don't have a level 5 or lower
        Digimon, reduce the play cost by 5.
    [On Play] [Start of Your Main Phase] Reveal the top 3 cards of your
        deck. Add 1 card with the [Angel], [Archangel], [Three Great Angels]
        or [Seven Great Demon Lords] trait among them to your hand. Trash
        the rest.
    [End of Your Turn] [Once Per Turn] By placing one of your level 6 Digimon
        on top of your security stack, this Digimon may digivolve into
        [Lucemon: Chaos Mode] in the trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Cost reduction: if no Lv5 or lower Digimon on field, reduce by 5
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("EX6-018 Cost reduction without Lv5- Digimon")
        effect0.set_effect_description(
            "When you would play this card, if you don't have a level 5 or "
            "lower Digimon, reduce the play cost by 5."
        )
        effect0.cost_reduction = 5

        def condition0(context: Dict[str, Any]) -> bool:
            player = card.owner if card else None
            if not player:
                return False
            card_source = context.get('card_source')
            if card_source is not card:
                return False
            # Check no Lv5 or lower Digimon on field
            has_low_digimon = any(
                p.is_digimon and p.level is not None and p.level <= 5
                for p in player.battle_area
            )
            return not has_low_digimon
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _trait_filter(c):
            traits = getattr(c, 'card_traits', []) or []
            return any(
                'Angel' in t or 'Archangel' in t
                or 'Three Great Angels' in t
                or 'Seven Great Demon Lords' in t
                for t in traits
            )

        # [On Play] Reveal top 3, add 1 with matching trait, trash rest
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-018 On Play: reveal top 3, add trait card")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with "
            "the [Angel], [Archangel], [Three Great Angels] or [Seven Great "
            "Demon Lords] trait among them to your hand. Trash the rest."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_reveal_and_select_multi(
                player, 3, [(_trait_filter, 'hand')],
                remaining_placement='trash', is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Start of Your Main Phase] Same reveal effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("EX6-018 Start Main: reveal top 3, add trait card")
        effect2.set_effect_description(
            "[Start of Your Main Phase] Reveal the top 3 cards of your deck. "
            "Add 1 card with the [Angel], [Archangel], [Three Great Angels] or "
            "[Seven Great Demon Lords] trait among them to your hand. Trash the rest."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_reveal_and_select_multi(
                player, 3, [(_trait_filter, 'hand')],
                remaining_placement='trash', is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [End of Your Turn] Place Lv6 to security, digivolve into Chaos Mode from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.EndOfTurn)
        effect3.set_effect_name("EX6-018 End Turn: place Lv6, digivolve Chaos Mode")
        effect3.set_effect_description(
            "[End of Your Turn] [Once Per Turn] By placing one of your level 6 "
            "Digimon on top of your security stack, this Digimon may digivolve "
            "into [Lucemon: Chaos Mode] in the trash without paying the cost."
        )
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EOT_EX6-018")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: place 1 of your Lv6 Digimon on top of security
            def target_filter(p):
                return p.is_digimon and p is not perm and p.level is not None and p.level == 6

            def on_place_security(target_perm):
                if target_perm in player.battle_area:
                    player.put_permanent_to_security(target_perm)

                    # Digivolve into Lucemon: Chaos Mode from trash
                    cur_perm = card.permanent_of_this_card() if card else None
                    if not cur_perm:
                        return

                    def chaos_filter(c):
                        names = getattr(c, 'card_names', []) or []
                        return any(
                            'Lucemon: Chaos Mode' in n or 'Lucemon Chaos Mode' in n
                            for n in names
                        )

                    chaos_cards = [c for c in player.trash_cards if chaos_filter(c)]
                    if chaos_cards:
                        chosen = chaos_cards[0]
                        player.trash_cards.remove(chosen)
                        cur_perm.card_sources.append(chosen)

            has_targets = any(target_filter(p) for p in player.battle_area)
            if has_targets:
                game.effect_select_own_permanent(
                    player, on_place_security, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
