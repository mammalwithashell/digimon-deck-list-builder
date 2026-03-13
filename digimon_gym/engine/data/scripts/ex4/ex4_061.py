from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_061(CardScript):
    """EX4-061 Tai Kamiya & Matt Ishida | Tamer | Blue | Cost 4

    [Your Turn] When you play [Gabumon] or [Agumon], by suspending this
        Tamer, gain 1 memory.

    [Your Turn][Once Per Turn] When one of your Digimon digivolves, if you
        have 1 or fewer Digimon, you may play 1 [Gabumon] if that Digimon has
        [Greymon] in its name or 1 [Agumon] if it has [Garurumon] in its name
        from your hand or trash without paying the cost.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Your Turn] When you play Gabumon or Agumon,
        #     suspend this Tamer to gain 1 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX4-061 Suspend to gain memory on Agumon/Gabumon play")
        effect0.set_effect_description(
            "[Your Turn] When you play [Gabumon] or [Agumon], by suspending "
            "this Tamer, gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm.is_suspended:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if not player.is_my_turn:
                return False
            # Trigger: a permanent with name Agumon or Gabumon was played
            played_perm = context.get('permanent')
            if not played_perm:
                return False
            if played_perm.owner != player:
                return False
            if not played_perm.is_digimon:
                return False
            top = played_perm.top_card
            if not top:
                return False
            names = getattr(top, 'card_names', []) or []
            if 'Agumon' in names or 'Gabumon' in names:
                return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            perm.suspend()
            player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn][Once Per Turn] When one of your Digimon
        #     digivolves, if you have 1 or fewer Digimon, play Gabumon/Agumon
        #     from hand or trash free ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX4-061 On digivolve: play Gabumon/Agumon from hand or trash")
        effect1.set_effect_description(
            "[Your Turn][Once Per Turn] When one of your Digimon digivolves, "
            "if you have 1 or fewer Digimon, you may play 1 [Gabumon] if that "
            "Digimon has [Greymon] in its name or 1 [Agumon] if it has "
            "[Garurumon] in its name from your hand or trash without paying "
            "the cost."
        )
        effect1.is_when_digivolving = True
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Play_EX4_061")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            if not player.is_my_turn:
                return False
            # Trigger: one of our Digimon digivolved
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != player:
                return False
            if not trigger_perm.is_digimon:
                return False
            # Must have 1 or fewer Digimon
            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if len(own_digimon) > 1:
                return False
            # The digivolved Digimon must have Greymon or Garurumon in its name
            top = trigger_perm.top_card
            if not top:
                return False
            has_greymon = trigger_perm.contains_card_name('Greymon')
            has_garurumon = trigger_perm.contains_card_name('Garurumon')
            if not (has_greymon or has_garurumon):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if len(own_digimon) != 1:
                return
            digivolved_perm = own_digimon[0]

            has_greymon = digivolved_perm.contains_card_name('Greymon')
            has_garurumon = digivolved_perm.contains_card_name('Garurumon')

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                if has_greymon and 'Gabumon' in names:
                    return True
                if has_garurumon and 'Agumon' in names:
                    return True
                return False

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter,
                free=True, is_optional=True,
                prompt="Select 1 [Gabumon] or [Agumon] to play from hand or trash."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX4-061 Security: Play this card free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
