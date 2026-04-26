from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_001(CardScript):
    """AD1-001 Greymon | Lv.4 Red Digimon | Dinosaur/ADVENTURE | DP 5000

    [Digivolve] Lv.3 w/[Omnimon] in text or w/[ADVENTURE] trait: Cost 2

    [On Play] [When Digivolving] You may return 1 card with [Greymon],
    [Garurumon] or [Omnimon] in its name from your trash to the hand.

    [All Turns] When your Digimon or Tamers are played or digivolve, if
    any of them have [Garurumon] or [Tai Kamiya] in their names, this
    Digimon may digivolve into a Digimon card with [Greymon] in its name
    in the hand without paying the cost.

    Inherited: Raid
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-Digi 1: Lv.3 w/[ADVENTURE] trait → Cost 2 ──────────────
        alt1 = ICardEffect()
        alt1.set_effect_name("AD1-001 Alt-digi: ADVENTURE trait")
        alt1.set_effect_description("[Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2")
        alt1._alt_digi_cost = 2
        alt1._alt_digi_level = 3
        alt1._alt_digi_trait = "ADVENTURE"

        def alt1_cond(context: Dict[str, Any]) -> bool:
            return True
        alt1.set_can_use_condition(alt1_cond)
        effects.append(alt1)

        # ── Alt-Digi 2: Lv.3 w/[Omnimon] in text → Cost 2 ─────────────
        # C# uses HasText("Omnimon") which checks combined card text
        alt2 = ICardEffect()
        alt2.set_effect_name("AD1-001 Alt-digi: Omnimon in text")
        alt2.set_effect_description("[Digivolve] Lv.3 w/[Omnimon] in text: Cost 2")
        alt2._alt_digi_cost = 2
        alt2._alt_digi_level = 3
        alt2._alt_digi_has_text = "Omnimon"

        def alt2_cond(context: Dict[str, Any]) -> bool:
            return True
        alt2.set_can_use_condition(alt2_cond)
        effects.append(alt2)

        # ── [On Play] trash recovery ────────────────────────────────────
        def _trash_filter(c):
            return (c.contains_card_name('Greymon')
                    or c.contains_card_name('Garurumon')
                    or c.contains_card_name('Omnimon'))

        op_effect = ICardEffect()
        op_effect.set_timing(EffectTiming.OnEnterFieldAnyone)
        op_effect.set_effect_name("AD1-001 On Play: recover from trash")
        op_effect.set_effect_description(
            "[On Play] You may return 1 card with [Greymon], [Garurumon] "
            "or [Omnimon] in its name from your trash to the hand.")
        op_effect.is_on_play = True
        op_effect.is_optional = True

        def op_condition(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if player is None:
                return False
            return any(_trash_filter(c) for c in player.trash_cards)
        op_effect.set_can_use_condition(op_condition)

        def op_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase
            valid = [SEL_TRASH_START + i
                     for i, c in enumerate(player.trash_cards) if _trash_filter(c)]
            if not valid:
                return

            def on_select(action_id):
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)
                    game.logger.log(
                        f"[Effect] AD1-001 returned {game._card_ref(chosen)} "
                        f"from trash to hand")

            game.request_selection(
                GamePhase.SelectTrash, player, on_select, valid,
                is_optional=True,
                prompt="You may return 1 [Greymon]/[Garurumon]/[Omnimon] "
                       "from trash to hand.")
        op_effect.set_on_process_callback(op_process)
        effects.append(op_effect)

        # ── [When Digivolving] trash recovery (shared logic) ────────────
        wd_effect = ICardEffect()
        wd_effect.set_timing(EffectTiming.WhenDigivolving)
        wd_effect.set_effect_name("AD1-001 When Digivolving: recover from trash")
        wd_effect.set_effect_description(
            "[When Digivolving] You may return 1 card with [Greymon], "
            "[Garurumon] or [Omnimon] in its name from your trash to the hand.")
        wd_effect.is_when_digivolving = True
        wd_effect.is_optional = True

        def wd_condition(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if player is None:
                return False
            return any(_trash_filter(c) for c in player.trash_cards)
        wd_effect.set_can_use_condition(wd_condition)

        def wd_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase
            valid = [SEL_TRASH_START + i
                     for i, c in enumerate(player.trash_cards) if _trash_filter(c)]
            if not valid:
                return

            def on_select(action_id):
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen = player.trash_cards[idx]
                    player.trash_cards.remove(chosen)
                    player.hand_cards.append(chosen)
                    game.logger.log(
                        f"[Effect] AD1-001 returned {game._card_ref(chosen)} "
                        f"from trash to hand")

            game.request_selection(
                GamePhase.SelectTrash, player, on_select, valid,
                is_optional=True,
                prompt="You may return 1 [Greymon]/[Garurumon]/[Omnimon] "
                       "from trash to hand.")
        wd_effect.set_on_process_callback(wd_process)
        effects.append(wd_effect)

        # ── [All Turns] Reactive digivolve — play observer ──────────────
        # Fires when a Digimon/Tamer is played; checks if it has
        # [Garurumon] or [Tai Kamiya] in name.

        def _is_trigger_name(perm):
            """Check if a permanent has Garurumon or Tai Kamiya in name."""
            return (perm.contains_card_name('Garurumon')
                    or perm.contains_card_name('Tai Kamiya'))

        def _greymon_hand_filter(c):
            """Filter for Digimon cards with [Greymon] in name."""
            if not getattr(c, 'is_digimon', False):
                return False
            return c.contains_card_name('Greymon')

        play_obs = ICardEffect()
        play_obs.set_effect_name("AD1-001 All Turns: reactive digi (play)")
        play_obs.set_effect_description(
            "[All Turns] When Digimon/Tamer played with [Garurumon]/[Tai Kamiya] "
            "→ digivolve into [Greymon] from hand free.")
        play_obs.is_optional = True
        play_obs._is_play_observer = True

        def play_obs_cond(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            played_perm = context.get('played_permanent')
            if played_perm is None:
                return False
            if not _is_trigger_name(played_perm):
                return False
            player = card.owner if card else None
            if player is None:
                return False
            return any(_greymon_hand_filter(c) for c in player.hand_cards)
        play_obs.set_can_use_condition(play_obs_cond)

        def play_obs_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            game.effect_digivolve_from_hand(
                player, perm, _greymon_hand_filter,
                cost_override=0, is_optional=True,
                prompt="Digivolve this Digimon into a [Greymon] from hand "
                       "without paying the cost.")
        play_obs.set_on_process_callback(play_obs_process)
        effects.append(play_obs)

        # ── [All Turns] Reactive digivolve — digivolve observer ─────────
        # Fires when a Digimon digivolves; checks if it has
        # [Garurumon] or [Tai Kamiya] in name.
        digi_obs = ICardEffect()
        digi_obs.set_effect_name("AD1-001 All Turns: reactive digi (digivolve)")
        digi_obs.set_effect_description(
            "[All Turns] When Digimon/Tamer digivolves with [Garurumon]/[Tai Kamiya] "
            "→ digivolve into [Greymon] from hand free.")
        digi_obs.is_optional = True
        digi_obs._is_digivolve_observer = True

        def digi_obs_cond(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            digivolved_perm = context.get('digivolved_permanent')
            if digivolved_perm is None:
                return False
            if not _is_trigger_name(digivolved_perm):
                return False
            player = card.owner if card else None
            if player is None:
                return False
            return any(_greymon_hand_filter(c) for c in player.hand_cards)
        digi_obs.set_can_use_condition(digi_obs_cond)

        def digi_obs_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            game.effect_digivolve_from_hand(
                player, perm, _greymon_hand_filter,
                cost_override=0, is_optional=True,
                prompt="Digivolve this Digimon into a [Greymon] from hand "
                       "without paying the cost.")
        digi_obs.set_on_process_callback(digi_obs_process)
        effects.append(digi_obs)

        # ── Inherited: Raid ─────────────────────────────────────────────
        raid_effect = ICardEffect()
        raid_effect.set_effect_name("AD1-001 Inherited: Raid")
        raid_effect.set_effect_description(
            "Raid (When this Digimon attacks, you may switch the target "
            "of attack to 1 of your opponent's unsuspended Digimon with "
            "the highest DP.)")
        raid_effect.is_inherited_effect = True
        raid_effect.is_on_attack = True
        raid_effect._is_raid = True

        def raid_cond(context: Dict[str, Any]) -> bool:
            return True
        raid_effect.set_can_use_condition(raid_cond)
        effects.append(raid_effect)

        return effects
