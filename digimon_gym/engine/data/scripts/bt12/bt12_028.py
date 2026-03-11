from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_028(CardScript):
    """BT12-028 Paildramon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── When Digivolving ──────────────────────────────────────────────────
        # Trash the top 3 digivolution cards of all of your opponent's Digimon.
        # Then, if DNA digivolving, 2 of your opponent's Digimon with no
        # digivolution cards can't attack until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT12-028 Trash digi-cards + can't attack if DNA")
        effect0.set_effect_description(
            "[When Digivolving] Trash the top 3 digivolution cards of all of your "
            "opponent's Digimon. Then, when DNA digivolving, 2 of your opponent's "
            "Digimon with no digivolution cards in play can't attack until the end "
            "of your opponent's turn."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Trash top 3 digivolution cards from each opponent Digimon that
            # has at least 1 digivolution card (has_no_digivolution_cards == False).
            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                if opp_perm.has_no_digivolution_cards:
                    continue
                opp_perm.trash_digivolution_cards(3, from_top=True)

            # If DNA digivolving, select up to 2 opponent Digimon with no
            # digivolution cards and apply CANNOT_ATTACK until end of opp turn.
            is_dna = ctx.get('is_dna_digivolve', False)
            if not is_dna:
                return

            from digimon_gym.engine.interfaces.modifiers import ModifierType

            def _no_digi_cards(p):
                return p.is_digimon and p.has_no_digivolution_cards

            candidates = [p for p in enemy.battle_area if _no_digi_cards(p)]
            # Select up to 2 — greedy auto-select (engine handles selection UI)
            selected = []

            def on_select(target_perm):
                selected.append(target_perm)

            max_count = min(2, len(candidates))
            for _ in range(max_count):
                remaining = [p for p in candidates if p not in selected]
                if not remaining:
                    break
                game.effect_select_opponent_permanent(
                    player, on_select,
                    filter_fn=lambda p: p.is_digimon and p.has_no_digivolution_cards and p not in selected,
                    is_optional=False
                )

            for target in selected:
                game.register_modifier(
                    ModifierType.CANNOT_ATTACK, target,
                    value_fn=lambda: True,
                    expiry='end_of_opponent_turn'
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Inherited: End of Attack ──────────────────────────────────────────
        # If this Digimon has [Imperialdramon] in its name or [Free] trait,
        # gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndAttack)
        effect1.set_effect_name("BT12-028 End of Attack gain 1 memory (inherited)")
        effect1.set_effect_description(
            "[End of Attack] If this Digimon has [Imperialdramon] in its name or "
            "a [Free] trait, gain 1 memory."
        )
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None:
                return False
            if perm.contains_card_name('Imperialdramon'):
                return True
            if perm.has_trait('Free'):
                return True
            return False

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
