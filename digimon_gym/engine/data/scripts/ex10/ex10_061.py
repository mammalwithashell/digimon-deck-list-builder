from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _is_dark_masters_digimon(c) -> bool:
    """Check if a card is a Digimon with the [Dark Masters] trait."""
    if not getattr(c, 'is_digimon', False):
        return False
    traits = getattr(c, 'card_traits', []) or []
    return any('Dark Masters' in t for t in traits)


def _has_dark_masters_trait(perm) -> bool:
    """Check if a permanent has the [Dark Masters] trait."""
    top = perm.top_card if perm else None
    if not top:
        return False
    traits = getattr(top, 'card_traits', []) or []
    return any('Dark Masters' in t for t in traits)


def _count_face_up_dm_security(player, max_count=4):
    """Count distinct-named face-up Dark Masters Digimon in security."""
    seen_names = set()
    count = 0
    for c in player.security_cards:
        if not player.is_security_face_up(c):
            continue
        if not _is_dark_masters_digimon(c):
            continue
        names = getattr(c, 'card_names', []) or []
        name = names[0] if names else None
        if name and name not in seen_names:
            seen_names.add(name)
            count += 1
            if count >= max_count:
                break
    return count


def _get_face_up_dm_from_security(player, max_count=4):
    """Get up to max_count distinct-named face-up Dark Masters Digimon from security."""
    seen_names = set()
    result = []
    for c in player.security_cards:
        if not player.is_security_face_up(c):
            continue
        if not _is_dark_masters_digimon(c):
            continue
        names = getattr(c, 'card_names', []) or []
        name = names[0] if names else None
        if name and name not in seen_names:
            seen_names.add(name)
            result.append(c)
            if len(result) >= max_count:
                break
    return result


