from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_096(CardScript):
    """BT23-096 Comet Hammer | Option, Black, CS, Cost 5

    While you have a Digimon or Tamer with the [CS] trait on the field, you
    can ignore this card's color requirements.
    [Main] <De-Digivolve 4> 1 of your opponent's Digimon. (Trash up to 4 cards
        from the top. You can't trash past level 3 cards.) Then, place this
        card in the battle area.
    [Your Turn] When one of your [CS] trait Digimon attacks, <Delay>
        (By trashing this card after the placing turn, activate the effect
        below.)
      - <De-Digivolve 4> 1 of your opponent's Digimon. (Trash up to 4 cards
        from the top. You can't trash past level 3 cards.)
    [Security] <De-Digivolve 4> 1 of your opponent's Digimon. Then, place
        this card in the battle area.

    C# ref: DCGO/Assets/Scripts/CardEffect/BT23/Black/BT23_096.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # ─── Dynamic color bypass ─────────────────────────────────────
        # "While you have a Digimon or Tamer with the [CS] trait on the
        # field, you can ignore this card's color requirements."
        # C# uses CardEffectCommons.HasMatchConditionPermanent on permanents
        # with HasCSTraits + (IsDigimon || IsTamer). Implemented via the
        # _match_color_requirement_fn callable which the engine consults
        # during action_mask play-cost validation.
        def _has_cs_on_field() -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            for p in owner.battle_area:
                if not (p.is_digimon or p.is_tamer):
                    continue
                top = p.top_card
                if top is None:
                    continue
                traits = getattr(top, 'card_traits', []) or []
                # Substring match handles combined traits like "Hudie/CS".
                if any('CS' in t for t in traits):
                    return True
            return False

        def _check_cs_color_req() -> bool:
            # False = bypass color requirement; True = enforce as normal.
            return not _has_cs_on_field()

        card._match_color_requirement_fn = _check_cs_color_req

        effects: List[ICardEffect] = []

        # ─── Shared de-digivolve helper ────────────────────────────────
        def _run_de_digivolve(player, game, after_callback=None):
            """Select 1 opponent Digimon and de-digivolve it by 4.

            after_callback: optional zero-arg function run after the
                de-digivolve resolves (or if no valid targets exist).
                Used by the Security effect to chain "place in battle area"
                after the de-digivolve selection completes.
            """
            enemy = player.enemy if player else None
            if not (player and game and enemy):
                if after_callback:
                    after_callback()
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(4)
                if removed:
                    enemy.trash_cards.extend(removed)
                if after_callback:
                    after_callback()

            # Check if there are any valid opponent Digimon to de-digivolve.
            has_target = any(
                p.is_digimon
                and game.modifiers.can_be_selected_by_effect(p)
                for p in enemy.battle_area
            )
            if not has_target:
                # No valid target — just run the chained callback.
                if after_callback:
                    after_callback()
                return

            game.effect_select_opponent_permanent(
                player,
                on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to de-digivolve by 4.",
            )

        # ─── Effect 0: [Main] <De-Digivolve 4> ────────────────────────
        # Then, place this card in the battle area.
        # The engine auto-keeps option permanents on the field when a
        # _is_delay factory effect is present, so we do not need to
        # manually re-add the permanent here.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "BT23-096 [Main] <De-Digivolve 4> 1 opponent Digimon")
        effect0.set_effect_description(
            "[Main] <De-Digivolve 4> 1 of your opponent's Digimon. "
            "(Trash up to 4 cards from the top. You can't trash past level 3 "
            "cards.) Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            _run_de_digivolve(player, game)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: <Delay> factory marker ─────────────────────────
        # Keeps the option on field after resolution. The engine's
        # _option_stays_on_field check scans for _is_delay effects to
        # decide whether to trash the option permanent after the main
        # effect resolves.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-096 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Only allow the player-activated Delay-trash action when the
            # owner is in a state where the OnAllyAttack effect could fire.
            # This is a UX guard: the action_mask does not check this
            # condition (it only checks _has_delay_effect), but we check
            # it anyway so the effect is well-gated if the engine starts
            # consulting it later.
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ─── Effect 2: [Your Turn] OnAllyAttack → Delay de-digivolve ──
        # When one of your [CS] trait Digimon attacks, optionally trash
        # this option card and de-digivolve 4 an opponent's Digimon.
        # C# uses OnAllyAttack timing with isOptional=true. The delete-
        # permanent cost (DeletePeremanentAndProcessAccordingToResult)
        # fires FIRST; on success, the de-digivolve runs.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnAllyAttack)
        effect2.set_effect_name(
            "BT23-096 [Your Turn] Delay on CS attack: <De-Digivolve 4>")
        effect2.set_effect_description(
            "[Your Turn] When one of your [CS] trait Digimon attacks, "
            "<Delay> <De-Digivolve 4> 1 of your opponent's Digimon.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] only
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Placing-turn restriction: Delay cannot activate on the same
            # turn it was placed.
            perm = card.permanent_of_this_card()
            game = context.get('game')
            if perm and game and perm.turn_played >= game.turn_count:
                return False
            # Attacker must be set (i.e., we are in an OnAllyAttack window)
            attacker = context.get('attacker')
            if attacker is None:
                return False
            # Attacker must be one of our own Digimon
            owner = card.owner
            if not owner or attacker not in owner.battle_area:
                return False
            if not attacker.is_digimon:
                return False
            # Attacker must have the [CS] trait
            top = attacker.top_card
            if top is None:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('CS' in t for t in traits):
                return False
            # Must have a valid de-digivolve target
            enemy = owner.enemy
            if not enemy:
                return False
            if not any(
                p.is_digimon
                and game.modifiers.can_be_selected_by_effect(p)
                for p in enemy.battle_area
            ):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Pay Delay cost: trash this option card from battle area.
            delay_perm = card.permanent_of_this_card() if card else None
            if delay_perm and delay_perm in player.battle_area:
                player.battle_area.remove(delay_perm)
                game.cleanup_modifiers_for_permanent(delay_perm)
                for cs in delay_perm.card_sources:
                    player.trash_cards.append(cs)
                game.logger.log(
                    f"[Delay] {player.player_name} trashed "
                    f"{game._card_ref(card)} to activate its delayed effect")
            # Resolve the de-digivolve.
            _run_de_digivolve(player, game)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ─── Effect 3: [Security] <De-Digivolve 4> + place in BA ──────
        # DCGO runs SharedActivateCoroutine (de-digivolve) then
        # PlaceDelayOptionCards (places the card in the battle area).
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name(
            "BT23-096 [Security] <De-Digivolve 4> + place in battle area")
        effect3.set_effect_description(
            "[Security] <De-Digivolve 4> 1 of your opponent's Digimon. Then, "
            "place this card in the battle area.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def place_self_in_ba():
                # Place this card in the battle area from security. This
                # sets card._security_played = True so the security_attack
                # flow does not trash it afterward.
                if card is not None:
                    game.effect_play_from_security(player, card)

            _run_de_digivolve(player, game, after_callback=place_self_in_ba)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
