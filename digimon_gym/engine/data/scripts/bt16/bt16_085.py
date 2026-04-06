from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_085(CardScript):
    """BT16-085 Davis Motomiya & Ken Ichijoji | Tamer | Cost 4

    [Security] Play this card without paying the cost.
    [Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from hand
    without paying the cost. At the next end of your opponent's turn, return
    that Digimon to the hand.
    [Your Turn] When one of your Digimon digivolves into a blue or green
    Digimon, by suspending this Tamer, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Security: Play this card without paying the cost ─────────────────
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-085 Security: Play this card")
        effect0.set_effect_description("Security: Play this card without paying the cost.")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── [Start of Your Main Phase] Play 1 [Veemon] or [Wormmon] free ──────
        # The played Digimon is returned at the next end of opponent's turn.
        # We track the played permanent via a mutable container so the bounce
        # effect can reference it.
        # A separate OnEndTurn effect handles the bounce back.
        #
        # NOTE: Because the bounce effect must be registered dynamically after
        # the card is played, we define an OnEndTurn effect that checks a shared
        # reference. The reference is cleared after use so it only fires once.

        # Shared state: holds the played permanent's identity across effects.
        _pending_bounce: Dict[str, Any] = {"perm": None}

        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT16-085 Play 1 [Veemon] or [Wormmon] free")
        effect1.set_effect_description(
            "[Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from "
            "your hand without paying the cost. At the next end of your opponent's "
            "turn, return that Digimon to the hand."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Must have a qualifying card in hand
            hand = owner.hand_cards if owner else []
            return any(
                any('Veemon' in n or 'Wormmon' in n
                    for n in getattr(c, 'card_names', []))
                for c in hand
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Veemon' in n or 'Wormmon' in n for n in names)

            # We need to know which permanent was played so we can bounce it.
            # Wrap the play with our own selection to capture it.
            valid_indices = []
            for i, c in enumerate(player.hand_cards):
                if play_filter(c):
                    valid_indices.append(i)  # hand index

            if not valid_indices:
                return

            # Import the SEL_HAND_START offset (mirrors game.py constant)
            _SEL_HAND_START = 0
            from ....data.enums import GamePhase

            def on_hand_selected(action_id: int):
                idx = action_id - _SEL_HAND_START
                if not (0 <= idx < len(player.hand_cards)):
                    return
                chosen_card = player.hand_cards[idx]
                # Play card for free
                played_perm = player.play_card_from_source(chosen_card, pay_cost=False)
                game.logger.log(
                    f"[BT16-085] {player.player_name} played "
                    f"{game._card_ref(chosen_card)} free from hand."
                )
                # Fire OnPlay effects
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": chosen_card,
                     "played_permanent": played_perm,
                     "event_player": player},
                )
                # Schedule the bounce: remember which permanent to bounce back
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
                prompt="Select 1 [Veemon] or [Wormmon] from your hand to play for free.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── [End of Opponent's Turn] Bounce the played Digimon back to hand ───
        # This effect fires every OnEndTurn. It only acts when:
        #   1. A pending bounce is stored (a Veemon/Wormmon was played this way)
        #   2. It is the OPPONENT's turn ending (not our turn)
        #   3. The tracked permanent is still on the field
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT16-085 Return played Digimon to hand")
        effect2.set_effect_description(
            "[End of Opponent's Turn] Return the Digimon played by this card's effect to hand."
        )
        effect2.is_optional = False

        def condition2(context: Dict[str, Any]) -> bool:
            target_perm = _pending_bounce.get("perm")
            if target_perm is None:
                return False
            owner = _pending_bounce.get("owner")
            if owner is None:
                return False
            # Only fire when it's opponent's turn ending (not the owner's own turn)
            if owner.is_my_turn:
                return False
            # Permanent must still be on the field
            return target_perm in owner.battle_area

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            target_perm = _pending_bounce.get("perm")
            owner = _pending_bounce.get("owner")
            if not (target_perm and owner):
                return
            # Clear the pending bounce (one-shot)
            _pending_bounce["perm"] = None
            _pending_bounce["owner"] = None
            # Bounce the Digimon back to its owner's hand
            if target_perm in owner.battle_area:
                owner.bounce_permanent_to_hand(target_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ── [Your Turn] When one of your Digimon digivolves into blue/green,
        #    by suspending this Tamer, gain 1 memory. ──────────────────────────
        #
        # Timing: OnEnterFieldAnyone with is_when_digivolving = True
        # Condition: owner's turn, this tamer is unsuspended,
        #            the digivolved permanent is blue or green, is owned by player.
        # Process: suspend this tamer, gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.is_when_digivolving = True
        effect3.is_optional = True
        effect3.set_effect_name("BT16-085 Suspend this Tamer, gain 1 memory")
        effect3.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into a blue or green "
            "Digimon, by suspending this Tamer, gain 1 memory."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            # Tamer must be on the field and unsuspended (suspension is the cost)
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            if tamer_perm.is_suspended:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # The digivolved permanent must be one of ours and blue or green
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
            return CardColor.Blue in colors or CardColor.Green in colors

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

            # If DNA digivolving, trash any 3 digivolution cards under
            # your opponent's Digimon.
            is_dna = ctx.get('is_dna_digivolve', False)
            if is_dna:
                enemy = player.enemy
                trashed_total = 0
                max_trash = 3
                # Collect opponent Digimon with digivolution cards
                for opp_perm in list(enemy.battle_area):
                    if trashed_total >= max_trash:
                        break
                    if not opp_perm.is_digimon:
                        continue
                    digi_cards = [c for c in opp_perm.card_sources
                                  if c is not opp_perm.top_card]
                    if not digi_cards:
                        continue
                    can_trash = min(len(digi_cards), max_trash - trashed_total)
                    trashed = opp_perm.trash_digivolution_cards(can_trash)
                    if trashed:
                        enemy.trash_cards.extend(trashed)
                        trashed_total += len(trashed)
                        game.logger.log(
                            f"[BT16-085] Trashed {len(trashed)} digivolution "
                            f"card(s) from {game._perm_ref(opp_perm)}."
                        )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
