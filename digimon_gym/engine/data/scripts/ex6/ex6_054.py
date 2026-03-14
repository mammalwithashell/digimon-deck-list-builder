from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_054(CardScript):
    """EX6-054 Lucemon: Chaos Mode | Lv.5

    [On Play] [When Digivolving] Your opponent may delete 1 of their Digimon
        or Tamers. If this effect didn't delete, trash the top card of your
        opponent's security stack and Recovery +1 (Deck).
    [All Turns] When this Digimon would leave the battle area, by returning
        1 [Lucemon] from this Digimon's digivolution cards or from your trash
        to the bottom of the deck, you may play 1 [Lucemon: Satan Mode] or
        level 6 Digimon card with the [Seven Great Demon Lords] trait from
        your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digi from Lucemon for cost 6
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Lucemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Play] [When Digivolving] opponent may delete or trash security + recovery
        def _build_main_effect(is_on_play=False, is_when_digivolving=False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving
            effect.set_effect_name("EX6-054 Opponent delete or trash security + recovery")
            effect.set_effect_description(
                "Your opponent may delete 1 of their Digimon or Tamers. If this "
                "effect didn't delete, trash the top card of your opponent's "
                "security stack and Recovery +1 (Deck)."
            )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                enemy = player.enemy
                if not enemy:
                    return

                # Opponent may delete 1 of their Digimon or Tamers
                opp_targets = [p for p in enemy.battle_area if p.is_digimon or p.is_tamer]
                if opp_targets:
                    # Simplified: opponent auto-deletes
                    target = opp_targets[0]
                    if target in enemy.battle_area:
                        enemy.delete_permanent(target)
                else:
                    # Didn't delete -> trash opponent security + recovery
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
                    player.recovery(1)

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_main_effect(is_on_play=True))
        effects.append(_build_main_effect(is_when_digivolving=True))

        # [All Turns] When would leave, return 1 [Lucemon] to deck bottom,
        # play Satan Mode or Lv6 Seven Great Demon Lords from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX6-054 Leave: return Lucemon, play from trash")
        effect3.set_effect_description(
            "[All Turns] When this Digimon would leave the battle area, by "
            "returning 1 [Lucemon] from this Digimon's digivolution cards or "
            "from your trash to the bottom of the deck, you may play 1 "
            "[Lucemon: Satan Mode] or level 6 Digimon card with the "
            "[Seven Great Demon Lords] trait from your trash without paying the cost."
        )
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            perm = card.permanent_of_this_card()
            # Check if Lucemon exists in digi cards or trash
            has_lucemon_source = False
            if perm:
                has_lucemon_source = any(
                    any(n == 'Lucemon' for n in (getattr(cs, 'card_names', []) or []))
                    for cs in perm.card_sources[:-1]
                )
            if not has_lucemon_source:
                has_lucemon_source = any(
                    any(n == 'Lucemon' for n in (getattr(c, 'card_names', []) or []))
                    for c in player.trash_cards
                )
            return has_lucemon_source
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # Cost: return 1 [Lucemon] from digi cards or trash to deck bottom
            returned = False
            for cs in list(perm.card_sources[:-1]):
                names = getattr(cs, 'card_names', []) or []
                if any(n == 'Lucemon' for n in names):
                    perm.card_sources.remove(cs)
                    player.library_cards.append(cs)
                    returned = True
                    break
            if not returned:
                for c in list(player.trash_cards):
                    names = getattr(c, 'card_names', []) or []
                    if any(n == 'Lucemon' for n in names):
                        player.trash_cards.remove(c)
                        player.library_cards.append(c)
                        returned = True
                        break
            if not returned:
                return

            # Play 1 [Lucemon: Satan Mode] or Lv6 with Seven Great Demon Lords
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                if any('Lucemon: Satan Mode' in n or 'Lucemon Satan Mode' in n for n in names):
                    return True
                if getattr(c, 'level', None) == 6:
                    traits = getattr(c, 'card_traits', []) or []
                    if any('Seven Great Demon Lords' in t for t in traits):
                        return True
                return False

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
