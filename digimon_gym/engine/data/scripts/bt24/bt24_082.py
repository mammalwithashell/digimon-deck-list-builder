from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_082(CardScript):
    """BT24-082 Owen Dreadnought"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-082 By bottom decking, Play 1 [Owen Dreadnought] from hand. Then if you have no digimon, Play 1 [Elizamon] from trash")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.")
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
            """Action: Return self to deck bottom, play Owen from hand, then Elizamon from trash.

            Chains selections properly: Owen play resolves first, THEN checks
            the no-Digimon condition for Elizamon play from trash.
            """
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this Tamer to bottom of deck
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                player.return_permanent_to_deck_bottom(tamer_perm)

            def _try_elizamon_from_trash():
                """Then, if you don't have a Digimon, play 1 [Elizamon] from trash."""
                if not any(p.is_digimon for p in player.battle_area):
                    def eliza_filter(c):
                        c_names = getattr(c, 'card_names', []) or []
                        return any('Elizamon' in n for n in c_names)
                    game.effect_play_from_zone(
                        player, 'trash', eliza_filter, free=True, is_optional=True)

            # Play 1 [Owen Dreadnought] from hand without paying cost
            # Chain Elizamon check into the Owen selection's callback
            def owen_filter(c):
                c_names = getattr(c, 'card_names', []) or []
                return any('Owen Dreadnought' in n for n in c_names)

            from ....data.enums import GamePhase
            from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE
            valid = []
            for i, c in enumerate(player.hand_cards):
                if (owen_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE
                        and not game._is_play_blocked_by_modifier(c)
                        and not game._is_effect_play_blocked(c)):
                    valid.append(SEL_HAND_START + i)

            if not valid:
                # No Owen in hand — skip directly to Elizamon check
                _try_elizamon_from_trash()
                return

            def on_owen_selected(action_id: int):
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    owen_card = player.hand_cards[idx]
                    played_perm = player.play_card_from_source(owen_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{game._card_ref(owen_card)} from hand")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": owen_card, "played_permanent": played_perm, "event_player": player},
                    )
                    game._fire_play_observers(played_perm, player)
                # After Owen play resolves, check Elizamon
                _try_elizamon_from_trash()

            def on_owen_declined():
                # Player declined Owen play — still check Elizamon
                _try_elizamon_from_trash()

            game.request_selection(
                GamePhase.SelectTarget, player, on_owen_selected, valid,
                is_optional=True,
                prompt="Select an [Owen Dreadnought] from hand to play without paying the cost.",
                on_decline=on_owen_declined)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon,
        # by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.
        # Uses _is_digivolve_observer pattern so the engine fires this via _fire_digivolve_observers.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-082 +3k and may attack")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.")
        effect1.is_optional = True
        effect1._is_digivolve_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Cost: tamer must not already be suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and getattr(tamer_perm, 'is_suspended', False):
                return False
            # Must be our turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check the digivolved permanent has Reptile or Dragonkin trait
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            traits = getattr(digivolved.top_card, 'card_traits', []) or [] if digivolved.top_card else []
            if not any('Reptile' in t or 'Dragonkin' in t for t in traits):
                return False
            # Must be our own Digimon
            player = card.owner if card else None
            if player and digivolved not in player.battle_area:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, DP +3000 to digivolved Digimon, then it may attack."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                tamer_perm.suspend()
            # Target: the digivolved Digimon gets +3000 DP
            digivolved = ctx.get('digivolved_permanent')
            if digivolved:
                digivolved.change_dp(3000)
                # "Then, it may attack" — optional attack via MAY_ATTACK
                from ....interfaces.modifiers import ModifierType
                if digivolved.is_suspended:
                    digivolved.unsuspend()
                digivolved.grant_keyword('_is_rush')
                game.register_modifier(
                    digivolved, ModifierType.MAY_ATTACK,
                    expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT24-082 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this tamer from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                game.effect_play_from_security(player, card)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
