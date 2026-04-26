from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST21_15(CardScript):
    """ST21-15 Gennai's House | Option"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # While you have no face-up [Gennai's House] security cards, you can
        # ignore this card's color requirements.
        effect_color = ICardEffect()
        effect_color.set_effect_name("ST21-15 Ignore color requirements without face-up Gennai's House")
        effect_color.set_effect_description(
            "While you have no face-up [Gennai's House] security cards, "
            "you can ignore this card's color requirements."
        )
        effect_color.is_declarative = True
        effect_color._ignore_color_requirement = True

        def condition_color(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Check if there are any face-up "Gennai's House" security cards
            for sec_card in owner.face_up_security:
                names = getattr(sec_card, 'card_names', []) or []
                for n in names:
                    if "Gennai's House" in n:
                        return False  # Has face-up Gennai's House, can't ignore
            return True

        effect_color.set_can_use_condition(condition_color)
        effects.append(effect_color)

        # [Security] [Your Turn] All of your level 3 or higher Digimon get +3000 DP.
        # This is a continuous security effect — while face-up in security,
        # it grants +3000 DP to all your Lv.3+ Digimon during your turn.
        effect_sec_buff = ICardEffect()
        effect_sec_buff.set_effect_name("ST21-15 Security buff: +3000 DP to Lv.3+ Digimon")
        effect_sec_buff.set_effect_description(
            "[Security] [Your Turn] All of your level 3 or higher Digimon get +3000 DP."
        )
        effect_sec_buff.is_declarative = True

        def condition_sec_buff(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            # This effect is active while this card is face-up in security
            return card in owner.face_up_security

        effect_sec_buff.set_can_use_condition(condition_sec_buff)

        # The DP buff is handled via a continuous modifier — but since the card
        # is in security (not on field), we use a declarative dp_buff approach.
        # We set _dp_modifier_condition to apply +3000 to all own Lv.3+ Digimon
        def dp_buff_condition(permanent, context):
            """Return True if this permanent should get the DP buff."""
            owner = card.owner if card else None
            if not owner:
                return False
            if not owner.is_my_turn:
                return False
            if card not in owner.face_up_security:
                return False
            if not permanent.is_digimon:
                return False
            if (permanent.level or 0) < 3:
                return False
            return permanent in owner.battle_area

        effect_sec_buff._dp_modifier_value = 3000
        effect_sec_buff._dp_modifier_condition = dp_buff_condition
        effects.append(effect_sec_buff)

        # [Main] Add your bottom security card to the hand. Then, place this
        # card face up as the bottom security card.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("ST21-15 Main: Move bottom security to hand, place self as security")
        effect0.set_effect_description(
            "[Main] Add your bottom security card to the hand. Then, place "
            "this card face up as the bottom security card."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Add bottom security card to hand
            if player.security_cards:
                bottom_card = player.security_cards[-1]
                player.security_cards.remove(bottom_card)
                player.face_up_security.discard(bottom_card)
                player.hand_cards.append(bottom_card)
                if game:
                    game.logger.log(
                        f"[ST21-15] {player.player_name} added bottom security "
                        f"card to hand."
                    )

            # Place this card face up as the bottom security card
            # The option card should be in hand or on field after resolution
            # After an option resolves, it goes to trash. But this card's
            # effect places it into security instead.
            # We need to intercept the trash and redirect to security.
            # Mark card for security placement (engine checks _redirect_to_security)
            if card:
                player.security_cards.append(card)
                player.face_up_security.add(card)
                if game:
                    game.logger.log(
                        f"[ST21-15] Placed face up as bottom security card."
                    )
                # Prevent the option from being trashed after resolution
                # by removing it from the permanent that the engine will trash
                perm = card.permanent_of_this_card() if card else None
                if perm and perm in player.battle_area:
                    player.battle_area.remove(perm)
                    if game and hasattr(game, 'cleanup_modifiers_for_permanent'):
                        game.cleanup_modifiers_for_permanent(perm)
                    # Remove card from permanent's sources so it won't be
                    # double-trashed by the option cleanup
                    if card in perm.card_sources:
                        perm.card_sources.remove(card)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security Effect: [Security] You may play 1 level 3 Digimon card from
        # your trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST21-15 Security: Play Lv.3 from trash free")
        effect1.set_effect_description(
            "[Security] You may play 1 level 3 Digimon card from your trash "
            "without paying the cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def lv3_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return (getattr(c, 'level', None) or 0) == 3

            game.effect_play_from_zone(
                player, 'trash', lv3_filter,
                free=True, is_optional=True,
                prompt="You may play 1 level 3 Digimon card from your trash without paying the cost."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
