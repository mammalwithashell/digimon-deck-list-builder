from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_094(CardScript):
    """BT19-094 Seventh Divine Cruz | Option

    [Trash] [Your Turn] When any of your Digimon digivolve into [Lucemon (X
        Antibody)], by returning this card to the bottom of the deck, your
        opponent may trash their top security card. If this effect didn't
        trash, Recovery +1 (Deck).
    [Main] Delete your opponent's Digimon until they have as many as the
        number of your security cards. If this effect deleted, Recovery +1.
    [Security] You may play 1 [Lucemon] from your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Trash] [Your Turn] When digivolve into Lucemon (X Antibody),
        # return this card to deck bottom, opponent may trash security or recovery +1
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT19-094 Trash trigger: opponent security or recovery")
        effect0.set_effect_description(
            "[Trash] [Your Turn] When any of your Digimon digivolve into "
            "[Lucemon (X Antibody)], by returning this card to the bottom of "
            "the deck, your opponent may trash their top security card. If "
            "this effect didn't trash, Recovery +1 (Deck)."
        )
        effect0.is_optional = True
        effect0._is_trash_trigger = True

        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must be in trash
            player = card.owner if card else None
            if not player or card not in player.trash_cards:
                return False
            # Check if a Digimon digivolved into Lucemon (X Antibody)
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if not digivolved.contains_card_name('Lucemon (X Antibody)'):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this card to bottom of deck
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.library_cards.append(card)
            else:
                return

            # Opponent may trash their top security
            enemy = player.enemy
            if enemy and enemy.security_cards:
                # Simplified: opponent auto-trashes
                trashed = enemy.security_cards.pop(0)
                enemy.trash_cards.append(trashed)
            else:
                # Didn't trash -> Recovery +1
                player.recovery(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Main] Delete opponent Digimon until count <= your security count.
        # If this effect deleted, Recovery +1.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT19-094 Main: delete until equal to security count")
        effect1.set_effect_description(
            "[Main] Delete your opponent's Digimon until they have as many as "
            "the number of your security cards. If this effect deleted, Recovery +1."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            my_security_count = len(player.security_cards)
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            deleted_any = False

            while len([p for p in enemy.battle_area if p.is_digimon]) > my_security_count:
                opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not opp_digimon:
                    break
                # Delete highest level first (auto-select)
                target = max(opp_digimon, key=lambda p: (p.level or 0))
                if target in enemy.battle_area:
                    enemy.delete_permanent(target)
                    deleted_any = True

            if deleted_any:
                player.recovery(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play 1 [Lucemon] from trash without paying the cost
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT19-094 Security: play Lucemon from trash")
        effect2.set_effect_description(
            "[Security] You may play 1 [Lucemon] from your trash without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any(n == 'Lucemon' for n in names)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
