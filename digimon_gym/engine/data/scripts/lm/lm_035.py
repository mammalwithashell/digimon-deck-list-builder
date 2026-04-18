from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_035(CardScript):
    """LM-035 Amber Memory Boost! | Option (Yellow, Cost 3)

    Purple also meets this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 yellow or purple Digimon
        card among them to the hand. Return the rest to the bottom of deck.
        Then, place this card in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate
        the effect below.)
        - Gain 2 memory.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # --- Partial color bypass: "Purple also meets this card's color requirements" ---
        # LM-035 natively requires Yellow; this clause additionally accepts Purple
        # Digimon/Tamers on the field as satisfying the color requirement.
        # Implemented via _match_color_requirement_fn: bypass the engine's color
        # check iff the player has a Purple Digimon/Tamer on field; otherwise the
        # engine's default Yellow check applies.
        def _check_color_req():
            owner = card.owner if card else None
            if not owner:
                return True  # enforce (default)
            for p in owner.battle_area:
                top = p.top_card
                if not top:
                    continue
                if not (p.is_digimon or p.is_tamer):
                    continue
                colors = getattr(top, 'card_colors', []) or []
                if any(getattr(c, 'name', None) == 'Purple' for c in colors):
                    return False  # bypass — Purple on field satisfies requirement
            return True  # enforce Yellow via default path
        card._match_color_requirement_fn = _check_color_req

        effects = []

        # --- Effect 0: [Main] Reveal top 3, add 1 yellow/purple Digimon,
        #               rest to deck bottom. Place self in battle area is
        #               handled automatically by the engine because we have
        #               a _is_delay marker below (_option_stays_on_field). ---
        effect_main = ICardEffect()
        effect_main.set_timing(EffectTiming.OptionSkill)
        effect_main.set_effect_name(
            "LM-035 Reveal top 3, add 1 yellow/purple Digimon to hand"
        )
        effect_main.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 yellow or "
            "purple Digimon card among them to the hand. Return the rest to "
            "the bottom of deck. Then, place this card in the battle area."
        )

        def condition_main(context: Dict[str, Any]) -> bool:
            return True
        effect_main.set_can_use_condition(condition_main)

        def process_main(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def yellow_or_purple_digimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                color_names = [getattr(col, 'name', '') for col in colors]
                return 'Yellow' in color_names or 'Purple' in color_names

            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[
                    (yellow_or_purple_digimon_filter, 'hand'),
                ],
                remaining_placement='deck_bottom',
                is_optional=False,
            )
        effect_main.set_on_process_callback(process_main)
        effects.append(effect_main)

        # --- Effect 1: Delay marker ---
        # Marker effect that tells the engine "this Option has a Delay" so
        # _option_stays_on_field() keeps it on the field after main resolution.
        effect_delay_marker = ICardEffect()
        effect_delay_marker.set_effect_name("LM-035 Delay")
        effect_delay_marker.set_effect_description("Delay")
        effect_delay_marker._is_delay = True

        def condition_delay_marker(context: Dict[str, Any]) -> bool:
            # Delay can only activate on the owner's turn, after the placing turn.
            # Turn-placed enforcement is handled by action_mask (perm.turn_played
            # < game.turn_count). No permanent_of_this_card() check here — the
            # engine's _execute_delay trashes the permanent BEFORE calling the
            # body condition/callback.
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect_delay_marker.set_can_use_condition(condition_delay_marker)
        effects.append(effect_delay_marker)

        # --- Effect 2: [Main] <Delay> Gain 2 memory ---
        # Engine's _execute_delay already removes the permanent and trashes the
        # card source BEFORE invoking this callback. Do NOT try to delete the
        # permanent here; just run the body effect.
        effect_delay_body = ICardEffect()
        effect_delay_body.set_timing(EffectTiming.OnDeclaration)
        effect_delay_body._is_field_main = True
        effect_delay_body.set_effect_name("LM-035 Delay: Gain 2 memory")
        effect_delay_body.set_effect_description("[Main] <Delay> Gain 2 memory.")

        def condition_delay_body(context: Dict[str, Any]) -> bool:
            # Don't check permanent_of_this_card() — engine already trashed
            # this card by the time this condition is evaluated.
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect_delay_body.set_can_use_condition(condition_delay_body)

        def process_delay_body(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            # Engine's _execute_delay has already trashed the card source and
            # removed the permanent. Simply grant +2 memory.
            player.add_memory(2)
        effect_delay_body.set_on_process_callback(process_delay_body)
        effects.append(effect_delay_body)

        # --- Effect 3: [Security] Place this card in the battle area. ---
        effect_sec = ICardEffect()
        effect_sec.set_timing(EffectTiming.SecuritySkill)
        effect_sec.set_effect_name("LM-035 Security: Place in battle area")
        effect_sec.set_effect_description(
            "[Security] Place this card in the battle area."
        )
        effect_sec.is_security_effect = True

        def condition_sec(context: Dict[str, Any]) -> bool:
            return True
        effect_sec.set_can_use_condition(condition_sec)

        def process_sec(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            # Use effect_play_from_security so the card is placed in battle
            # area, _security_played flag is set (preventing post-attack
            # trash), and OnEnterFieldAnyone effects fire.
            game.effect_play_from_security(player, card)
        effect_sec.set_on_process_callback(process_sec)
        effects.append(effect_sec)

        return effects
