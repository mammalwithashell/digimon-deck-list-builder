from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_044(CardScript):
    """EX11-044 Pyramidimon | Lv.6

    Reboot
    Fragment (3)
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
      By trashing any 3 [Mineral] or [Rock] trait cards from your
      Digimon's digivolution cards, delete 1 of your opponent's
      highest play cost Digimon or Tamers.
    [All Turns] [Once Per Turn] When effects trash any of this Digimon's
      digivolution cards, you may place 3 [Mineral] or [Rock] trait cards
      from your trash as this Digimon's bottom digivolution cards.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Reboot
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-044 Reboot")
        effect0.set_effect_description("Reboot")
        effect0._is_reboot = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Fragment (3)
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-044 Fragment")
        effect1.set_effect_description("Fragment")
        effect1._is_fragment = True
        effect1._fragment_count = 3

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Helper: check if player has 3+ Mineral/Rock divi-cards across all Digimon
        def _has_3_mineral_rock_sources(player) -> bool:
            if not player:
                return False
            count = 0
            for perm in player.battle_area:
                for c in perm.digivolution_cards:
                    if c is perm.top_card:
                        continue
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        count += 1
                        if count >= 3:
                            return True
            return False

        def _trash_3_and_delete(ctx: Dict[str, Any]):
            """By trashing 3 Mineral/Rock sources, delete opponent's highest cost Digimon/Tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Trash 3 Mineral/Rock digivolution cards from any of player's Digimon
            trashed_count = 0
            for field_perm in player.battle_area:
                if trashed_count >= 3:
                    break
                for c in list(field_perm.digivolution_cards):
                    if trashed_count >= 3:
                        break
                    if c is field_perm.top_card:
                        continue
                    traits = getattr(c, 'card_traits', []) or []
                    if 'Mineral' in traits or 'Rock' in traits:
                        field_perm.card_sources.remove(c)
                        player.trash_cards.append(c)
                        trashed_count += 1

            if trashed_count < 3:
                return

            # Delete 1 of opponent's highest play cost Digimon or Tamers
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon or p.is_tamer]
            if not targets:
                return

            def get_play_cost(p):
                if p.top_card and p.top_card.c_entity_base:
                    return p.top_card.c_entity_base.play_cost or 0
                return 0

            max_cost = max(get_play_cost(p) for p in targets)

            def target_filter(p):
                return (p.is_digimon or p.is_tamer) and get_play_cost(p) == max_cost

            def on_delete(target_perm):
                enemy2 = player.enemy if player else None
                if enemy2:
                    enemy2.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False,
                prompt="Delete 1 of your opponent's highest play cost Digimon or Tamers.")

        # [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
        # Build 3 effects sharing the same once-per-turn hash
        def _build_delete_effect(is_on_play=False, is_when_digivolving=False, is_on_attack=False):
            effect = ICardEffect()
            if is_on_attack:
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
            else:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_on_play = is_on_play
                effect.is_when_digivolving = is_when_digivolving
            effect.set_effect_name("EX11-044 Trash 3 sources, delete highest cost")
            effect.set_effect_description(
                "[Once Per Turn] By trashing 3 [Mineral] or [Rock] trait cards "
                "from your Digimon's digivolution cards, delete 1 of your "
                "opponent's highest play cost Digimon or Tamers."
            )
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string("DELETE_EX11_044")

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                return _has_3_mineral_rock_sources(owner)

            effect.set_can_use_condition(condition)
            effect.set_on_process_callback(_trash_3_and_delete)
            return effect

        effects.append(_build_delete_effect(is_on_play=True))
        effects.append(_build_delete_effect(is_when_digivolving=True))
        effects.append(_build_delete_effect(is_on_attack=True))

        # [All Turns] [Once Per Turn] When effects trash any of this Digimon's
        # digivolution cards, you may place 3 [Mineral] or [Rock] trait cards
        # from your trash as this Digimon's bottom digivolution cards.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect5.set_effect_name("EX11-044 Place 3 Mineral/Rock from trash under this Digimon")
        effect5.set_effect_description(
            "[All Turns] [Once Per Turn] When effects trash any of this "
            "Digimon's digivolution cards, you may place 3 [Mineral] or "
            "[Rock] trait cards from your trash as this Digimon's bottom "
            "digivolution cards."
        )
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("EX11_044_AT")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must have at least 3 Mineral/Rock in trash
            count = sum(
                1 for c in owner.trash_cards
                if 'Mineral' in (getattr(c, 'card_traits', []) or [])
                or 'Rock' in (getattr(c, 'card_traits', []) or [])
            )
            return count >= 3

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Place 3 Mineral/Rock cards from trash as bottom sources."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if not (player and perm):
                return
            placed = 0
            for source_card in list(player.trash_cards):
                if placed >= 3:
                    break
                traits = getattr(source_card, 'card_traits', []) or []
                if 'Mineral' in traits or 'Rock' in traits:
                    player.trash_cards.remove(source_card)
                    perm.add_card_source_bottom(source_card)
                    placed += 1

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
