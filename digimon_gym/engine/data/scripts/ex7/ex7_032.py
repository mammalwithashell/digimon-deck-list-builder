from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_032(CardScript):
    """EX7-032 Galemon | Lv.4 Green Bird Dragon/LIBERATOR

    [When Digivolving] If you have 1 or fewer Tamers, you may play 1 [Shoto Kazama]
        from your hand without paying the cost.
    Inherited [All Turns] [Once Per Turn] When this Digimon deletes your opponent's
        Digimon in battle, gain 1 memory.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] If you have 1 or fewer Tamers, play 1 [Shoto Kazama] free ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX7-032 Play 1 [Shoto Kazama] from hand if 1 or fewer Tamers")
        effect0.set_effect_description(
            "[When Digivolving] If you have 1 or fewer Tamers, you may play 1 "
            "[Shoto Kazama] from your hand without paying the cost."
        )
        effect0.is_when_digivolving = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Check 1 or fewer Tamers in battle area
            owner = card.owner if card else None
            if owner is None:
                return False
            tamer_count = sum(1 for p in owner.battle_area if p.is_tamer)
            if tamer_count > 1:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play [Shoto Kazama] from hand free"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Shoto Kazama' in n for n in names)
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Inherited [All Turns] [Once Per Turn] When deletes opponent in battle, gain 1 memory ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndBattle)
        effect1.set_effect_name("EX7-032 When deletes opponent Digimon in battle, gain 1 memory")
        effect1.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon deletes your opponent's "
            "Digimon in battle, gain 1 memory."
        )
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("EX7_032_KillGainMemory")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
