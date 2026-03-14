from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_067(CardScript):
    """EX11-067 Dokuson Aruba | Tamer

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] 1 of your Digimon with [Lucemon] in its text on the field may
        digivolve into a Digimon card with [Lucemon] in its name in the hand
        or trash without paying the cost.
    [Your Turn] When any of your Digimon digivolve into a Digimon card with
        [Lucemon] in its name, by suspending this Tamer, gain 1 memory.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-067 Set memory to 3")
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
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Play] 1 of your Digimon with [Lucemon] in text may digivolve
        # into Lucemon from hand or trash
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-067 On Play: Lucemon Digimon may digivolve")
        effect1.set_effect_description(
            "[On Play] 1 of your Digimon with [Lucemon] in its text on the field "
            "may digivolve into a Digimon card with [Lucemon] in its name in the "
            "hand or trash without paying the cost."
        )
        effect1.is_on_play = True
        effect1.is_optional = True

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

            # Select 1 of your Digimon with [Lucemon] in text
            def perm_filter(p):
                if not p.is_digimon:
                    return False
                # Check if any card in the stack has Lucemon in text
                for cs in p.card_sources:
                    text = getattr(cs, 'card_text', '') or ''
                    names = getattr(cs, 'card_names', []) or []
                    if 'Lucemon' in text or any('Lucemon' in n for n in names):
                        return True
                return False

            def on_select_perm(target_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', []) or []
                    return any('Lucemon' in n for n in names)

                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_override=0, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select_perm, filter_fn=perm_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Your Turn] When digivolve into Lucemon, suspend this Tamer, gain 1 memory
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-067 Digivolve into Lucemon: suspend, +1 memory")
        effect2.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve into a Digimon card "
            "with [Lucemon] in its name, by suspending this Tamer, gain 1 memory."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            perm = card.permanent_of_this_card()
            if not perm or perm.is_suspended:
                return False
            # Check if a digivolve into Lucemon happened
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if not digivolved.contains_card_name('Lucemon'):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if perm and not perm.is_suspended:
                perm.suspend()
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Play this card without paying the cost
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX11-067 Security: play this card")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")
        effect3.is_security_effect = True
        effect3._security_play_self = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.play_card_from_source(card, pay_cost=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
