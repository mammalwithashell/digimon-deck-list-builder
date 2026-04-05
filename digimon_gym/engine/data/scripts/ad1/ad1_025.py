from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_025(CardScript):
    """AD1-025 Omnimon | Lv.7 Red/Yellow

    <Raid>
    <Blocker>
    <Partition ([WarGreymon] & [MetalGarurumon])>
    [On Play] [When Digivolving] Return all of your opponent's Digimon with as
        many or fewer digivolution cards as this Digimon to the bottom of the
        deck. Then, delete 1 of your opponent's Digimon.
    [All Turns] [Once Per Turn] When any of your opponent's Digimon leave the
        battle area, trash 1 of their Option cards in the battle area and trash
        their top security card.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Raid ─────────────────────────────────────────────────────────
        effect_raid = ICardEffect()
        effect_raid.set_effect_name("AD1-025 Raid")
        effect_raid.set_effect_description("Raid")
        effect_raid.is_on_attack = True
        effect_raid._is_raid = True

        def condition_raid(context: Dict[str, Any]) -> bool:
            return True
        effect_raid.set_can_use_condition(condition_raid)
        effects.append(effect_raid)

        # ── Blocker ──────────────────────────────────────────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("AD1-025 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # ── Partition ([WarGreymon] & [MetalGarurumon]) ──────────────────
        # Metadata flag for engine
        effect_partition_flag = ICardEffect()
        effect_partition_flag.set_effect_name("AD1-025 Partition")
        effect_partition_flag.set_effect_description(
            "Partition ([WarGreymon] & [MetalGarurumon])")
        effect_partition_flag._is_partition = True

        def condition_partition_flag(context: Dict[str, Any]) -> bool:
            return True
        effect_partition_flag.set_can_use_condition(condition_partition_flag)
        effects.append(effect_partition_flag)

        # Partition WhenRemoveField implementation:
        # When this Digimon with each of the specified digivolution cards would
        # leave the battle area other than by your own effects or by battle,
        # you may play 1 each of the specified cards without paying the costs.
        #
        # By the time WhenRemoveField fires, the permanent is already removed
        # and its card_sources are in the owner's trash. The engine scans trash
        # cards for matching effects, so this effect will be found there.
        effect_partition = ICardEffect()
        effect_partition.set_timing(EffectTiming.WhenRemoveField)
        effect_partition.set_effect_name("AD1-025 Partition Play")
        effect_partition.set_effect_description(
            "Partition: play 1 [WarGreymon] and 1 [MetalGarurumon] from trash")
        effect_partition.is_optional = True
        effect_partition._is_partition = True

        def condition_partition(context: Dict[str, Any]) -> bool:
            # By WhenRemoveField, the permanent is already removed and card_sources
            # are in trash. This effect fires from the trash zone scan, so
            # context['permanent'] is None and context['card'] is this card_source.
            # Do NOT check card.permanent_of_this_card() or event_permanent here.

            # Guard: only fire from WhenRemoveField timing, NOT from
            # _execute_deletion_effects_inner (which also scans card_sources
            # but sets 'deleted_permanent' in context instead of 'removal_cause')
            if 'removal_cause' not in context:
                return False

            owner = card.owner if card else None
            if not owner:
                return False

            # Verify this card is in trash (it should be since the effect fires
            # from trash zone scanning)
            if card not in owner.trash_cards:
                return False

            # Check removal cause: NOT by own effects and NOT by battle
            is_own = context.get('is_own_effect', False)
            removal_cause = context.get('removal_cause', '')
            if is_own or removal_cause == 'battle':
                return False

            # Check trash for both WarGreymon and MetalGarurumon
            # (the evo cards were just moved to trash)
            has_wargreymon = any(
                c.contains_card_name('WarGreymon') for c in owner.trash_cards
                if c is not card  # exclude self (Omnimon)
            )
            has_metalgarurumon = any(
                c.contains_card_name('MetalGarurumon') for c in owner.trash_cards
                if c is not card  # exclude self (Omnimon)
            )
            return has_wargreymon and has_metalgarurumon

        effect_partition.set_can_use_condition(condition_partition)

        def process_partition(ctx: Dict[str, Any]):
            """Play 1 [WarGreymon] and 1 [MetalGarurumon] from trash without paying costs."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Play 1 WarGreymon from trash
            for c in list(player.trash_cards):
                if c.contains_card_name('WarGreymon'):
                    player.trash_cards.remove(c)
                    played = player.play_card_from_source(c, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": c, "played_permanent": played,
                             "event_player": player})
                    break

            # Play 1 MetalGarurumon from trash
            for c in list(player.trash_cards):
                if c.contains_card_name('MetalGarurumon'):
                    player.trash_cards.remove(c)
                    played = player.play_card_from_source(c, pay_cost=False)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": c, "played_permanent": played,
                             "event_player": player})
                    break

        effect_partition.set_on_process_callback(process_partition)
        effects.append(effect_partition)

        # ── [On Play] Bottom-deck + delete ───────────────────────────────
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("AD1-025 Bottom-deck opponent Digimon, then delete 1")
        effect_op.set_effect_description(
            "[On Play] Return all of your opponent's Digimon with as many or "
            "fewer digivolution cards as this Digimon to the bottom of the deck. "
            "Then, delete 1 of your opponent's Digimon.")
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)

        def process_op(ctx: Dict[str, Any]):
            """Bottom-deck all opponent Digimon with <= evo cards, then delete 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Our evo card count = len(card_sources) - 1 (exclude top card)
            our_evo_count = len(perm.card_sources) - 1

            # Return all opponent Digimon with <= evo cards to deck bottom
            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                opp_evo = len(opp_perm.card_sources) - 1
                if opp_evo <= our_evo_count:
                    enemy.return_permanent_to_deck_bottom(opp_perm)

            # Then, delete 1 of your opponent's Digimon (mandatory)
            def delete_filter(p):
                return p.is_digimon

            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm, is_opponent_effect=True)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False)

        effect_op.set_on_process_callback(process_op)
        effects.append(effect_op)

        # ── [When Digivolving] Bottom-deck + delete (same effect) ────────
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("AD1-025 Bottom-deck opponent Digimon, then delete 1")
        effect_wd.set_effect_description(
            "[When Digivolving] Return all of your opponent's Digimon with as "
            "many or fewer digivolution cards as this Digimon to the bottom of "
            "the deck. Then, delete 1 of your opponent's Digimon.")
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)

        def process_wd(ctx: Dict[str, Any]):
            """Bottom-deck all opponent Digimon with <= evo cards, then delete 1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            our_evo_count = len(perm.card_sources) - 1

            for opp_perm in list(enemy.battle_area):
                if not opp_perm.is_digimon:
                    continue
                opp_evo = len(opp_perm.card_sources) - 1
                if opp_evo <= our_evo_count:
                    enemy.return_permanent_to_deck_bottom(opp_perm)

            def delete_filter(p):
                return p.is_digimon

            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm, is_opponent_effect=True)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False)

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # ── [All Turns] [Once Per Turn] Opponent Digimon leaves ──────────
        # When any of your opponent's Digimon leave the battle area,
        # trash 1 of their Option cards in the battle area and trash their
        # top security card.
        effect_at = ICardEffect()
        effect_at.set_timing(EffectTiming.OnRemovedField)
        effect_at.set_effect_name("AD1-025 Trash option + security on opponent leave")
        effect_at.set_effect_description(
            "[All Turns] [Once Per Turn] When any of your opponent's Digimon "
            "leave the battle area, trash 1 of their Option cards in the battle "
            "area and trash their top security card.")
        effect_at.set_max_count_per_turn(1)
        effect_at.set_hash_string("AD1-025_AT")

        def condition_at(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False

            # Only trigger when OPPONENT's Digimon leaves
            owner = card.owner if card else None
            if not owner:
                return False
            enemy = owner.enemy if owner else None
            event_player = context.get('event_player')
            # The event_player is the player whose permanent was removed
            if event_player is None or event_player is not enemy:
                return False

            # Check it was a Digimon that left
            event_perm = context.get('event_permanent')
            if not event_perm or not getattr(event_perm, 'is_digimon', False):
                return False

            return True

        effect_at.set_can_use_condition(condition_at)

        def process_at(ctx: Dict[str, Any]):
            """Trash 1 opponent Option in battle area, then trash top security."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Trash 1 of their Option cards in the battle area
            option_perms = [p for p in enemy.battle_area if p.is_option]
            if option_perms:
                def option_filter(p):
                    return p.is_option

                def on_trash_option(target_perm):
                    if target_perm in enemy.battle_area:
                        enemy.battle_area.remove(target_perm)
                        for cs in target_perm.card_sources:
                            enemy.trash_cards.append(cs)

                game.effect_select_opponent_permanent(
                    player, on_trash_option,
                    filter_fn=option_filter,
                    is_optional=False)

            # Trash their top security card
            if enemy.security_cards:
                top_sec = enemy.security_cards[-1]
                enemy.trash_security_card(top_sec)

        effect_at.set_on_process_callback(process_at)
        effects.append(effect_at)

        return effects
