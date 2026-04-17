"""Action mask building for the RL action space.

Free function that takes a Game instance as its first argument.
No module in this file imports from game.py — dependency flows one way.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, List

from .constants import (
    FIELD_SLOTS, TARGETS_PER_ATTACKER, FIELDS_PER_HAND, EFFECTS_PER_PERM,
    SOURCES_PER_FIELD, BREEDING_SLOT, SECURITY_TARGET, ACTION_SPACE_SIZE,
    MAX_SOURCES, SEL_TRASH_START, SEL_TRASH_END, HAND_MAIN_START,
    TRASH_MAIN_START, MAX_TRASH,
)
from ..data.enums import GamePhase, EffectTiming
from ..interfaces.modifiers import ModifierType
from ..validation.digivolve_validator import can_digivolve, has_valid_dna_targets
from ..validation.digixros_validator import has_any_digixros_material

if TYPE_CHECKING:
    from . import Game


def build_action_mask(game: "Game", player_id: int) -> List[float]:
    """Build an ACTION_SPACE_SIZE-float mask (1.0 = valid, 0.0 = invalid).

      0-29:      Play card from hand
      30-59:     [Hand][Main] effects (30 + hand_idx)
      60:        Hatch
      61:        Move from breeding
      62:        Pass / end turn
      63-92:     DNA Digivolve (63 + hand_idx)
      100-399:   Attack (100 + attacker*15 + target, target 14 = security)
      400-999:   Digivolve (400 + hand*15 + field, field 14 = breeding)
      1000-1149: Activate effect (1000 + perm*10 + effectIdx)
      1150-1194: [Trash][Main] effects (1150 + trash_idx)
      2000-2167: Source selection (2000 + field*12 + sourceIdx)
    """
    mask = [0.0] * ACTION_SPACE_SIZE
    me = game.player1 if player_id == 1 else game.player2
    opp = game.player2 if player_id == 1 else game.player1

    phase = game.current_phase

    if phase == GamePhase.Mulligan:
        if game.active_player is not None and player_id == game.active_player.player_id:
            # Opening mulligan actions:
            #   0 = keep hand, 1 = mulligan (redraw 5)
            mask[0] = 1.0
            if not game._mulligan_used.get(player_id, False):
                mask[1] = 1.0

    elif phase == GamePhase.Main:
        # Play cards (0-29)
        for i in range(min(len(me.hand_cards), 30)):
            card = me.hand_cards[i]
            # DCGO: ICanNotPlayCardEffect blocks playing from hand
            if game._is_play_blocked_by_modifier(card):
                continue
            play_cost = game.calculate_play_cost(me, card)
            # DigiXros: account for potential cost reduction from materials.
            # This is an optimistic estimate (assumes max materials available).
            # The actual cost depends on how many materials the agent selects,
            # but selection is optional so the agent can always decline.
            effective_cost = play_cost
            if card.has_digixros:
                xros = card.digixros_cost
                if xros and has_any_digixros_material(card, me):
                    max_mats = xros.max_materials if xros.max_materials >= 0 else 99
                    max_reduction = max_mats * xros.reduce_cost_per_card
                    effective_cost = max(0, play_cost - max_reduction)
            if game.memory >= 0 and effective_cost <= game.memory + 10:
                # Option color requirement: must have a matching-color
                # Digimon or Tamer on the field to play an Option card.
                # Cards with match_color_requirement=False bypass this check.
                # IGNORE_COLOR_REQUIREMENT aura modifiers also bypass this check.
                if card.is_option and card.match_color_requirement:
                    # Check if any IGNORE_COLOR_REQUIREMENT modifier is active
                    ignore_color = False
                    if hasattr(game, 'modifiers'):
                        ctx = {'card': card, 'player': me}
                        for entry in game.modifiers._modifiers.get(ModifierType.IGNORE_COLOR_REQUIREMENT, []):
                            if entry.condition is None or entry.condition(None, ctx):
                                ignore_color = True
                                break
                    if not ignore_color:
                        option_colors = set(card.card_colors)
                        has_color_match = any(
                            bool(option_colors & set(p.top_card.card_colors))
                            for p in me.battle_area
                            if p.top_card and (p.is_digimon or p.is_tamer)
                        )
                        if (not has_color_match and me.breeding_area and me.breeding_area.top_card
                                and me.breeding_area.is_digimon):
                            has_color_match = bool(
                                option_colors & set(me.breeding_area.top_card.card_colors)
                            )
                        if not has_color_match:
                            continue
                mask[i] = 1.0

        # Attack (100-399): 100 + attacker*TARGETS_PER_ATTACKER + target
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            attacker = me.battle_area[i]
            if not attacker.can_attack():
                continue
            # <Blitz>: when digivolved this turn, can attack even when memory < 0
            # Normal rule: can only attack during own turn (memory >= 0)
            if game.memory < 0:
                has_blitz = attacker.has_keyword('_is_blitz')
                digivolved_this_turn = (attacker.turn_digivolved >= 0 and
                                        attacker.turn_digivolved == game.turn_count)
                if not (has_blitz and digivolved_this_turn):
                    continue
            # Security attack (target index SECURITY_TARGET) — check cannot_attack_player
            if attacker.can_attack_player():
                mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
            # Digimon attacks (targets 0..FIELD_SLOTS-1, suspended only per rules)
            if attacker.has_keyword('_is_cannot_attack_digimon'):
                continue
            has_raid = attacker.has_keyword('_is_raid')
            # CAN_ATTACK_UNSUSPENDED: modifier or keyword check
            can_atk_unsuspended = attacker.has_keyword('_is_can_attack_unsuspended')
            if not can_atk_unsuspended and hasattr(game, 'modifiers'):
                can_atk_unsuspended = game.modifiers.has_modifier(
                    attacker, ModifierType.CAN_ATTACK_UNSUSPENDED)
            for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                target = opp.battle_area[j]
                # CANNOT_ATTACK_TARGET: per-pair attack restriction queried
                # with {'attacker': attacker}. Used for cards like BT10-042
                # Venusmon where a specific defender refuses attacks from
                # specific attackers.
                if hasattr(game, 'modifiers') and game.modifiers.has_modifier(
                        target, ModifierType.CANNOT_ATTACK_TARGET,
                        {'attacker': attacker}):
                    continue
                if target.is_suspended and target.is_digimon:
                    mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                elif not target.is_suspended and target.is_digimon:
                    if can_atk_unsuspended:
                        mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                    elif has_raid:
                        unsuspended_dps = [
                            (p.dp or 0) for p in opp.battle_area
                            if not p.is_suspended and p.is_digimon
                        ]
                        if unsuspended_dps and (target.dp or 0) == max(unsuspended_dps):
                            mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

        # Digivolve (400-999): 400 + hand*FIELDS_PER_HAND + field
        for h in range(min(len(me.hand_cards), 30)):
            card = me.hand_cards[h]
            if not card.is_digimon:
                continue
            for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                base_perm = me.battle_area[f]
                # DCGO: ICanNotDigivolveEffect blocks digivolution on target
                # Pass digivolving_card so conditional restrictions (e.g. "only into X") work
                if game.modifiers.has_modifier(base_perm, ModifierType.CANNOT_DIGIVOLVE,
                                               {'digivolving_card': card}):
                    continue
                if can_digivolve(card, base_perm):
                    mask[400 + h * FIELDS_PER_HAND + f] = 1.0
            # Virtual field index BREEDING_SLOT = breeding area digivolve.
            if me.breeding_area and can_digivolve(card, me.breeding_area):
                mask[400 + h * FIELDS_PER_HAND + BREEDING_SLOT] = 1.0

        # DNA Digivolve (63-92): 63 + hand_idx
        for h in range(min(len(me.hand_cards), 30)):
            card = me.hand_cards[h]
            if not card.is_digimon:
                continue
            if has_valid_dna_targets(card, me.battle_area):
                mask[63 + h] = 1.0

        # [Hand][Main] effects (30-59): 30 + hand_idx
        for h in range(min(len(me.hand_cards), 30)):
            card = me.hand_cards[h]
            effects = card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if getattr(effect, '_is_hand_main', False):
                    ctx = {'game': game, 'player': me}
                    if effect.can_use_condition is None or effect.can_use_condition(ctx):
                        mask[HAND_MAIN_START + h] = 1.0
                        break

        # <Training>: suspend to place top deck card at bottom of digi stack (1000-1999)
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            if (perm.is_digimon and not perm.is_suspended
                    and perm.has_keyword('_is_training') and me.library_cards):
                mask[1000 + i * EFFECTS_PER_PERM] = 1.0

        # <Delay>: trash Option card in battle area to activate delayed effect (effectIdx=1)
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            if (game._has_delay_effect(perm)
                    and perm.turn_played < game.turn_count):  # Not same turn placed
                mask[1000 + i * EFFECTS_PER_PERM + 1] = 1.0

        # Field [Main] effects (effectIdx=2): player-activatable [Main] on field permanents
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            for source in perm.card_sources:
                found = False
                for effect in source.effect_list(EffectTiming.NoTiming):
                    if getattr(effect, '_is_field_main', False):
                        ctx = {'game': game, 'player': me, 'permanent': perm}
                        if effect.can_use_condition is None or effect.can_use_condition(ctx):
                            mask[1000 + i * EFFECTS_PER_PERM + 2] = 1.0
                            found = True
                            break
                if found:
                    break

        # [Trash][Main] effects (1150+): activatable [Main] effects on cards in trash
        for t in range(min(len(me.trash_cards), MAX_TRASH)):
            card = me.trash_cards[t]
            effects = card.effect_list(EffectTiming.NoTiming)
            for effect in effects:
                if getattr(effect, '_is_trash_main', False):
                    ctx = {'game': game, 'player': me, 'card': card}
                    if effect.can_use_condition is None or effect.can_use_condition(ctx):
                        mask[TRASH_MAIN_START + t] = 1.0
                        break

        # Force Attack: if any of the turn player's Digimon have
        # FORCE_ATTACK modifier, restrict the mask to attack actions
        # for those Digimon only (opponent grants "this Digimon attacks").
        forced_attackers = []
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            if (perm.is_digimon
                    and game.modifiers.has_modifier(perm, ModifierType.FORCE_ATTACK)):
                forced_attackers.append(i)

        if forced_attackers:
            # Only allow attack actions for forced Digimon
            forced_mask = [0.0] * ACTION_SPACE_SIZE
            has_any_attack = False
            for i in forced_attackers:
                attacker = me.battle_area[i]
                if not attacker.can_attack():
                    continue
                # Security attack
                if attacker.can_attack_player():
                    forced_mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
                    has_any_attack = True
                # Digimon attacks
                has_raid = attacker.has_keyword('_is_raid')
                can_atk_unsuspended_f = attacker.has_keyword('_is_can_attack_unsuspended')
                if not can_atk_unsuspended_f:
                    can_atk_unsuspended_f = game.modifiers.has_modifier(
                        attacker, ModifierType.CAN_ATTACK_UNSUSPENDED)
                for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                    target = opp.battle_area[j]
                    # CANNOT_ATTACK_TARGET per-pair check
                    if game.modifiers.has_modifier(
                            target, ModifierType.CANNOT_ATTACK_TARGET,
                            {'attacker': attacker}):
                        continue
                    if target.is_suspended and target.is_digimon:
                        forced_mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                        has_any_attack = True
                    elif not target.is_suspended and target.is_digimon:
                        if can_atk_unsuspended_f:
                            forced_mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                            has_any_attack = True
                        elif has_raid:
                            unsuspended_dps = [
                                (p.dp or 0) for p in opp.battle_area
                                if not p.is_suspended and p.is_digimon
                            ]
                            if unsuspended_dps and (target.dp or 0) == max(unsuspended_dps):
                                forced_mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0
                                has_any_attack = True
            if has_any_attack:
                return forced_mask
            # If forced Digimon can't attack (suspended etc), skip force
            # and fall through to normal mask

        # Pass (62) - always valid in main
        mask[62] = 1.0

    elif phase == GamePhase.Breeding:
        # Hatch (60)
        if me.breeding_area is None and me.digitama_library_cards:
            mask[60] = 1.0
        # Move (61)
        if me.breeding_area is not None and (me.breeding_area.level or 0) >= 3:
            mask[61] = 1.0
        # <Training> in breeding area
        ba = me.breeding_area
        if (ba is not None and ba.is_digimon and not ba.is_suspended
                and ba.has_keyword('_is_training') and me.library_cards):
            mask[1000 + BREEDING_SLOT * EFFECTS_PER_PERM] = 1.0
        # Pass (62)
        mask[62] = 1.0

    elif phase == GamePhase.BlockTiming:
        from ..core.permanent import Permanent as _Permanent
        attacker = game.pending_attack.attacker if game.pending_attack else _Permanent([])
        has_collision = attacker.has_keyword('_is_collision')
        has_any_blocker = False
        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            if perm.can_block(attacker):
                mask[100 + i] = 1.0
                has_any_blocker = True
        # <Collision>: opponent must block (cannot decline) if blockers exist
        if has_collision and has_any_blocker:
            mask[62] = 0.0  # forced block
        else:
            mask[62] = 1.0  # can decline

    elif phase == GamePhase.CounterTiming:
        mask[62] = 1.0  # always can decline
        # Blast digivolve options (400-999): 400 + hand*FIELDS_PER_HAND + field
        for h in range(min(len(me.hand_cards), 30)):
            card = me.hand_cards[h]
            if not card.is_digimon:
                continue
            # Check for blast digivolve effect
            effects = card.effect_list(EffectTiming.NoTiming)
            has_blast = any(getattr(e, '_is_blast_digivolve', False) for e in effects)
            if not has_blast:
                continue
            # Check valid field targets using proper evo_costs validation
            for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                base_perm = me.battle_area[f]
                if can_digivolve(card, base_perm):
                    mask[400 + h * FIELDS_PER_HAND + f] = 1.0

    elif phase == GamePhase.EndOfTurnAction:
        # End-of-turn keyword actions: Vortex attacks and Overclock activations
        mask[62] = 1.0  # Can always decline / end turn

        for i in range(min(len(me.battle_area), FIELD_SLOTS)):
            perm = me.battle_area[i]
            if not perm.is_digimon:
                continue

            # <Vortex>: attack opponent Digimon at end of turn
            if perm.has_keyword('_is_vortex') and perm.can_attack(is_vortex=True):
                for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                    target = opp.battle_area[j]
                    if target.is_digimon:
                        if game.modifiers.has_modifier(
                                target, ModifierType.CANNOT_ATTACK_TARGET,
                                {'attacker': perm}):
                            continue
                        mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

            # <Overclock>: activate to sacrifice + attack player
            if perm.has_keyword('_is_overclock'):
                has_sacrifice = any(
                    p is not perm and (p.is_token or p.is_digimon)
                    for p in me.battle_area
                )
                if has_sacrifice:
                    mask[1000 + i * EFFECTS_PER_PERM] = 1.0

            # MAY_ATTACK: Digimon granted optional attack at end of turn
            if game.modifiers.has_modifier(perm, ModifierType.MAY_ATTACK) and perm.can_attack():
                if perm.can_attack_player():
                    mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
                if not perm.has_keyword('_is_cannot_attack_digimon'):
                    for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                        target = opp.battle_area[j]
                        if target.is_digimon:
                            if game.modifiers.has_modifier(
                                    target, ModifierType.CANNOT_ATTACK_TARGET,
                                    {'attacker': perm}):
                                continue
                            mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

            # FORCE_ATTACK: Digimon granted mandatory attack at end of turn
            if game.modifiers.has_modifier(perm, ModifierType.FORCE_ATTACK) and perm.can_attack():
                if perm.can_attack_player():
                    mask[100 + i * TARGETS_PER_ATTACKER + SECURITY_TARGET] = 1.0
                if not perm.has_keyword('_is_cannot_attack_digimon'):
                    for j in range(min(len(opp.battle_area), FIELD_SLOTS)):
                        target = opp.battle_area[j]
                        if target.is_digimon:
                            if game.modifiers.has_modifier(
                                    target, ModifierType.CANNOT_ATTACK_TARGET,
                                    {'attacker': perm}):
                                continue
                            mask[100 + i * TARGETS_PER_ATTACKER + j] = 1.0

    elif phase == GamePhase.AllianceTiming:
        # Alliance: select an unsuspended ally to suspend for DP + SA+1 bonus
        mask[62] = 1.0  # Can always decline Alliance
        pa = game.pending_attack
        if pa:
            for i in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[i]
                if perm is not pa.attacker and perm.is_digimon and not perm.is_suspended:
                    mask[100 + i] = 1.0

    elif phase == GamePhase.SelectTrash:
        ps = game.pending_selection
        if ps and ps.valid_indices:
            for idx in ps.valid_indices:
                if 0 <= idx < ACTION_SPACE_SIZE:
                    mask[idx] = 1.0
        else:
            max_trash_sel = SEL_TRASH_END - SEL_TRASH_START + 1  # 50 slots
            for i in range(min(len(me.trash_cards), max_trash_sel)):
                mask[SEL_TRASH_START + i] = 1.0
        if ps and getattr(ps, 'is_optional', False):
            mask[62] = 1.0

    elif phase == GamePhase.SelectSource:
        ps = game.pending_selection
        if ps and ps.valid_indices:
            for idx in ps.valid_indices:
                if 0 <= idx < ACTION_SPACE_SIZE:
                    mask[idx] = 1.0
        else:
            # Source selection: 2000 + field*SOURCES_PER_FIELD + sourceIdx
            for f in range(min(len(me.battle_area), FIELD_SLOTS)):
                perm = me.battle_area[f]
                for s in range(min(len(perm.card_sources), MAX_SOURCES)):
                    mask[2000 + f * SOURCES_PER_FIELD + s] = 1.0
        if ps and getattr(ps, 'is_optional', False):
            mask[62] = 1.0

    elif phase in (GamePhase.SelectTarget, GamePhase.SelectMaterial,
                   GamePhase.SelectHand, GamePhase.SelectReveal,
                   GamePhase.SelectEffectChoice, GamePhase.SelectSecurity):
        # Generic selection: use valid_indices from pending selection
        ps = game.pending_selection
        if ps is None:
            # Defensive recovery: selection phase with no pending selection
            game.logger.log(f"[Recovery] Selection phase {phase} with no pending selection — recovering")
            game.current_phase = GamePhase.Main
            game.active_player = None
            return build_action_mask(game, player_id)
        if ps.valid_indices:
            for idx in ps.valid_indices:
                if 0 <= idx < ACTION_SPACE_SIZE:
                    mask[idx] = 1.0
        # Optional selections allow declining (pass)
        if getattr(ps, 'is_optional', False):
            mask[62] = 1.0
        # Safety: if mask is all zeros for selection phase, force recovery
        if not any(mask):
            game.logger.log(
                f"[Recovery] All-zero mask in selection phase {phase} — recovering to Main")
            game.pending_selection = None
            game.current_phase = GamePhase.Main
            game.active_player = None
            return build_action_mask(game, player_id)

    return mask
