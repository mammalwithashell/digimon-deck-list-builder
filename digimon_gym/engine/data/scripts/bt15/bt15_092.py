from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_092(CardScript):
    """BT15-092 Revelation of Light"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes this card from the security stack, activate this card's [Security] effects.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDiscardSecurity)
        effect0.set_effect_name("BT15-092 Activate security effect")
        effect0.set_effect_description("When an effect trashes this card from the security stack, activate this card's [Security] effects.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _security_search_and_play(player, game, option_card_source):
            """Search security stack, play 1 yellow Lv4 or lower Digimon free, shuffle, maybe return self to top."""
            import random

            def sec_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                colors = [col.name for col in getattr(c, 'card_colors', [])]
                if 'Yellow' not in colors:
                    return False
                return True

            # Check if any valid target exists in security
            has_target = any(sec_filter(c) for c in player.security_cards)

            def after_selection(selected_card):
                # Remove selected from security and play it free
                removed = player.remove_from_security(selected_card)
                if removed:
                    played_perm = player.play_card_from_source(removed, pay_cost=False)
                    if played_perm and game:
                        game.execute_effects(EffectTiming.OnEnterFieldAnyone, {
                            'played_card': removed,
                            'played_permanent': played_perm,
                            'event_permanent': played_perm,
                            'event_player': player,
                        })
                # Shuffle security stack
                random.shuffle(player.security_cards)
                # If you have a Tamer with [Kari Kamiya] in its name, place this card on top of security
                _maybe_return_to_security(player, option_card_source)

            def on_decline():
                # Player chose not to play — still shuffle security
                random.shuffle(player.security_cards)
                # Conditional self-placement still applies
                _maybe_return_to_security(player, option_card_source)

            if has_target:
                game.effect_select_own_security(
                    player, sec_filter, after_selection, is_optional=True,
                    prompt="You may play 1 yellow level 4 or lower Digimon card from your security stack without paying the cost.")
            else:
                # No valid targets — shuffle security and check Kari condition
                random.shuffle(player.security_cards)
                _maybe_return_to_security(player, option_card_source)

        def _maybe_return_to_security(player, option_card_source):
            """If player has a Tamer with [Kari Kamiya] in its name, place the option card on top of security."""
            has_kari = False
            for p in player.battle_area:
                if not getattr(p, 'is_tamer', False):
                    continue
                names = []
                if p.top_card:
                    names = getattr(p.top_card, 'card_names', []) or []
                if any('Kari Kamiya' in n for n in names):
                    has_kari = True
                    break
            if has_kari and option_card_source is not None:
                # Place this option card on top of security stack (append = top in this engine)
                if option_card_source not in player.security_cards:
                    player.security_cards.append(option_card_source)

        # Timing: EffectTiming.OptionSkill
        # [Main] Search your security stack. You may play 1 yellow level 4 or lower Digimon card among it
        # without paying the cost. Then, shuffle your security stack. If you have a Tamer with
        # [Kari Kamiya] in its name, place this card on top of your security stack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT15-092 Play 1 Digimon from security")
        effect1.set_effect_description("[Main] Search your security stack. You may play 1 yellow level 4 or lower Digimon card among it without paying the cost. Then, shuffle your security stack. If you have a Tamer with [Kari Kamiya] in its name, place this card on top of your security stack.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Search security, play 1 yellow Lv4- Digimon free, shuffle, maybe return self to top"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # The option card source is the card being played (this option card)
            option_card_source = card
            _security_search_and_play(player, game, option_card_source)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Search your security stack. You may play 1 yellow level 4 or lower Digimon card among it
        # without paying the cost. Then, shuffle your security stack. If you have a Tamer with
        # [Kari Kamiya] in its name, place this card on top of your security stack.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT15-092 Security: Play 1 Digimon from security")
        effect2.set_effect_description("[Security] Search your security stack. You may play 1 yellow level 4 or lower Digimon card among it without paying the cost. Then, shuffle your security stack. If you have a Tamer with [Kari Kamiya] in its name, place this card on top of your security stack.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Security: Same as main effect"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            option_card_source = card
            _security_search_and_play(player, game, option_card_source)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
