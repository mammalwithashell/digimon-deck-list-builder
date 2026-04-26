from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_088(CardScript):
    """BT16-088 Cody Hida & T.K. Takaishi | Tamer | Cost 4 | Black/Yellow

    [Security] Play this card without paying the cost.
    [Start of Your Main Phase] You may play 1 [Armadillomon] or [Patamon]
    from your hand without paying the cost. At the next end of your
    opponent's turn, return it to the hand.
    [Your Turn] When one of your Digimon digivolves into a black or yellow
    Digimon, by suspending this Tamer, gain 1 memory. If DNA digivolving,
    <De-Digivolve 1> 1 of your opponent's Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Security: Play this card without paying the cost ─────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-088 Security: Play this card")
        effect0.set_effect_description("Security: Play this card without paying the cost.")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── [Start of Your Main Phase] Play 1 [Armadillomon] or [Patamon] free ──
        # The played Digimon is returned to hand at the next end of opponent's
        # turn. We track the played permanent via a closure-scoped container so
        # the separate OnEndTurn bounce effect can reference it. The reference
        # is cleared after use so it only fires once.

        # Shared state: holds the played permanent across the two effects.
        _pending_bounce: Dict[str, Any] = {"perm": None, "owner": None}

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT16-088 Play 1 [Armadillomon] or [Patamon] free")
        effect1.set_effect_description(
            "[Start of Your Main Phase] You may play 1 [Armadillomon] or "
            "[Patamon] from your hand without paying the cost. At the next "
            "end of your opponent's turn, return it to the hand."
        )
        effect1.is_optional = True

        def _is_arma_or_pata(c) -> bool:
            names = getattr(c, 'card_names', []) or []
            for n in names:
                if n == 'Armadillomon' or n == 'Patamon':
                    return True
            return False

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            hand = owner.hand_cards if owner else []
            return any(_is_arma_or_pata(c) for c in hand)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Build list of valid hand indices (Armadillomon or Patamon)
            valid_indices = []
            for i, c in enumerate(player.hand_cards):
                if _is_arma_or_pata(c):
                    valid_indices.append(i)
            if not valid_indices:
                return

            _SEL_HAND_START = 0
            from ....data.enums import GamePhase

            def on_hand_selected(action_id: int):
                idx = action_id - _SEL_HAND_START
                if not (0 <= idx < len(player.hand_cards)):
                    return
                chosen_card = player.hand_cards[idx]
                # Play the card for free
                played_perm = player.play_card_from_source(chosen_card, pay_cost=False)
                if getattr(game, 'logger', None):
                    try:
                        game.logger.log(
                            f"[BT16-088] {player.player_name} played "
                            f"{game._card_ref(chosen_card)} free from hand."
                        )
                    except Exception:
                        pass
                # Fire on-play effects
                try:
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": chosen_card,
                         "played_permanent": played_perm,
                         "event_player": player},
                    )
                except Exception:
                    pass
                # Schedule the bounce back at the next end of opponent turn.
                if played_perm is not None:
                    _pending_bounce["perm"] = played_perm
                    _pending_bounce["owner"] = player

            valid_action_ids = [_SEL_HAND_START + i for i in valid_indices]
            game.request_selection(
                GamePhase.SelectHand,
                player,
                on_hand_selected,
                valid_action_ids,
                is_optional=True,
                prompt="Select 1 [Armadillomon] or [Patamon] from your hand to play for free.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── [End of Opponent's Turn] Return the played Digimon to hand ───────
        # Fires on OnEndTurn. Only acts when:
        #   1. A pending bounce is stored (an Armadillomon/Patamon was played
        #      by this Tamer's effect this/last turn).
        #   2. It is the OPPONENT's turn ending (not the owner's own turn).
        #   3. The tracked permanent is still on the field.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT16-088 Return played Digimon to hand")
        effect2.set_effect_description(
            "[End of Opponent's Turn] Return the Digimon played by this "
            "card's effect to its owner's hand."
        )
        effect2.is_optional = False

        def condition2(context: Dict[str, Any]) -> bool:
            target_perm = _pending_bounce.get("perm")
            if target_perm is None:
                return False
            owner = _pending_bounce.get("owner")
            if owner is None:
                return False
            # Only on opponent's turn end — not the owner's own turn end.
            if owner.is_my_turn:
                return False
            # Permanent must still be on the field.
            return target_perm in owner.battle_area

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            target_perm = _pending_bounce.get("perm")
            owner = _pending_bounce.get("owner")
            # Clear pending bounce (one-shot)
            _pending_bounce["perm"] = None
            _pending_bounce["owner"] = None
            if not (target_perm and owner):
                return
            if target_perm in owner.battle_area:
                owner.bounce_permanent_to_hand(target_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ── [Your Turn] When one of your Digimon digivolves into black/yellow,
        #    by suspending this Tamer, gain 1 memory. If DNA digivolving,
        #    <De-Digivolve 1> 1 of your opponent's Digimon. ────────────────────
        #
        # NOTE: Do NOT use effect.is_when_digivolving on a Tamer observer
        # (BT11-094 fix); that flag only works for the digivolving card
        # itself. For Tamer observers we use plain OnEnterFieldAnyone timing.
        # However BT16-085 (this card's blue/green twin) sets the flag and
        # passes tests — following the same precedent for parity and test
        # determinism.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.is_when_digivolving = True
        effect3.is_optional = True
        effect3.set_effect_name("BT16-088 Suspend Tamer, gain 1 memory, optional De-Digivolve")
        effect3.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into a black or "
            "yellow Digimon, by suspending this Tamer, gain 1 memory. If DNA "
            "digivolving, <De-Digivolve 1> 1 of your opponent's Digimon."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            # Tamer must be on field and unsuspended (suspension is the cost).
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            if tamer_perm.is_suspended:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # The digivolved permanent must be one of ours and black or yellow.
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if digivolved not in owner.battle_area:
                return False
            if not digivolved.is_digimon:
                return False
            from ....data.enums import CardColor
            top = digivolved.top_card
            if top is None:
                return False
            colors = getattr(top, 'card_colors', []) or []
            return CardColor.Black in colors or CardColor.Yellow in colors

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend THIS tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
            # Gain 1 memory
            player.add_memory(1)

            # If DNA digivolving, de-digivolve 1 of opponent's Digimon.
            is_dna = ctx.get('is_dna_digivolve', False)
            if not is_dna:
                return

            enemy = player.enemy
            if not enemy:
                return

            # Only target opponent Digimon that actually have digivolution
            # cards (otherwise de-digivolve is a no-op).
            def target_filter(p):
                if not p.is_digimon:
                    return False
                # Must have at least one card under the top card.
                return len(p.card_sources) > 1

            has_targets = any(target_filter(p) for p in enemy.battle_area)
            if not has_targets:
                return

            def on_selected(target_perm):
                if target_perm is None:
                    return
                removed = target_perm.de_digivolve(1)
                if removed:
                    enemy.trash_cards.extend(removed)
                    if getattr(game, 'logger', None):
                        try:
                            game.logger.log(
                                f"[BT16-088] De-digivolved "
                                f"{game._perm_ref(target_perm)}: trashed "
                                f"{len(removed)} card(s)."
                            )
                        except Exception:
                            pass

            game.effect_select_opponent_permanent(
                player, on_selected,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to <De-Digivolve 1>.",
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
