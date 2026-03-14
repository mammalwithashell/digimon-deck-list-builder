from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_094(CardScript):
    """BT24-094 Central Town: Throne Room | Option (Green/Yellow, Cost 3)

    While you have no face-up security cards, you can ignore this card's color
    requirements.
    [Security] [All Turns] All of your green or yellow [TS] trait Digimon get
    +2000 DP. While you have [Merukimon] or [Minervamon], all of your green or
    yellow [TS] trait Digimon gain <Alliance>.
    [Main] Add your bottom security card to the hand and place this card face up
    as the bottom security card. Then, you may play 1 green or yellow [TS] trait
    Digimon card from your hand with the play cost reduced by 3.
    [Security] You may play 1 level 4 or lower green or yellow [TS] trait Digimon
    card from your hand or trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Helper: check if permanent is a green or yellow Digimon with [TS] trait
        def _is_green_yellow_ts(perm):
            top = getattr(perm, 'top_card', None)
            if not top:
                return False
            if not getattr(perm, 'is_digimon', False):
                return False
            colors = [c.name for c in (getattr(top, 'card_colors', None) or [])]
            if 'Green' not in colors and 'Yellow' not in colors:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any('TS' in t for t in traits)

        # --- Effect 0: Ignore color requirements (while no face-up security cards) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-094 Ignore color requirements")
        effect0.set_effect_description(
            "While you have no face-up security cards, you can ignore this card's "
            "color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            face_up_count = sum(
                1 for c in owner.security_cards
                if getattr(c, 'is_flipped', False)
            )
            return face_up_count == 0

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # descriptive-tagged: ignore_color_req

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] [All Turns] +2000 DP for green/yellow [TS] Digimon ---
        # Active while this card is face-down in security.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-094 All green or yellow [TS] Digimon +2000 DP")
        effect1.set_effect_description(
            "[All Turns] All of your green or yellow [TS] trait Digimon get +2000 DP.")
        effect1.dp_modifier = 2000
        effect1._applies_to_all_own_digimon = True

        def dp_permanent_condition(permanent) -> bool:
            return _is_green_yellow_ts(permanent)

        effect1._dp_permanent_condition = dp_permanent_condition

        def condition1(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Active while card is in security (face-down)
            return card in owner.security_cards

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Alliance for green/yellow [TS] Digimon ---
        # Requires Merukimon or Minervamon on field.
        # Uses _applies_to_all_own_digimon aura pattern.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-094 Grant Alliance to green or yellow [TS] Digimon")
        effect2.set_effect_description(
            "[All Turns] While you have [Merukimon] or [Minervamon], all of your "
            "green or yellow [TS] trait Digimon gain <Alliance>.")
        effect2._is_alliance = True
        effect2._applies_to_all_own_digimon = True
        effect2._keyword_permanent_condition = _is_green_yellow_ts

        def condition2(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Only active while card is in security (face-down)
            if card not in owner.security_cards:
                return False
            # Requires Merukimon or Minervamon
            if not any(
                p.contains_card_name('Merukimon') or p.contains_card_name('Minervamon')
                for p in owner.battle_area
            ):
                return False
            # Check target permanent
            ctx_perm = context.get('permanent')
            if ctx_perm is not None:
                return _is_green_yellow_ts(ctx_perm)
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Main] Swap bottom security, play green/yellow [TS] Digimon ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OptionSkill)
        effect3.set_effect_name(
            "BT24-094 Replace bottom security with this face-up card, play a [TS] Digimon for -3")
        effect3.set_effect_description(
            "[Main] Add your bottom security card to the hand and place this card "
            "face up as the bottom security card. Then, you may play 1 green or "
            "yellow [TS] trait Digimon card from your hand with the play cost "
            "reduced by 3.")

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Add bottom security card to hand
            if player.security_cards:
                bottom_sec = player.security_cards.pop(-1)
                player.hand_cards.append(bottom_sec)

            # Step 2: Place this option card face-up as bottom security card
            if card:
                player.security_cards.insert(0, card)

            # Step 3: Optionally play 1 green/yellow [TS] Digimon from hand with cost -3
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                if 'Green' not in colors and 'Yellow' not in colors:
                    return False
                if not any('TS' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter,
                free=False, manual_reduction=3, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Play level 4 or lower green/yellow [TS] Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name(
            "BT24-094 Play 1 lvl 4- Green or Yellow [TS] Digimon from hand or trash")
        effect4.set_effect_description(
            "[Security] You may play 1 level 4 or lower green or yellow [TS] trait "
            "Digimon card from your hand or trash without paying the cost.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                lv = getattr(c, 'level', None)
                if lv is None or lv > 4:
                    return False
                colors = [col.name for col in (getattr(c, 'card_colors', None) or [])]
                if 'Green' not in colors and 'Yellow' not in colors:
                    return False
                if not any('TS' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