class EX10_061(CardScript):
    """EX10-061 Apocalymon | Lv.7

    Digivolve: Lv.6 w/[Dark Masters] trait for 5 cost

    When this card would be played, by placing 1 of each face-up [Dark Masters]
    trait card with different names from your security stack under this card,
    reduce the play cost by 4 for each card placed.

    [On Play] [When Digivolving] You may play 1 of each [Dark Masters] trait
    card with different names from this Digimon's digivolution cards without
    paying the costs. Then, all of your [Dark Masters] trait Digimon gain
    <Rush> for the turn. At turn end, delete the Digimon this effect played.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve requirement ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-061 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "Dark Masters"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: BeforePayCost - place face-up Dark Masters from security ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("EX10-061 Placing 1 [Dark Masters] to get Play Cost -4")
        effect1.set_effect_description(
            "When this card would be played, by placing 1 of each face-up "
            "[Dark Masters] trait card with different names from your security "
            "stack under this card, reduce the play cost by 4 for each card placed."
        )
        effect1.set_hash_string("PlayCost-_EX10-061")

        def _cost_reduction_fn(context):
            player = card.owner if card else None
            if not player:
                return 0
            return _count_face_up_dm_security(player) * 4

        effect1._cost_reduction_value_fn = _cost_reduction_fn

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            player = card.owner if card else None
            if not player:
                return False
            return _count_face_up_dm_security(player) > 0

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Place face-up Dark Masters cards from security under this card."""
            player = ctx.get('player')
            if not player:
                return
            to_place = _get_face_up_dm_from_security(player)
            for c in to_place:
                removed = player.remove_from_security(c)
                if removed:
                    if not hasattr(card, '_pending_digi_sources'):
                        card._pending_digi_sources = []
                    card._pending_digi_sources.append(removed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Shared On Play / When Digivolving logic ---
        def _consume_pending_digi_sources(perm):
            """Move any _pending_digi_sources from BeforePayCost into the permanent stack."""
            pending = getattr(card, '_pending_digi_sources', None)
            if pending and perm:
                for cs in pending:
                    perm.card_sources.insert(0, cs)  # insert at bottom
                card._pending_digi_sources = []

        def _find_playable_dm(perm):
            """Find distinct-named Dark Masters Digimon in digivolution cards."""
            seen_names = set()
            playable = []
            for cs in list(perm.card_sources):
                if cs is perm.top_card:
                    continue
                if not _is_dark_masters_digimon(cs):
                    continue
                names = getattr(cs, 'card_names', []) or []
                name = names[0] if names else None
                if name and name not in seen_names:
                    seen_names.add(name)
                    playable.append(cs)
            return playable

        def _shared_process(ctx: Dict[str, Any]):
            """Play Dark Masters from digi cards, grant Rush, delete played at turn end.

            C# ref: canNoSelect = true (player may skip entirely),
            canEndNotMax = false (if selecting, must select all distinct names).
            Implemented as a "you may" branch choice: play all or none.
            """
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            # First, consume any pending digi sources from BeforePayCost
            _consume_pending_digi_sources(perm)

            playable = _find_playable_dm(perm)

            if not playable:
                # No Dark Masters in digi cards to play — still grant Rush
                _grant_rush_to_dm(player, game)
                return

            # C# behavior: canNoSelect=true + canEndNotMax=false means
            # "you may play all distinct DMs, or none". Offer as branch choice.
            def on_branch(branch_index: int):
                if branch_index == 0:
                    # Play all distinct Dark Masters from digi cards
                    _play_all_dm_and_grant_rush(player, perm, game, playable)
                else:
                    # Skip playing — still grant Rush
                    _grant_rush_to_dm(player, game)

            game.effect_choose_branch(
                player, 2, on_branch,
                prompt="Play [Dark Masters] from digivolution cards?",
                branch_labels=["Play all Dark Masters", "Skip"],
            )

        def _play_all_dm_and_grant_rush(player, perm, game, playable):
            """Play all distinct DM cards from digi stack, grant Rush, register EOT delete."""
            played_perms = []
            for cs in playable:
                if cs in perm.card_sources:
                    perm.card_sources.remove(cs)
                played_perm = player.play_card_from_source(cs, pay_cost=False)
                if played_perm:
                    played_perms.append(played_perm)
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {
                            'played_card': cs,
                            'played_permanent': played_perm,
                            'event_permanent': played_perm,
                            'event_player': player,
                            'is_effect_play': True,
                        },
                    )
                    game._fire_play_observers(played_perm, player)

            # Grant <Rush> to all Dark Masters trait Digimon for the turn
            _grant_rush_to_dm(player, game)

            # Register end-of-turn deletion for the Digimon this effect played
            if played_perms and game:
                _register_eot_deletion(player, perm, played_perms, game)

        def _grant_rush_to_dm(player, game):
            """Grant Rush to all of owner's Dark Masters trait Digimon for the turn."""
            if not player:
                return
            current_turn = game.turn_count if hasattr(game, 'turn_count') else -1
            for p in player.battle_area:
                if p.is_digimon and _has_dark_masters_trait(p):
                    p.grant_keyword('_is_rush', current_turn)

        def _register_eot_deletion(player, source_perm, played_perms, game):
            """Register an end-of-turn effect to delete Digimon played by this effect."""
            eot_effect = ICardEffect()
            eot_effect.set_timing(EffectTiming.OnEndTurn)
            eot_effect.set_effect_name("EX10-061 Delete the played Digimon")
            eot_effect.set_effect_description(
                "[End of Your Turn] Delete the Digimon that were played by Apocalymon."
            )

            def eot_condition(context: Dict[str, Any]) -> bool:
                # Only fire on owner's end of turn
                if not (card and card.owner and card.owner.is_my_turn):
                    return False
                # Only fire if there are still played permanents on field
                return any(pp in player.battle_area for pp in played_perms)

            eot_effect.set_can_use_condition(eot_condition)

            def eot_process(ctx: Dict[str, Any]):
                for pp in list(played_perms):
                    if pp in player.battle_area:
                        player.delete_permanent(pp)

            eot_effect.set_on_process_callback(eot_process)

            # Register as a player permanent effect so it fires at end of turn
            if hasattr(card, '_granted_eot_effects'):
                card._granted_eot_effects.append(eot_effect)
            else:
                card._granted_eot_effects = [eot_effect]

            # Also register on the source permanent's granted effects
            if source_perm:
                current_turn = game.turn_count if hasattr(game, 'turn_count') else -1
                source_perm._granted_effects.append((eot_effect, current_turn))

        # --- Effect 2: [On Play] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-061 Play 1 [Dark Masters] from sources, give <Rush>, Delete them at end of turn")
        effect2.set_effect_description(
            "[On Play] You may play 1 of each [Dark Masters] trait card with "
            "different names from this Digimon's digivolution cards without "
            "paying the costs. Then, all of your [Dark Masters] trait Digimon "
            "gain <Rush> for the turn. At turn end, delete the Digimon this "
            "effect played."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_process)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX10-061 Play 1 [Dark Masters] from sources, give <Rush>, Delete them at end of turn")
        effect3.set_effect_description(
            "[When Digivolving] You may play 1 of each [Dark Masters] trait card "
            "with different names from this Digimon's digivolution cards without "
            "paying the costs. Then, all of your [Dark Masters] trait Digimon "
            "gain <Rush> for the turn. At turn end, delete the Digimon this "
            "effect played."
        )
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_shared_process)
        effects.append(effect3)

        return effects
