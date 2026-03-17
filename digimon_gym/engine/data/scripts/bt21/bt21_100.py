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

        # --- Effect 0: Ignore color requirement while Takato ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-100 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req while Takato Matsuki")

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            return any(p.contains_card_name('Takato Matsuki') for p in owner.battle_area)
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # descriptive-tagged: engine handles color requirement ignoring
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Main] Draw 1, trash 1 from hand ---
        # (Place in battle area is handled by engine for options with Delay)
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
            # Trash 1 from hand
            if player.hand_cards:
                game.effect_select_hand_card(
                    player, lambda c: True,
                    callback=lambda selected: (
                        player.hand_cards.remove(selected)
                        if selected in player.hand_cards else None,
                        player.trash_cards.append(selected)),
                    is_optional=False,
                    prompt="Trash 1 card from your hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-100 Delay")
        effect2.set_effect_description("Delay")
        effect2.is_on_deletion = True
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Your Turn] When effects delete Digimon, <Delay>
        #     Digivolve from trash into Growlmon/Gallantmon/Megidramon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT21-100 Digivolve to Gallant line from trash")
        effect3.set_effect_description(
            "[Your Turn] When effects delete Digimon, <Delay>. "
            "1 of your Digimon with [Guilmon] or [Growlmon] in its name may "
            "digivolve into a Digimon card with [Growlmon], [Gallantmon] or "
            "[Megidramon] in its name in the trash without paying the cost.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Select 1 of your Digimon with [Guilmon]/[Growlmon], then select
            a matching Digimon card from trash to digivolve into free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select own Digimon with Guilmon or Growlmon in name
            def own_filter(p):
                if not p.is_digimon:
                    return False
                return (p.contains_card_name('Guilmon') or
                        p.contains_card_name('Growlmon'))

            own_qualifying = [p for p in player.battle_area if own_filter(p)]
            if not own_qualifying:
                return

            # Check if there are valid trash targets
            def trash_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Growlmon' in n or 'Gallantmon' in n or
                           'Megidramon' in n for n in names)

            trash_qualifying = [c for c in player.trash_cards if trash_filter(c)]
            if not trash_qualifying:
                return

            def on_own_selected(own_perm):
                if own_perm is None:
                    return
                # Step 2: Select from trash to digivolve into
                from ....game.constants import SEL_TRASH_START
                from ....data.enums import GamePhase

                valid = [SEL_TRASH_START + i
                         for i, c in enumerate(player.trash_cards)
                         if trash_filter(c)]
                if not valid:
                    return

                def on_trash_select(action_id: int):
                    idx = action_id - SEL_TRASH_START
                    if not (0 <= idx < len(player.trash_cards)):
                        return
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    own_perm.add_card_source(chosen)
                    own_perm.turn_digivolved = game.turn_count
                    game.logger.log(
                        f"[Effect Digivolve] {game._card_ref(chosen)} onto "
                        f"{game._perm_ref(own_perm)} (cost: 0, from trash)")
                    player.draw()
                    game.execute_effects(
                        EffectTiming.WhenDigivolving,
                        {"digivolved_permanent": own_perm})

                game.request_selection(
                    GamePhase.SelectTarget, player, on_trash_select, valid,
                    is_optional=True,
                    prompt="Select a [Growlmon]/[Gallantmon]/[Megidramon] from trash to digivolve into.")

            game.effect_select_own_permanent(
                player, on_own_selected,
                filter_fn=own_filter,
                is_optional=True,
                prompt="Select 1 of your [Guilmon]/[Growlmon] Digimon to digivolve.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Gain 1 memory, place in battle area ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT21-100 Gain 1 memory")
        effect4.set_effect_description(
            "[Security] Gain 1 memory. Then, place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
