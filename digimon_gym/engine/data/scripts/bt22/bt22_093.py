from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


def _has_cs_trait(c) -> bool:
    """Exact-match CS trait check (DCGO EqualsTraits semantics).

    DCGO CardSource.HasCSTraits => EqualsTraits("CS") which does exact
    string equality against each entry in CardTraits. Substring matching
    would incorrectly admit composites like 'Hudie/CS'.
    """
    if c is None:
        return False
    traits = getattr(c, 'card_traits', []) or []
    return any(t == 'CS' for t in traits)


class BT22_093(CardScript):
    """BT22-093 Ami Aiba

    [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
    [Your Turn] When any of your Digimon digivolve into a Digimon with the
      [CS] trait, if it has a digivolution card with the same level as the
      digivolved Digimon, by suspending this Tamer, that Digimon may digivolve
      into a Digimon card with the [CS] trait in the hand without paying the
      cost.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: [Start of Your Main Phase] ─────────────────────
        # If your opponent has a Digimon, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT22-093 Memory +1")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card is None or card.permanent_of_this_card() is None:
                return False
            if not (card.owner and card.owner.is_my_turn):
                return False
            enemy = card.owner.enemy
            return bool(enemy and any(p.is_digimon for p in enemy.battle_area))

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Effect 1: [Your Turn] OnDigivolve observer ───────────────
        # When any of your Digimon digivolve into a Digimon with the [CS] trait,
        # if it has a digivolution card with the same level as the digivolved
        # Digimon, by suspending this Tamer, that Digimon may digivolve into a
        # Digimon card with the [CS] trait in the hand without paying the cost.
        #
        # Uses _is_digivolve_observer pattern (fires from _fire_digivolve_observers).
        # MUST NOT set is_when_digivolving=True — that flag routes via
        # execute_effects(WhenDigivolving) which requires the effect source
        # permanent to BE the digivolved permanent (see BT11-094 discovery).
        effect1 = ICardEffect()
        effect1.set_effect_name(
            "BT22-093 Suspend self, chain-digivolve into [CS] Digimon from hand"
        )
        effect1.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve into a Digimon with "
            "the [CS] trait, if it has a digivolution card with the same level "
            "as the digivolved Digimon, by suspending this Tamer, that Digimon "
            "may digivolve into a Digimon card with the [CS] trait in the hand "
            "without paying the cost."
        )
        effect1.is_optional = True
        effect1._is_digivolve_observer = True

        def condition1(context: Dict[str, Any]) -> bool:
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return False
            if not (card.owner and card.owner.is_my_turn):
                return False
            # Cost check: tamer must be unsuspended so we can pay by suspending it.
            if tamer_perm.is_suspended:
                return False

            digi_perm = context.get('digivolved_permanent')
            if not digi_perm or not digi_perm.top_card:
                return False
            # Must belong to the same player (DCGO IsPermanentExistsOnOwnerBattleAreaDigimon)
            owner = card.owner
            if owner and digi_perm not in owner.battle_area:
                return False
            # Top card must have [CS] trait (exact match)
            if not _has_cs_trait(digi_perm.top_card):
                return False
            # Must have a digivolution card (stack source BELOW the top) whose
            # level matches the new top card's level. DCGO DigivolutionCards
            # excludes TopCard; Python's digivolution_cards currently returns
            # ALL card_sources so we must exclude the top explicitly.
            digi_level = digi_perm.top_card.level
            if digi_level is None:
                return False
            top_source = digi_perm.top_card
            stack_sources = [
                cs for cs in digi_perm.card_sources if cs is not top_source
            ]
            if not any(
                cs.level is not None and cs.level == digi_level
                for cs in stack_sources
            ):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, then let the digivolved Digimon chain-digivolve
            into a [CS] Digimon from hand without paying the cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm or tamer_perm.is_suspended:
                return
            digi_perm = ctx.get('digivolved_permanent')
            if not digi_perm:
                return

            # Cost: suspend this Tamer
            tamer_perm.suspend()

            # Offer the chain digivolve (optional, from hand, free).
            def digi_filter(c):
                return getattr(c, 'is_digimon', False) and _has_cs_trait(c)

            game.effect_digivolve_from_hand(
                player,
                digi_perm,
                digi_filter,
                cost_override=0,
                is_optional=True,
                prompt=(
                    "Select a [CS] Digimon from hand to digivolve "
                    f"{game._perm_ref(digi_perm)} without paying the cost."
                ),
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Effect 2: [Security] Play this card without paying the cost ──
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT22-093 Security: Play this card")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            game = ctx.get('game')
            player = ctx.get('player')
            if not (game and player):
                return
            game.effect_play_from_security(player, card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
