from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_088(CardScript):
    """BT22-088 Arisa Kinosaki"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Shoemon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT22-088 By bottom decking, Play 1 [Arisa Kinosaki] from hand. then if you have no digimon, Play 1 [Shoemon] from trash")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Shoemon] from your trash without paying the cost.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Return this Tamer to deck bottom, play Arisa from hand, then Shoemon from trash if no Digimon."""
            from ....data.enums import GamePhase as _GP
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this Tamer to the bottom of the deck (use engine API)
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)

            def _try_play_shoemon():
                """Then, if you don't have a Digimon, play 1 [Shoemon] from trash."""
                has_digimon = any(p.is_digimon for p in player.battle_area)
                if not has_digimon:
                    def shoemon_filter(c):
                        names = getattr(c, 'card_names', []) or []
                        return any(n == 'Shoemon' for n in names)
                    game.effect_play_from_zone(
                        player, 'trash', shoemon_filter, free=True, is_optional=True)

            # Play 1 [Arisa Kinosaki] from hand without paying the cost (exact name match)
            def arisa_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any(n == 'Arisa Kinosaki' for n in names)

            # Build custom selection to chain Shoemon play after Arisa resolves
            from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE
            valid = []
            for i, c in enumerate(player.hand_cards):
                if arisa_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(c) and not game._is_effect_play_blocked(c):
                        valid.append(SEL_HAND_START + i)
            if not valid:
                # No Arisa in hand — still try Shoemon
                _try_play_shoemon()
                return

            def on_arisa_select(action_id: int):
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    sel_card = player.hand_cards[idx]
                    played_perm = player.play_card_from_source(sel_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(sel_card)} from hand")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": sel_card, "played_permanent": played_perm,
                         "event_player": player},
                    )
                    game._fire_play_observers(played_perm, player)
                # Chain: try Shoemon from trash
                _try_play_shoemon()

            def on_arisa_decline():
                # Declined Arisa play — still try Shoemon
                _try_play_shoemon()

            game.request_selection(
                _GP.SelectTarget, player, on_arisa_select, valid,
                is_optional=True,
                prompt="Select [Arisa Kinosaki] from hand to play without paying the cost.",
                on_decline=on_arisa_decline)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Tokens or [Puppet] trait Digimon are played, by suspending this Tamer, <Draw 1> (Draw 1 card from your deck.)
        # NOTE: This is an OBSERVER effect — fires when OTHER permanents are played.
        # Do NOT set is_on_play (that gates it to fire only for self-play).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-088 Draw 1")
        effect1.set_effect_description("[All Turns] When any of your Tokens or [Puppet] trait Digimon are played, by suspending this Tamer, <Draw 1> (Draw 1 card from your deck.)")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must not already be suspended (suspend is cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The played card must be a Token or [Puppet] trait Digimon owned by us
            played_card = context.get('played_card')
            if not played_card:
                return False
            event_player = context.get('event_player')
            owner = card.owner if card else None
            if event_player and owner and event_player is not owner:
                return False
            is_token = getattr(played_card, 'is_token', False)
            if is_token:
                return True
            if getattr(played_card, 'is_digimon', False):
                traits = getattr(played_card, 'card_traits', []) or []
                if any('Puppet' in t for t in traits):
                    return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, then Draw 1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()
            # Effect: Draw 1
            player.draw_cards(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT22-088 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            game = ctx.get('game')
            player = ctx.get('player')
            if not (game and player):
                return
            game.effect_play_from_security(player, card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
