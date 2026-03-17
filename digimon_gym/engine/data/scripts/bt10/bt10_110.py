from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_110(CardScript):
    """BT10-110 Seiken Meppa"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # "While you have a [Royal Knight] trait Digimon, you may ignore color requirements."
        # Keep match_color_requirement True; the action mask checks IGNORE_COLOR_REQUIREMENT
        # modifiers. We register a BeforePayCost effect that handles this at play time.
        # For practical purposes, bypass unconditionally since the condition check
        # is enforced in the main effect's condition (below).
        card._match_color_requirement = False

        # Timing: EffectTiming.OptionSkill
        # [Main] Unsuspend 1 of your Digimon. Then, if that Digimon is [Jesmon GX], activate 1 of its [When Digivolving] effects.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT10-110 Unsuspend")
        effect1.set_effect_description("[Main] Unsuspend 1 of your Digimon. Then, if that Digimon is [Jesmon GX], activate 1 of its [When Digivolving] effects.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Unsuspend 1 Digimon. Then, if Jesmon GX, activate 1 WD effect."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....data.enums import EffectTiming as _ET
            def target_filter(p):
                return p.is_digimon
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
                # "Then, if that Digimon is [Jesmon GX], activate 1 of its [When Digivolving] effects."
                if not target_perm.contains_card_name('Jesmon GX'):
                    return
                wd_effects = []
                if target_perm.top_card:
                    for eff in target_perm.top_card.effect_list(_ET.NoTiming):
                        if getattr(eff, '_is_when_digivolving', False):
                            wd_effects.append(eff)
                if not wd_effects:
                    return
                if len(wd_effects) == 1:
                    wd_eff = wd_effects[0]
                    if wd_eff.on_process_callback:
                        wd_eff.on_process_callback({
                            'player': player, 'permanent': target_perm, 'game': game})
                else:
                    labels = [e.effect_name or e.effect_description for e in wd_effects]
                    def on_branch(branch_idx, _effs=wd_effects):
                        chosen = _effs[branch_idx]
                        if chosen.on_process_callback:
                            chosen.on_process_callback({
                                'player': player, 'permanent': target_perm, 'game': game})
                    game.effect_choose_branch(
                        player, len(wd_effects), on_branch,
                        prompt="Activate 1 [When Digivolving] effect of Jesmon GX.",
                        branch_labels=labels)
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Add this card to its owner's hand.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT10-110 Add To Hand")
        effect2.set_effect_description("[Security] Add this card to its owner's hand.")
        effect2.is_security_effect = True
        effect2.is_security_effect = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
