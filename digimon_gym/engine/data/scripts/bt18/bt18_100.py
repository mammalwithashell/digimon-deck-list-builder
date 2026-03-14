from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_100(CardScript):
    """BT18-100 Gospel of the Fallen Angel | Option

    [Main] Your Digimon in the breeding area may digivolve into [Lucemon]
        from your trash without paying the cost. Then, place this card in
        the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate
        the effect below.)
        - By digivolving 1 of your Digimon with [Lucemon] in its name into
          a Digimon card with [Lucemon] in its name from your trash with the
          digivolution cost reduced by 3, trash 1 Option card in your opponent's
          battle area.
    [Security] Place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Main] Digimon in breeding digivolves into [Lucemon] from trash,
        # then place this card in battle area
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT18-100 Main: breeding digivolve into Lucemon")
        effect0.set_effect_description(
            "[Main] Your Digimon in the breeding area may digivolve into "
            "[Lucemon] from your trash without paying the cost. Then, place "
            "this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Check if there's a Digimon in breeding
            breeding_perm = player.breeding_area
            if breeding_perm:
                # Find Lucemon in trash
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', []) or []
                    return any(n == 'Lucemon' for n in names)

                lucemon_cards = [c for c in player.trash_cards if digi_filter(c)]
                if lucemon_cards:
                    chosen = lucemon_cards[0]
                    player.trash_cards.remove(chosen)
                    breeding_perm.card_sources.append(chosen)

            # Place this card in battle area (as a Delay card)
            # This is handled by the Delay system

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Delay flag
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT18-100 Delay: place in battle area")
        effect1.set_effect_description("[Main] <Delay>")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Delay activation: digivolve Lucemon into Lucemon from trash (cost -3),
        # then trash 1 opponent Option
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.DelaySkill)
        effect2.set_effect_name("BT18-100 Delay: digivolve Lucemon, trash opponent Option")
        effect2.set_effect_description(
            "[Delay] By digivolving 1 of your Digimon with [Lucemon] in its name "
            "into a Digimon card with [Lucemon] in its name from your trash with "
            "the digivolution cost reduced by 3, trash 1 Option card in your "
            "opponent's battle area."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Find a Digimon with [Lucemon] in its name on the field
            def perm_filter(p):
                return p.is_digimon and p.contains_card_name('Lucemon')

            def on_select_perm(target_perm):
                # Digivolve into Lucemon from trash with cost reduced by 3
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', []) or []
                    return any('Lucemon' in n for n in names)

                lucemon_trash = [c for c in player.trash_cards if digi_filter(c)]
                if lucemon_trash:
                    chosen = lucemon_trash[0]
                    player.trash_cards.remove(chosen)
                    target_perm.card_sources.append(chosen)

                    # Trash 1 Option card in opponent's battle area
                    enemy = player.enemy
                    if enemy:
                        opp_options = [
                            p for p in enemy.battle_area
                            if any(getattr(cs, 'is_option', False) for cs in p.card_sources)
                        ]
                        if opp_options:
                            target_opt = opp_options[0]
                            if target_opt in enemy.battle_area:
                                enemy.battle_area.remove(target_opt)
                                for cs in target_opt.card_sources:
                                    enemy.trash_cards.append(cs)

            has_lucemon = any(perm_filter(p) for p in player.battle_area)
            if has_lucemon:
                game.effect_select_own_permanent(
                    player, on_select_perm, filter_fn=perm_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Place this card in the battle area
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT18-100 Security: place in battle area")
        effect3.set_effect_description("[Security] Place this card in the battle area.")
        effect3.is_security_effect = True
        effect3._security_place_in_battle_area = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
