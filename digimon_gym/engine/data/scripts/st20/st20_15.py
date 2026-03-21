from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_15(CardScript):
    """ST20-15 Island of Adventure | Option White Cost 2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Dynamic color bypass: ignore color requirements while no face-up IoA in security
        def _check_ioa_color_req():
            owner = card.owner if card else None
            if not owner:
                return True
            # Can ignore color if no face-up Island of Adventure in security
            for sec_card in owner.security_cards:
                if owner.is_security_face_up(sec_card):
                    names = getattr(sec_card, 'card_names', []) or []
                    if any('Island of Adventure' in n for n in names):
                        return True  # enforce color requirement (already have one face-up)
            return False  # bypass color requirement
        card._match_color_requirement_fn = _check_ioa_color_req
        effects = []

        # --- Effect 1: [Security] [All Turns] All of your level 3 or higher
        #     Digimon get +2000 DP. ---
        # This is a continuous effect active while this card is face-up in security.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST20-15 Security: All Lv.3+ Digimon +2000 DP")
        effect1.set_effect_description(
            "[Security] [All Turns] All of your level 3 or higher Digimon "
            "get +2000 DP."
        )
        effect1.dp_modifier = 2000
        effect1._applies_to_all_own_digimon = True

        def dp_permanent_condition(permanent) -> bool:
            """Only applies to level 3 or higher Digimon."""
            top = getattr(permanent, 'top_card', None)
            if not top:
                return False
            level = getattr(permanent, 'level', None)
            if level is None or level < 3:
                return False
            return True

        effect1._dp_permanent_condition = dp_permanent_condition

        def condition1(context: Dict[str, Any]) -> bool:
            # Active while card is face-up in security
            owner = card.owner if card else None
            if not owner:
                return False
            if card not in owner.security_cards:
                return False
            return owner.is_security_face_up(card)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Main] Add your top security card to the hand. Then,
        #     place this card face up as the top security card. ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("ST20-15 Swap top security to hand, place self face-up")
        effect2.set_effect_description(
            "[Main] Add your top security card to the hand. Then, place this "
            "card face up as the top security card."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Add top security to hand, then place this card face-up as top security."""
            player = ctx.get('player')
            if not player:
                return
            # Add top security card to hand (top = last in list)
            if player.security_cards:
                top_sec = player.security_cards[-1]
                player.security_cards.remove(top_sec)
                player.face_up_security.discard(top_sec)
                player.hand_cards.append(top_sec)

            # Place this card face-up as the top security card
            if card:
                player.add_to_security_face_up(card, to_top=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] You may play 1 Tamer card from your hand
        #     without paying the cost. ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("ST20-15 Security: Play 1 Tamer from hand free")
        effect3.set_effect_description(
            "[Security] You may play 1 Tamer card from your hand without "
            "paying the cost."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play 1 Tamer from hand without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def tamer_filter(c):
                return getattr(c, 'is_tamer', False)

            game.effect_play_from_zone(
                player, 'hand', tamer_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
