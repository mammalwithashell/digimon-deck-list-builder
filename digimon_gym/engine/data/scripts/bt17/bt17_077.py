from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_077(CardScript):
    """BT17-077 Imperialdramon: Paladin Mode | Lv.7 White ACE

    [Hand] [Counter] <Blast Digivolve>
    [When Attacking] By returning 1 of your opponent's Digimon with no
        digivolution cards to the bottom of the deck, unsuspend this Digimon.
    [On Play] Trash all digivolution cards of all of your opponent's Digimon.
        Then, return all cards from your or your opponent's trash to the bottom
        of the deck. If this effect returned a white level 7 card, gain 3 memory.
    [When Digivolving] Same as On Play.
    Inherited: Ace Overflow <-5>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alternate digivolution from Lv.6 [Imperialdramon] for cost 5 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-077 Alternate digivolution requirement")
        effect0.set_effect_description(
            "Alternate digivolution: from Lv.6 [Imperialdramon] for cost 5"
        )
        effect0._alt_digi_cost = 5
        effect0._alt_digi_name = "Imperialdramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not permanent:
                return False
            top = permanent.top_card
            if not top:
                return False
            # Lv.6 Imperialdramon (any variant)
            if getattr(top, 'level', None) != 6:
                return False
            if not top.contains_card_name('Imperialdramon'):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Blast Digivolve ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT17-077 Blast Digivolve")
        effect1.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect1.is_counter_effect = True
        effect1._is_blast_digivolve = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [When Attacking] Return 1 opponent Digimon with 0 digi-cards -> unsuspend self ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name(
            "BT17-077 When Attacking: Bottom deck 0-digi-card Digimon, unsuspend self"
        )
        effect2.set_effect_description(
            "[When Attacking] By returning 1 of your opponent's Digimon with no "
            "digivolution cards to the bottom of the deck, unsuspend this Digimon."
        )
        effect2.is_on_attack = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be THIS Digimon attacking
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacker') or context.get('permanent')
            if perm and ctx_perm and ctx_perm is not perm:
                return False
            # Opponent must have a Digimon with no digivolution cards
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy
            if not enemy:
                return False
            return any(
                p.is_digimon and len(p.card_sources) <= 1
                for p in enemy.battle_area
            )

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Optionally return 1 opp Digimon with no digi-cards to deck bottom; if done, unsuspend self."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None

            def no_digi_filter(p):
                # DigivolutionCards.Count == 0 means only 1 card in stack
                return p.is_digimon and len(p.card_sources) <= 1

            def on_bottom_deck(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
                # Unsuspend this Digimon on success
                if perm:
                    perm.unsuspend()

            game.effect_select_opponent_permanent(
                player, on_bottom_deck,
                filter_fn=no_digi_filter,
                is_optional=True
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Shared process: On Play / When Digivolving effect ---
        def _trash_sources_and_return_trash(ctx: Dict[str, Any]):
            """
            Step 1: Trash all digivolution cards from all opponent Digimon.
            Step 2: Player chooses own or opponent trash; return ALL those cards to deck bottom.
            Step 3: If any returned card was white Lv.7, gain 3 memory.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: Trash all digivolution cards (card_sources[:-1]) from each opp Digimon
            for perm in list(enemy.battle_area):
                if not perm.is_digimon:
                    continue
                digi_card_count = max(len(perm.card_sources) - 1, 0)
                if digi_card_count > 0:
                    trashed = perm.trash_digivolution_cards(digi_card_count, from_top=True)
                    if trashed:
                        enemy.trash_cards.extend(trashed)

            # Step 2: Choose own or opponent trash to bottom deck
            # Branch 0 = own trash, Branch 1 = opponent trash
            def on_branch(choice: int):
                source_player = player if choice == 0 else enemy
                returned_cards = list(source_player.trash_cards)
                # Clear chosen trash and move all to that player's library bottom
                source_player.trash_cards.clear()
                for c in returned_cards:
                    source_player.library_cards.append(c)

                # Step 3: If any returned card is white Lv.7, gain 3 memory
                def _is_white_lv7(c) -> bool:
                    from digimon_gym.engine.data.enums import CardColor
                    colors = getattr(c, 'card_colors', []) or []
                    level = getattr(c, 'level', None)
                    return level == 7 and CardColor.White in colors

                if any(_is_white_lv7(c) for c in returned_cards):
                    player.add_memory(3)

            game.effect_choose_branch(
                player, 2,
                callback=on_branch,
                branch_labels=["Your Trash", "Opponent's Trash"]
            )

        # --- Effect 3: [On Play] Trash digi-sources, return trash, memory check ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT17-077 On Play: Trash digi-sources, return trash, memory +3")
        effect3.set_effect_description(
            "[On Play] Trash all digivolution cards of all of your opponent's Digimon. "
            "Then, return all cards from your or your opponent's trash to the bottom of "
            "the deck. If this effect returned a white level 7 card, gain 3 memory."
        )
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_trash_sources_and_return_trash)
        effects.append(effect3)

        # --- Effect 4: [When Digivolving] Same as On Play ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT17-077 When Digivolving: Trash digi-sources, return trash, memory +3")
        effect4.set_effect_description(
            "[When Digivolving] Trash all digivolution cards of all of your opponent's Digimon. "
            "Then, return all cards from your or your opponent's trash to the bottom of "
            "the deck. If this effect returned a white level 7 card, gain 3 memory."
        )
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_trash_sources_and_return_trash)
        effects.append(effect4)

        # --- Inherited: Ace Overflow <-5> ---
        # Engine handles ACE Overflow automatically via is_ace / ace_overflow_cost on the
        # card entity base. This inherited effect is descriptive — no process callback needed.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("BT17-077 Ace Overflow <-5>")
        effect_ace.set_effect_description("Ace Overflow <-5>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
