from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_094(CardScript):
    """BT11-094 Mirei Mikagura | Tamer | Purple/Yellow | Cost 5

    C# reference (behavioral source of truth):
      [Start of Your Turn] Gain 1 memory.
      [Your Turn] When one of your Digimon digivolves into [Angewomon]
        or [LadyDevimon], if you have 1 or fewer Digimon in play, by
        suspending this Tamer, you may play 1 [Angewomon] or
        [LadyDevimon] with a different name than the Digimon you
        digivolved into from your hand without paying the cost.
      [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: [Start of Your Turn] Gain 1 memory ─────────────
        # C# EffectTiming.OnStartTurn. Field presence + owner-turn gate.
        # NOTE: card text has NO "if memory <= N" gate — fire every turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT11-094 Memory +1")
        effect0.set_effect_description("[Start of Your Turn] Gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory."""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Effect 1: [Your Turn] When digivolving into Angewomon or
        #   LadyDevimon, suspend → play a differently-named target ────
        #
        # IMPORTANT: We use EffectTiming.WhenDigivolving WITHOUT the
        # `is_when_digivolving` flag. When that flag is True the engine's
        # `_effect_matches_timing` requires `perm is digivolved_perm`,
        # which is only valid for effects ON the digivolving Digimon
        # itself (not for a Tamer observing another Digimon's
        # digivolution). Leaving the flag False lets the timing-only
        # match path fire the effect once per host (Mirei).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenDigivolving)
        effect1.set_effect_name(
            "BT11-094 Play 1 [Angewomon] or [LadyDevimon]"
        )
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon digivolves into "
            "[Angewomon] or [LadyDevimon], if you have 1 or fewer "
            "Digimon in play, by suspending this Tamer, you may play "
            "1 [Angewomon] or [LadyDevimon] with a different name "
            "than the Digimon you digivolved into from your hand "
            "without paying the cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm is None:
                return False
            # [Your Turn] — owner must be turn player
            if not (card.owner and card.owner.is_my_turn):
                return False
            # Cost: "by suspending this Tamer" — must be unsuspended
            if tamer_perm.is_suspended:
                return False
            # Condition: "if you have 1 or fewer Digimon in play"
            owner = card.owner
            digimon_count = sum(1 for p in owner.battle_area if p.is_digimon)
            if digimon_count > 1:
                return False
            # Trigger: "one of YOUR Digimon digivolves into [Angewomon]
            # or [LadyDevimon]"
            digivolved_perm = context.get('digivolved_permanent')
            if digivolved_perm is None:
                return False
            # Must be owned by the Tamer's controller
            if digivolved_perm.owner is not owner:
                return False
            top = digivolved_perm.top_card
            if top is None:
                return False
            # Name match on the new top card (substring via
            # contains_card_name captures "X Antibody" variants whose
            # card_names include the base name)
            if not (top.contains_card_name('Angewomon')
                    or top.contains_card_name('LadyDevimon')):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend Mirei, then optionally play a hand card."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm is None or tamer_perm.is_suspended:
                return

            # Pay the cost: suspend this Tamer
            tamer_perm.suspend()

            digivolved_perm = ctx.get('digivolved_permanent')
            digivolved_top = digivolved_perm.top_card if digivolved_perm else None
            # Capture the digivolved-into card's names (exact-match list)
            digivolved_names = set(
                getattr(digivolved_top, 'card_names', []) or []
            ) if digivolved_top else set()

            def play_filter(c) -> bool:
                # Must be an [Angewomon] or [LadyDevimon] card
                if not (c.contains_card_name('Angewomon')
                        or c.contains_card_name('LadyDevimon')):
                    return False
                # "different name than the Digimon you digivolved into"
                hand_names = set(getattr(c, 'card_names', []) or [])
                # If any name overlaps, it's the SAME named card — reject
                if hand_names & digivolved_names:
                    return False
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Effect 2: [Security] Play this card without paying cost ──
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT11-094 Security: Play this card")
        effect2.set_effect_description(
            "Security Effect [Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
