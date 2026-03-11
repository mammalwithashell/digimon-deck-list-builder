from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_091(CardScript):
    """BT24-091 Tidal Stream | Option (Blue, Cost 5)

    While you have an [TS] trait Digimon or Tamer on the field, you can ignore
    this card's color requirements.
    [Security] Activate this card's [Main] effects.
    [Main] Return all of your opponent's lowest level Digimon to the bottom of
    the deck. If this effect returned at least 1, 1 of your [TS] trait Digimon
    unsuspends. Then, you may link this card to 1 of your Digimon without paying
    the cost.
    [Link] [TS] trait: Cost 3
    [When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level
    Digimon to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements ---
        # Condition: you have a [TS] Digimon or Tamer on the field
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-091 Ignore color requirements")
        effect0.set_effect_description(
            "While you have an [TS] trait Digimon or Tamer on the field, "
            "you can ignore this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            return any(
                (p.is_digimon or p.is_tamer)
                and any('TS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                for p in owner.battle_area
                if p.top_card
            )

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass  # Color requirement bypass — not modeled in engine

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Security — activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT24-091 Security: Activate [Main] effects")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Security activates the main effect — engine handles re-dispatch
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Main] Return all opponent's lowest level Digimon to bottom of deck ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name(
            "BT24-091 Return all opponent's lowest-level Digimon to deck bottom, "
            "unsuspend a [TS] Digimon, then link"
        )
        effect2.set_effect_description(
            "[Main] Return all of your opponent's lowest level Digimon to the bottom of "
            "the deck. If this effect returned at least 1, 1 of your [TS] trait Digimon "
            "unsuspends. Then, you may link this card to 1 of your Digimon on the field "
            "without paying the cost."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Find the minimum level among opponent's Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            bounced_count = 0
            if opp_digimon:
                min_level = min(p.level for p in opp_digimon)
                targets = [p for p in opp_digimon if p.level == min_level]
                for target in list(targets):
                    if target in enemy.battle_area:
                        enemy.return_permanent_to_deck_bottom(target)
                        bounced_count += 1

            # If at least 1 was returned, unsuspend 1 of your [TS] Digimon (not optional)
            if bounced_count > 0:
                def ts_filter(p):
                    if not p.is_digimon:
                        return False
                    return any('TS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))

                def on_unsuspend(target_perm):
                    target_perm.unsuspend()

                game.effect_select_own_permanent(
                    player, on_unsuspend, filter_fn=ts_filter, is_optional=False
                )

            # Then optionally link this card to 1 of your Digimon
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Link] [TS] trait: Cost 3
        #    [When Attacking] [Once Per Turn] Return 1 opponent's lowest level Digimon to hand ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnUseAttack)
        effect3.set_effect_name("BT24-091 Bounce 1 opponent's lowest level Digimon")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level "
            "Digimon to the hand."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WA_BT24-091")
        effect3.is_linked_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            if not opp_digimon:
                return
            min_level = min(p.level for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == min_level

            def on_bounce(target_perm):
                enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False
            )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
