from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_070(CardScript):
    """EX11-070 Unchained | Tamer

    [Security] Play this card without paying the cost.
    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [End of Your Turn] 2 of your Digimon may DNA digivolve into
    [ExMaquinamon] in the hand. Then, this Tamer may <Mind Link> with 1 of
    your Digimon with [Maquinamon] in its text.

    --- Inherited ---
    [All Turns] This Digimon with [Maquinamon] in its text can't have less
    than 1000 DP, and your opponent's effects can't trash its stacked cards.
    [End of All Turns] You may play 1 [Unchained] from this Digimon's
    digivolution cards without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- [Security] Play this card without paying the cost ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-070 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX11-070 Set memory to 3")
        effect1.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- [End of Your Turn] DNA digivolve into [ExMaquinamon], then Mind Link ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("EX11-070 DNA digivolve into [ExMaquinamon]. Mind Link to [Maquinamon] in text.")
        effect2.set_effect_description(
            "[End of Your Turn] 2 of your Digimon may DNA digivolve into "
            "[ExMaquinamon] in the hand. Then, this Tamer may <Mind Link> "
            "with 1 of your Digimon with [Maquinamon] in its text.")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """DNA digivolve into ExMaquinamon (paying cost), then Mind Link."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: DNA digivolve into [ExMaquinamon] from hand (paying cost per C#)
            def dna_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('ExMaquinamon' in n for n in names)
            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)
            # Step 2: Mind Link with a Digimon that has [Maquinamon] in text
            def link_filter(p):
                if not p.top_card:
                    return False
                text = getattr(p.top_card, 'card_text', '') or ''
                return 'Maquinamon' in text
            game.effect_link_to_permanent(player, card, filter_fn=link_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Inherited: DP floor (can't have less than 1000 DP) ---
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-070 Can't have less than 1000 DP")
        effect3.set_effect_description(
            "[All Turns] This Digimon with [Maquinamon] in its text can't "
            "have less than 1000 DP.")
        effect3.is_inherited_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card()
            if permanent and permanent.top_card:
                text = getattr(permanent.top_card, 'card_text', '') or ''
                if 'Maquinamon' not in text:
                    return False
            else:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Register DP floor modifier so DP can't go below 1000."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from ....interfaces.modifiers import ModifierType

                def dp_floor_fn(current_dp, target, context):
                    return max(1000, current_dp)

                game.register_modifier(
                    perm, ModifierType.CHANGE_DP,
                    value_fn=dp_floor_fn, expiry='permanent')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Inherited: opponent's effects can't trash this Digimon's stacked cards ---
        effect4 = ICardEffect()
        effect4.set_effect_name("EX11-070 Immune to opponent stack trashing")
        effect4.set_effect_description(
            "[All Turns] Your opponent's effects can't trash this Digimon's stacked cards.")
        effect4.is_inherited_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card()
            if permanent and permanent.top_card:
                text = getattr(permanent.top_card, 'card_text', '') or ''
                if 'Maquinamon' not in text:
                    return False
            else:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Register IMMUNE_FROM_STACK_TRASHING modifier."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.IMMUNE_FROM_STACK_TRASHING,
                    condition=lambda p, c, src=perm: p is src,
                    expiry='permanent')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # --- Inherited: [End of All Turns] Play 1 [Unchained] from digi cards ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndTurn)
        effect5.set_effect_name("EX11-070 Play 1 [Unchained] from digivolution cards")
        effect5.set_effect_description(
            "[End of All Turns] You may play 1 [Unchained] from this "
            "Digimon's digivolution cards without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Play 1 [Unchained] from this Digimon's digivolution cards.
            Uses selection for player choice when multiple candidates exist."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            from ....game.constants import SEL_EFFECT_CHOICE_START
            from ....data.enums import GamePhase

            def is_unchained(c):
                names = getattr(c, 'card_names', []) or []
                return any('Unchained' in n for n in names)

            # Build valid indices for Unchained cards in digivolution stack
            valid = []
            for i, cs in enumerate(perm.card_sources):
                if cs is perm.top_card:
                    continue
                if is_unchained(cs):
                    valid.append(SEL_EFFECT_CHOICE_START + i)

            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_EFFECT_CHOICE_START
                if idx < 0 or idx >= len(perm.card_sources):
                    return
                selected = perm.card_sources[idx]
                perm.card_sources.remove(selected)
                played_perm = player.play_card_from_source(selected, pay_cost=False)
                game.logger.log(
                    f"[Effect] {player.player_name} played "
                    f"{game._card_ref(selected)} from digivolution cards")
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": selected, "played_permanent": played_perm, "event_player": player},
                )

            game.request_selection(
                GamePhase.SelectEffectChoice, player, on_select, valid,
                is_optional=True,
                prompt="Select an [Unchained] from digivolution cards to play.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
