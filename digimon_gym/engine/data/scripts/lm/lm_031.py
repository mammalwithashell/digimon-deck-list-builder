from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class LM_031(CardScript):
    """LM-031 Black Scramble | Option (Black, Cost 2)

    [Main] 1 of your black Digimon may digivolve into a black Digimon card in
        the hand with the digivolution cost reduced by 3. Then, place this
        card in the battle area.
    [Start of Your Turn] If your opponent has a Digimon, <Delay>
        (By trashing this card after the placing turn, activate the effect below.)
        - Return 1 black Digimon card from your trash to the top of the deck.
          Then, if you don't have a Digimon, you may play 1 black Digimon card
          with 2000 DP or less from your trash without paying the cost.
    [Security] You may play 1 black Digimon card with 2000 DP or less from
        your trash without paying the cost. Then, add this card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Black Digimon digivolves with cost -3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("LM-031 Digivolve 1 black Digimon with cost reduced by 3")
        effect0.set_effect_description(
            "[Main] 1 of your black Digimon may digivolve into a black "
            "Digimon card in the hand with the digivolution cost reduced by 3. "
            "Then, place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Filter for black Digimon cards in hand
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                return any(col.name == 'Black' for col in colors)

            # Select one of your black Digimon on the field as base
            def own_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top is None:
                    return False
                colors = getattr(top, 'card_colors', []) or []
                return any(col.name == 'Black' for col in colors)

            def on_select(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter,
                    cost_reduction=3, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        # Marks this option card to remain in the battle area after use.
        effect1 = ICardEffect()
        effect1.set_effect_name("LM-031 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Start of Your Turn] If opponent has Digimon, activate Delay ---
        # Timing: OnStartTurn. Condition requires: card on field + your turn + opponent has Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartTurn)
        effect2.set_effect_name(
            "LM-031 Return 1 black Digimon from trash to deck top, then play from trash"
        )
        effect2.set_effect_description(
            "[Start of Your Turn] If your opponent has a Digimon, <Delay> "
            "(By trashing this card after the placing turn, activate the effect below.)\n"
            "- Return 1 black Digimon card from your trash to the top of the deck. "
            "Then, if you don't have a Digimon, you may play 1 black Digimon card "
            "with 2000 DP or less from your trash without paying the cost."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Card must be in the battle area
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # Condition: opponent must have at least 1 Digimon
            enemy = owner.enemy if owner else None
            if not enemy:
                return False
            return any(p.is_digimon for p in enemy.battle_area)
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: trash this delay card from the battle area.
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is not None:
                player.delete_permanent(my_perm)

            # Return 1 black Digimon from trash to the top of the deck.
            for i, c in enumerate(player.trash_cards):
                if getattr(c, 'is_digimon', False):
                    colors = getattr(c, 'card_colors', []) or []
                    if any(col.name == 'Black' for col in colors):
                        moved = player.trash_cards.pop(i)
                        player.library_cards.insert(0, moved)
                        break

            # Then, if you don't have a Digimon, play 1 black Digimon <= 2000 DP from trash
            has_digimon = any(p.is_digimon for p in (player.battle_area or []))
            if not has_digimon:
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    colors = getattr(c, 'card_colors', []) or []
                    if not any(col.name == 'Black' for col in colors):
                        return False
                    dp = getattr(c, 'base_dp', None)
                    if dp is not None and dp > 2000:
                        return False
                    return True
                game.effect_play_from_zone(
                    player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play black Digimon DP <= 2000 from trash, add to hand ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("LM-031 Security Effect")
        effect3.set_effect_description(
            "[Security] You may play 1 black Digimon card with 2000 DP or "
            "less from your trash without paying the cost. Then, add this "
            "card to the hand."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Play 1 black Digimon with 2000 DP or less from trash (optional)
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                colors = getattr(c, 'card_colors', []) or []
                if not any(col.name == 'Black' for col in colors):
                    return False
                dp = getattr(c, 'base_dp', None)
                return dp is None or dp <= 2000

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

            # Then, add this card to the hand.
            # The engine trashes the security card before the security effect fires;
            # pop the last trashed card (this card) back to hand.
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
