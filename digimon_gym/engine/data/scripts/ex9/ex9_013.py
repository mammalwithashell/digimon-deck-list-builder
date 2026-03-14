from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_013(CardScript):
    """EX9-013 BlitzGreymon | Lv.6

    [Hand] [Counter] <Blast Digivolve>
    <Alliance> <Blocker>
    [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
    [End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
    in the hand. Then, 1 of your Digimon may attack.
    Inherited: Ace Overflow <-4>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect_blast = ICardEffect()
        effect_blast.set_effect_name("EX9-013 Blast Digivolve")
        effect_blast.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect_blast.is_counter_effect = True
        effect_blast._is_blast_digivolve = True

        def condition_blast(context: Dict[str, Any]) -> bool:
            return True
        effect_blast.set_can_use_condition(condition_blast)
        effects.append(effect_blast)

        # Alliance
        effect_alliance = ICardEffect()
        effect_alliance.set_effect_name("EX9-013 Alliance")
        effect_alliance.set_effect_description("<Alliance>")
        effect_alliance._is_alliance = True

        def condition_alliance(context: Dict[str, Any]) -> bool:
            return True
        effect_alliance.set_can_use_condition(condition_alliance)
        effects.append(effect_alliance)

        # Blocker
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX9-013 Blocker")
        effect_blocker.set_effect_description("<Blocker>")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # Shared process for De-Digivolve 3
        def _de_digivolve_process(ctx: Dict[str, Any]):
            """<De-Digivolve 3> 1 of opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def on_select(target_perm):
                removed = target_perm.de_digivolve(3)
                for c in removed:
                    enemy.trash_cards.append(c)

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon and len(p.card_sources) > 1,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to De-Digivolve 3.")

        # [On Play] De-Digivolve 3
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("EX9-013 De-Digivolve 3")
        effect_op.set_effect_description("[On Play] <De-Digivolve 3> 1 of your opponent's Digimon.")
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_de_digivolve_process)
        effects.append(effect_op)

        # [When Digivolving] De-Digivolve 3
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX9-013 De-Digivolve 3")
        effect_wd.set_effect_description("[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.")
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_de_digivolve_process)
        effects.append(effect_wd)

        # [End of Your Turn] DNA digivolve into Omnimon Alter-S + 1 Digimon may attack
        # Complex DNA digivolve from field — tagged as descriptive (engine gap for end-of-turn DNA)
        effect_eot = ICardEffect()
        effect_eot.set_timing(EffectTiming.OnEndTurn)
        effect_eot.set_effect_name("EX9-013 End of turn DNA digivolve + attack")
        effect_eot.set_effect_description("[End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, 1 of your Digimon may attack.")
        effect_eot.is_optional = True

        def condition_eot(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect_eot.set_can_use_condition(condition_eot)

        def process_eot(ctx: Dict[str, Any]):
            # DNA digivolve is complex — descriptive-tagged for now
            pass

        effect_eot.set_on_process_callback(process_eot)
        effects.append(effect_eot)

        # Inherited: Ace Overflow <-4>
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("EX9-013 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True
        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
