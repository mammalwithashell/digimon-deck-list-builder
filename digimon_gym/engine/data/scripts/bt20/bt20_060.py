from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_060(CardScript):
    """BT20-060 Alphamon: Ouryuken | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── DNA Digivolution Condition ─────────────────────────────────────
        # Jogress (DNA) requires: a Black Lv6 + a Yellow Lv6 on field
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-060 Jogress Condition")
        effect0.set_effect_description("Jogress Condition")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── [Hand][Counter] Blast DNA Digivolve ([Alphamon] + [Ouryumon]) ──
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-060 Blast DNA Digivolve")
        effect1.set_effect_description("[Hand] [Counter] Blast DNA Digivolve ([Alphamon] + [Ouryumon])")
        effect1.is_counter_effect = True
        effect1._is_blast_dna_digivolve = True
        # DNA requirements: one field permanent named Alphamon + one named Ouryumon
        effect1._blast_dna_names = ["Alphamon", "Ouryumon"]

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── On Play / When Digivolving shared effect factory ───────────────
        def _make_on_enter_effect(is_when_digivolving: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT20-060 Opponent Digimon -15000 DP; if DNA trash security + Recovery")
            if is_when_digivolving:
                effect.is_when_digivolving = True
                effect.set_effect_description(
                    "[When Digivolving] 1 of your opponent's Digimon gets -15000 DP until the end of "
                    "their turn. Then, if DNA digivolving, trash your opponent's top security card and "
                    "Recovery +1 (Deck)."
                )
            else:
                effect.is_on_play = True
                effect.set_effect_description(
                    "[On Play] 1 of your opponent's Digimon gets -15000 DP until the end of their turn. "
                    "Then, if DNA digivolving, trash your opponent's top security card and Recovery +1 (Deck)."
                )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return
                from digimon_gym.engine.interfaces.modifiers import ModifierType

                enemy = player.enemy
                if not enemy:
                    return

                # Select 1 of opponent's Digimon and apply -15000 DP until end of opponent's turn
                def on_target(target_perm):
                    game.register_modifier(
                        target_perm,
                        ModifierType.CHANGE_DP,
                        value_fn=lambda current, p, c: current - 15000,
                        expiry='end_of_opponent_turn',
                    )

                has_target = any(p.is_digimon for p in enemy.battle_area)
                if has_target:
                    game.effect_select_opponent_permanent(
                        player, on_target,
                        filter_fn=lambda p: p.is_digimon,
                        is_optional=False,
                        prompt="Select 1 of your opponent's Digimon to give -15000 DP until end of their turn.",
                    )

                # Then, if DNA digivolving, trash opponent's top security card and Recovery +1
                is_dna = ctx.get('is_dna_digivolve', False)
                if is_dna:
                    if enemy.security_cards:
                        enemy.trash_security_card(enemy.security_cards[0])
                    if player.library_cards:
                        player.recovery(1)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_on_enter_effect(is_when_digivolving=False))
        effects.append(_make_on_enter_effect(is_when_digivolving=True))

        # ── [All Turns][Once Per Turn] When security stacks are removed from, gain 3 memory. ──
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT20-060 Gain 3 memory")
        effect4.set_effect_description(
            "[All Turns] [Once Per Turn] When security stacks are removed from, gain 3 memory."
        )
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("RemovedSec_BT20_060")

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(3)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # ── Ace Overflow <-5> (descriptive — engine handles via card attributes) ──
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-060 Ace Overflow")
        effect5.set_effect_description("Ace Overflow <-5>")
        effect5.is_inherited_effect = True
        # ace_overflow driven by card entity attributes, not by process callback

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
