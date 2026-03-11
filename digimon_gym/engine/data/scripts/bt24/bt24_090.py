from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_090(CardScript):
    """BT24-090 Abyss Sanctuary: Throne Room | Option (Blue/Yellow, Cost 3)

    While you have no face-up security cards, you can ignore this card's color requirements.
    [Security] [All Turns] All of your blue or yellow [TS] trait Digimon get +2000 DP.
    While you have [Neptunemon] or [Venusmon], this effect can't be removed.
    [Main] You may play 1 level 4 or lower blue or yellow [TS] trait Digimon card from
    your hand or trash without paying the cost. Then, place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements (while no face-up security cards) ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-090 Ignore color requirements")
        effect0.set_effect_description(
            "While you have no face-up security cards, you can ignore this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # No face-up security cards
            face_up_count = sum(
                1 for c in owner.security_cards
                if getattr(c, 'is_flipped', False)
            )
            return face_up_count == 0

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # Color requirement bypass — not modeled in engine

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] [All Turns] +2000 DP for blue or yellow [TS] Digimon ---
        # Active while this card is face-down in security (IsExistInSecurity face-up=false)
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-090 All blue or yellow [TS] Digimon +2000 DP")
        effect1.set_effect_description(
            "[All Turns] All of your blue or yellow [TS] trait Digimon get +2000 DP."
        )
        effect1.dp_modifier = 2000
        effect1._applies_to_all_own_digimon = True

        def dp_permanent_condition(permanent) -> bool:
            top = getattr(permanent, 'top_card', None)
            if not top:
                return False
            colors = [c.name for c in (getattr(top, 'card_colors', None) or [])]
            if 'Blue' not in colors and 'Yellow' not in colors:
                return False
            traits = getattr(top, 'card_traits', []) or []
            if not any('TS' in t for t in traits):
                return False
            return True

        def condition1(context: Dict[str, Any]) -> bool:
            # Active while card is in security (face-down)
            owner = card.owner if card else None
            if not owner:
                return False
            return card in owner.security_cards

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Alliance for blue/yellow [TS] Digimon
        #    while Neptunemon or Venusmon exists (effect can't be removed) ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-090 Grant Alliance to blue or yellow [TS] Digimon")
        effect2.set_effect_description(
            "[All Turns] All of your blue or yellow [TS] trait Digimon gain <Alliance>. "
            "While you have [Neptunemon] or [Venusmon], this effect can't be removed."
        )
        effect2._is_alliance = True

        def condition2(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            # Only active while card is in security (face-down)
            if card not in owner.security_cards:
                return False
            # Requires Neptunemon or Venusmon for activation of Alliance grant
            return any(
                p.contains_card_name('Neptunemon') or p.contains_card_name('Venusmon')
                for p in owner.battle_area
            )

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            pass  # Declarative Alliance grant

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Main] You may play 1 level 4 or lower blue or yellow [TS] trait
        #    Digimon card from your hand or trash without paying the cost.
        #    Then, place this card in the battle area. ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OptionSkill)
        effect3.set_effect_name(
            "BT24-090 Play Lv4- blue/yellow [TS] Digimon free, place this in battle area"
        )
        effect3.set_effect_description(
            "[Main] You may play 1 level 4 or lower blue or yellow [TS] trait Digimon card "
            "from your hand or trash without paying the cost. Then, place this card in the "
            "battle area."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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
                if 'Blue' not in colors and 'Yellow' not in colors:
                    return False
                if not any('TS' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True

            # Play from hand or trash without paying cost
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter,
                free=True, is_optional=True,
                prompt="You may play 1 level 4 or lower blue or yellow [TS] Digimon from hand or trash without paying the cost."
            )

            # Place this option card in the battle area
            if card:
                from ....core.permanent import Permanent
                from ....core.card_source import CardSource
                perm = Permanent([card])
                perm._owner_game = game
                player.battle_area.append(perm)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Play level 4 or lower blue/yellow [TS] Digimon
        #    from hand or trash without paying cost ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name(
            "BT24-090 Play 1 lvl 4- Blue or Yellow [TS] Digimon from hand or trash"
        )
        effect4.set_effect_description(
            "[Security] You may play 1 level 4 or lower blue or yellow [TS] trait Digimon "
            "card from your hand or trash without paying the cost."
        )
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
                if 'Blue' not in colors and 'Yellow' not in colors:
                    return False
                if not any('TS' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
