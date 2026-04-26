from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_034(CardScript):
    """BT15-034 Salamon | Lv.3

    Main:
    [Start of Your Main Phase] If you have 3 or more security cards, you may
    add the top card of your security stack to the hand. If you have 2 or
    fewer, you may place 1 yellow Digimon card with the [Vaccine] trait from
    your hand at the top or bottom of your security stack.

    Inherited:
    [All Turns] [Once Per Turn] When a card is removed from your security
    stack, 1 of your opponent's Digimon gets -2000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ──────────────────────────────────────────────────────────────
        # Inherited: OnLoseSecurity → -2000 DP to 1 opponent Digimon
        # DCGO ref: CanTriggerWhenLoseSecurity(player => player == card.Owner)
        # ──────────────────────────────────────────────────────────────
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name(
            "BT15-034 Give an opponent's Digimon card -2000 DP.")
        effect0.set_effect_description(
            "[All Turns] [Once Per Turn] When a card is removed from your "
            "security stack, 1 of your opponent's Digimon gets -2000 DP for "
            "the turn.")
        effect0.is_inherited_effect = True
        effect0.is_optional = False  # DCGO canNoSelect=false on ActivateClass
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("-2000DP_BT15_034")

        def condition0(context: Dict[str, Any]) -> bool:
            # Field presence required (DCGO IsExistOnBattleArea)
            if card is None or card.permanent_of_this_card() is None:
                return False
            # DCGO: CanTriggerWhenLoseSecurity(player => player == card.Owner)
            # Only fire when the losing player IS this card's owner.
            owner = card.owner if card else None
            if owner is None:
                return False
            event_player = context.get('event_player')
            if event_player is None:
                # Direct-call fallback: treat ctx['player'] as the losing
                # player. When the engine fires OnLoseSecurity via
                # _fire_timing with {"player": self}, _collect_triggered_effects
                # lifts that into extra context as 'event_player'.
                event_player = context.get('player')
            if event_player is not owner:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select 1 opponent Digimon and give it -2000 DP for the turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if enemy is None:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                if target_perm is None:
                    return
                from ....interfaces.modifiers import ModifierType
                # Target-equality guard: without an explicit condition, the
                # CHANGE_DP modifier is considered active for every target,
                # which would leak -2000 DP across the whole board.
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_DP,
                    condition=(lambda t, c, _tp=target_perm: t is _tp),
                    value_fn=lambda cur, t, c: cur - 2000,
                    expiry='end_of_turn',
                )

            game.effect_select_opponent_permanent(
                player,
                on_select,
                filter_fn=target_filter,
                is_optional=False,
                prompt="Select 1 opponent Digimon to receive -2000 DP.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ──────────────────────────────────────────────────────────────
        # Main: [Start of Your Main Phase] branching effect
        # DCGO ref: OnStartMainPhase with two IsExistOnBattleArea branches
        #   - security count >= 3 → add top security to hand (+ reduce)
        #   - security count <= 2 → pick yellow Vaccine Digimon from hand,
        #     player chooses top/bottom placement
        # Both branches are optional. maxUsePerTurn = -1 (not OPT).
        # ──────────────────────────────────────────────────────────────
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name(
            "BT15-034 Add top security to hand, or place yellow Vaccine "
            "Digimon from hand at top/bottom of security")
        effect1.set_effect_description(
            "[Start of Your Main Phase] If you have 3 or more security "
            "cards, you may add the top card of your security stack to the "
            "hand. If you have 2 or fewer, you may place 1 yellow Digimon "
            "card with the [Vaccine] trait from your hand at the top or "
            "bottom of your security stack.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if owner is None or not owner.is_my_turn:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def _is_yellow_vaccine_digimon(c) -> bool:
            """Filter: yellow Digimon card with the [Vaccine] trait."""
            if not getattr(c, 'is_digimon', False):
                return False
            # Yellow color check
            colors = getattr(c, 'card_colors', None) or []
            has_yellow = any(
                (col is CardColor.Yellow) or getattr(col, 'name', None) == 'Yellow'
                for col in colors
            )
            if not has_yellow:
                return False
            # [Vaccine] in C# is an attribute — CardTraits in DCGO returns
            # Form + Attribute + Type, so C# ContainsTraits("Vaccine") matches.
            entity = getattr(c, 'c_entity_base', None)
            if entity is None:
                return False
            attrs = getattr(entity, 'attribute_eng', None) or []
            types_ = getattr(entity, 'type_eng', None) or []
            forms = getattr(entity, 'form_eng', None) or []
            all_traits = list(attrs) + list(types_) + list(forms)
            return any('Vaccine' == t for t in all_traits)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            sec_count = len(player.security_cards)

            if sec_count >= 3:
                # Branch A: add top security card to hand (player.security_cards[0])
                # DCGO uses CardObjectController.AddHandCards + IReduceSecurity.
                # Functionally: remove from security, place in hand.
                # The C# sequence does NOT ask for confirmation here — the
                # parent ActivateClass already handled the optional prompt.
                if not player.security_cards:
                    return
                top_card = player.security_cards[0]
                # Remove from security & face_up tracking
                player.security_cards.pop(0)
                player.face_up_security.discard(top_card)
                player.hand_cards.append(top_card)
                if hasattr(game, 'logger') and game.logger is not None:
                    try:
                        name = (top_card.card_names[0]
                                if top_card.card_names else 'card')
                        game.logger.log(
                            f"[BT15-034] {player.player_name} added top "
                            f"security ({name}) to hand.")
                    except Exception:
                        pass
                return

            # Branch B: sec_count <= 2
            # Player selects yellow Vaccine Digimon from hand, then chooses
            # top or bottom of security stack.
            # effect_select_hand_card is a no-op if no valid card is present,
            # so there is no silent failure if hand is empty.
            def on_card_selected(selected_card):
                if selected_card is None:
                    return
                # Second choice: top (branch 0) or bottom (branch 1)
                def on_placement(branch):
                    if selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                    else:
                        return
                    if branch == 0:
                        # Top of security (index 0 — where security_attack pops from)
                        player.security_cards.insert(0, selected_card)
                        place_word = "top"
                    else:
                        # Bottom of security
                        player.security_cards.append(selected_card)
                        place_word = "bottom"
                    # Fire OnAddSecurity so other effects can react
                    try:
                        player._fire_timing(
                            EffectTiming.OnAddSecurity,
                            {"added_cards": [selected_card],
                             "player": player},
                        )
                    except Exception:
                        pass
                    if hasattr(game, 'logger') and game.logger is not None:
                        try:
                            name = (selected_card.card_names[0]
                                    if selected_card.card_names else 'card')
                            game.logger.log(
                                f"[BT15-034] {player.player_name} placed "
                                f"{name} at {place_word} of security.")
                        except Exception:
                            pass

                game.effect_choose_branch(
                    player,
                    2,
                    on_placement,
                    prompt="Place the card at the top or bottom of security?",
                    branch_labels=[
                        "Top of security",
                        "Bottom of security",
                    ],
                )

            game.effect_select_hand_card(
                player,
                filter_fn=_is_yellow_vaccine_digimon,
                callback=on_card_selected,
                is_optional=False,
                prompt=("Select 1 yellow Digimon card with the [Vaccine] "
                        "trait to place in security."),
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
