from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_100(CardScript):
    """BT21-100 The Digimon I Designed | Option Purple

    While you have [Takato Matsuki], you can ignore this card's color
        requirements.
    [Main] <Draw 1> and trash 1 card in your hand. Then, place this card
        in the battle area.
    [Your Turn] When effects delete Digimon, <Delay>.
        - 1 of your Digimon with [Guilmon] or [Growlmon] in its name may
          digivolve into a Digimon card with [Growlmon], [Gallantmon] or
          [Megidramon] in its name in the trash without paying the cost.
    [Security] Gain 1 memory. Then, place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirement while Takato Matsuki on field ---
        # DCGO: IgnoreColorConditionClass with HasMatchConditionOwnersPermanent
        # Uses _match_color_requirement_fn so the engine action_mask checks it.
        def _has_takato():
            owner = card.owner if card else None
            if not owner:
                return True  # default: require color
            has_takato = any(
                p.contains_card_name('Takato Matsuki')
                for p in owner.battle_area
            )
            if has_takato:
                return False  # False = skip color requirement check
            return True  # True = normal color requirement applies

        card._match_color_requirement_fn = _has_takato

        # --- Effect 1: [Main] Draw 1, trash 1 from hand ---
        # (Place in battle area handled by engine: _is_delay flag keeps it on field)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT21-100 Draw 1 trash 1")
        effect1.set_effect_description(
            "[Main] <Draw 1> and trash 1 card in your hand. Then, place this "
            "card in the battle area.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Draw 1
            player.draw_cards(1)
            # Trash 1 from hand (mandatory)
            if player.hand_cards:
                def on_trash(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)

                game.effect_select_hand_card(
                    player, lambda c: True,
                    callback=on_trash,
                    is_optional=False,
                    prompt="Trash 1 card from your hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay marker ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-100 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Your Turn] When effects delete Digimon, <Delay>
        #     Digivolve from trash into Growlmon/Gallantmon/Megidramon ---
        # DCGO: OnDestroyedAnyone with TriggerPermanentCondition (IsDigimon)
        #       + IsByEffect + CanDeclareOptionDelayEffect + CanSelectPermanentCondition
        # Uses _is_deletion_observer + NoTiming so _fire_deletion_observers picks it up
        effect3 = ICardEffect()
        effect3._is_deletion_observer = True
        effect3.set_effect_name("BT21-100 Digivolve to Gallant line from trash")
        effect3.set_effect_description(
            "[Your Turn] When effects delete Digimon, <Delay>. "
            "1 of your Digimon with [Guilmon] or [Growlmon] in its name may "
            "digivolve into a Digimon card with [Growlmon], [Gallantmon] or "
            "[Megidramon] in its name in the trash without paying the cost.")
        effect3.is_optional = True

        def _own_perm_filter(p):
            """Filter for own Digimon with Guilmon or Growlmon in name."""
            if not p.is_digimon:
                return False
            return (p.contains_card_name('Guilmon') or
                    p.contains_card_name('Growlmon'))

        def _trash_card_filter(c):
            """Filter for Digimon cards with Growlmon/Gallantmon/Megidramon in name."""
            if not getattr(c, 'is_digimon', False):
                return False
            names = getattr(c, 'card_names', []) or []
            return any('Growlmon' in n or 'Gallantmon' in n or
                       'Megidramon' in n for n in names)

        def condition3(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # [Your Turn] only
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Delay placing-turn restriction: cannot activate on the turn it was placed
            perm = card.permanent_of_this_card()
            game = context.get('game')
            if perm and game and perm.turn_played >= game.turn_count:
                return False
            # DCGO: IsByEffect — only triggers when deletion is by effect, not battle
            removal_cause = context.get('removal_cause', 'effect')
            if removal_cause != 'effect':
                return False
            # DCGO: TriggerPermanentCondition — deleted permanent must be a Digimon
            deleted_perm = context.get('deleted_permanent')
            if not deleted_perm or not getattr(deleted_perm, 'is_digimon', False):
                return False
            # Must have a qualifying own Digimon on field
            owner = card.owner
            if not any(_own_perm_filter(p) for p in owner.battle_area):
                return False
            # Must have a qualifying card in trash
            if not any(_trash_card_filter(c) for c in owner.trash_cards):
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Trash this option (Delay cost), then select own Guilmon/Growlmon
            to digivolve into Growlmon/Gallantmon/Megidramon from trash free."""
            player = card.owner if card else ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # DCGO: First delete the delay option card (Delay cost = trash this card)
            delay_perm = card.permanent_of_this_card() if card else None
            if delay_perm and delay_perm in player.battle_area:
                player.battle_area.remove(delay_perm)
                for source in delay_perm.card_sources:
                    player.trash_cards.append(source)

            # Select own Digimon with Guilmon/Growlmon in name
            def on_selected(target_perm):
                if target_perm is None:
                    return
                # Digivolve into Growlmon/Gallantmon/Megidramon from trash, cost 0
                game.effect_digivolve_from_hand(
                    player, target_perm, _trash_card_filter,
                    cost_override=0, is_optional=True,
                    include_trash=True,
                    prompt="Select a [Growlmon]/[Gallantmon]/[Megidramon] Digimon "
                           "card from trash to digivolve into.")

            game.effect_select_own_permanent(
                player, on_selected,
                filter_fn=_own_perm_filter,
                is_optional=True,
                prompt="Select 1 of your [Guilmon]/[Growlmon] Digimon to digivolve.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Gain 1 memory, place in battle area ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT21-100 Security: Gain 1 memory, place in battle area")
        effect4.set_effect_description(
            "[Security] Gain 1 memory. Then, place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Gain 1 memory
            player.add_memory(1)
            # Place this card in the battle area
            game.effect_play_from_security(player, card)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
