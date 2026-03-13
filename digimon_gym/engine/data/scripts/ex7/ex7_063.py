from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_063(CardScript):
    """EX7-063 Arisa Kinosaki | Tamer, Yellow, LIBERATOR, Cost 3

    [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
    [All Turns] When one of your Tokens or Digimon with the [Puppet] trait is
        deleted, by suspending this Tamer, you may play 1 level 3 Digimon card
        with the [Puppet] trait from your hand without paying the cost.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Main Phase] +1 memory if opponent has Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX7-063 Gain 1 memory if opponent has Digimon")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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
            has_digimon = any(p.is_digimon for p in enemy.battle_area)
            if has_digimon:
                player.add_memory(1)
                game.logger.log(
                    f"[EX7-063] {player.player_name} gained 1 memory "
                    f"(opponent has Digimon)."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] When Token or Puppet Digimon deleted,
        #     suspend this Tamer to play Lv.3 Puppet from hand free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX7-063 Suspend to play Lv.3 Puppet on Token/Puppet deletion")
        effect1.set_effect_description(
            "[All Turns] When one of your Tokens or Digimon with the [Puppet] "
            "trait is deleted, by suspending this Tamer, you may play 1 level 3 "
            "Digimon card with the [Puppet] trait from your hand without paying "
            "the cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            tamer_perm = card.permanent_of_this_card()
            # Tamer must not already be suspended
            if tamer_perm and tamer_perm.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, then play 1 Lv.3 Puppet Digimon from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return

            # Pay cost: suspend this tamer
            tamer_perm.suspend()

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level != 3:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX7-063 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
