from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_065(CardScript):
    """EX9-065 Titamon | Lv.6 | Shaman/DM/Ver.4
    [Hand][Counter] <Blast Digivolve>
    <Scapegoat>
    [On Play][When Digivolving] Play 1 Lv.4 or lower [DM] Digimon from trash free.
    [All Turns] All [Ver.4] trait Digimon gain <Blocker> and <Retaliation>.
    Inherited: Ace Overflow <-4>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-065 Blast Digivolve")
        effect0.set_effect_description("[Hand][Counter] Blast Digivolve")
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Scapegoat
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-065 Scapegoat")
        effect1.set_effect_description("Scapegoat")
        effect1._is_scapegoat = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [On Play] Play Lv.4 or lower DM from trash
        def _play_dm_lv4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def dm_lv4_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('DM' in t for t in traits)

            game.effect_play_from_zone(
                player, 'trash', dm_lv4_filter, free=True, is_optional=True,
                prompt="Play 1 Lv.4 or lower [DM] Digimon from trash.")

        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX9-065 On Play: Play DM Lv4 from trash")
        effect2.set_effect_description("[On Play] Play 1 Lv.4 or lower [DM] from trash.")
        effect2.is_on_play = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_dm_lv4)
        effects.append(effect2)

        # [When Digivolving] Same
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX9-065 When Digivolving: Play DM Lv4 from trash")
        effect3.set_effect_description("[When Digivolving] Play 1 Lv.4 or lower [DM] from trash.")
        effect3.is_when_digivolving = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_play_dm_lv4)
        effects.append(effect3)

        # [All Turns] Grant Blocker and Retaliation to all [Ver.4] Digimon
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX9-065 Grant Blocker+Retaliation to Ver.4")
        effect4.set_effect_description(
            "[All Turns] All [Ver.4] trait Digimon gain Blocker and Retaliation."
        )

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            for p in list(player.battle_area):
                if p.is_digimon and p.has_trait('Ver.4'):
                    p.grant_keyword('_is_blocker')
                    p.grant_keyword('_is_retaliation')
        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Ace Overflow <-4>
        effect5 = ICardEffect()
        effect5.set_effect_name("EX9-065 Ace Overflow")
        effect5.set_effect_description("Ace Overflow <-4>")
        effect5.is_inherited_effect = True
        # ace_overflow handled by card entity attributes

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
