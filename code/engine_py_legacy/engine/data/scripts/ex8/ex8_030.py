from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType, ModifierEntry
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _is_tamer_sourced_effect(source_effect) -> bool:
    """Return True if ``source_effect`` counts as a Tamer card effect.

    Mirrors DCGO's ``ICardEffect.IsTamerEffect`` check. Since Python scripts
    rarely set ``is_tamer_effect`` or ``effect_source_card`` explicitly, we
    also fall back to inspecting the effect's ``effect_source_permanent``
    (set by the engine's effect-collection pass) — if the permanent's top
    card is a Tamer, the effect counts as a Tamer effect.
    """
    if source_effect is None:
        return False
    if getattr(source_effect, 'is_tamer_effect', False):
        return True
    src_card = getattr(source_effect, 'effect_source_card', None)
    if src_card is not None and getattr(src_card, 'is_tamer', False):
        return True
    src_perm = getattr(source_effect, 'effect_source_permanent', None)
    if src_perm is not None:
        top = getattr(src_perm, 'top_card', None)
        if top is not None and getattr(top, 'is_tamer', False):
            return True
    return False


class EX8_030(CardScript):
    """EX8-030 Tapirmon | Lv.3 Yellow Digimon (DP 2000, Cost 3)

    [All Turns] Your opponent can't gain memory other than by Tamer effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX8-030 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_trait = "NSo"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [All Turns] Opponent can't gain memory except by Tamer effects.
        # Registered as CANNOT_ADD_MEMORY modifier targeting the opponent.
        # The engine enforces this in Player.add_memory(), passing the
        # currently-processing effect as ``source_effect`` so we can
        # allow-list Tamer-sourced gains per DCGO's !IsTamerEffect check.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX8-030 Opponent can't gain memory except by Tamers")
        effect1.set_effect_description(
            "[All Turns] Your opponent can't gain memory other than by Tamer effects."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Field-presence check — the modifier is only live while this
            # permanent is on the battle area (CardEffectCommons.IsExistOnBattleArea
            # in the C# CanUseCondition).
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            def mem_block_condition(_timing, c, p=perm, e=enemy):
                # PlayerCondition: only block the opponent (enemy of perm owner).
                target_player = c.get('player')
                if target_player is not e:
                    return False
                # Field-presence gate — modifier is dead if source left the field.
                owner = e.enemy
                if owner is None or p not in owner.battle_area:
                    return False
                # CardEffectCondition: block only when the source effect is
                # NOT a Tamer effect (mirrors DCGO's !IsTamerEffect check).
                # If the source effect is None (engine-level gain with no
                # tracked source, e.g. unsuspend refund), we DO NOT block —
                # only card-effect-sourced gains fall under this restriction.
                source_effect = c.get('source_effect')
                if source_effect is None:
                    return False
                if _is_tamer_sourced_effect(source_effect):
                    return False
                return True

            game.modifiers.register(ModifierEntry(
                modifier_type=ModifierType.CANNOT_ADD_MEMORY,
                condition=mem_block_condition,
                source_permanent=perm,
                expiry='permanent',
            ))
            game.logger.log(
                "[EX8-030] Opponent can't gain memory except by Tamer effects "
                "(CANNOT_ADD_MEMORY registered)"
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
