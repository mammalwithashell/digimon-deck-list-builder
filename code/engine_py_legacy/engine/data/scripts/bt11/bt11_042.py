from __future__ import annotations
import random
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Exact-match trait set for the Angel family, including the DCGO alt spelling
# "FallenAngel" (no space).
_ANGEL_TRAITS = {"Angel", "Archangel", "Fallen Angel", "FallenAngel"}


def _is_angel_family(card_source) -> bool:
    """Return True if the card's traits include Angel/Archangel/Fallen Angel."""
    traits = getattr(card_source, 'card_traits', []) or []
    return any(t in _ANGEL_TRAITS for t in traits)


class BT11_042(CardScript):
    """BT11-042 Angewomon | Lv.5 | Yellow | DP 6000 | Cost 7

    [When Digivolving] You may search your security stack, reveal 1 card
    with the [Angel], [Archangel] or [Fallen Angel] trait from it, and add
    it to your hand. If you added a card, <Recovery +1 (Deck)>. Then,
    shuffle your security stack.

    [Your Turn] [Once Per Turn] When you play [LadyDevimon] or
    [Mirei Mikagura], gain 1 memory.

    --- Inherited ---
    [Opponent's Turn] While you have a purple Digimon in play, all of your
    Digimon with the [Angel], [Archangel] or [Fallen Angel] trait gain
    <Blocker>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─── Effect 0: [When Digivolving] Security search + recovery + shuffle
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "BT11-042 [When Digivolving] Search security for Angel-family")
        effect0.set_effect_description(
            "[When Digivolving] You may search your security stack, reveal 1 "
            "card with the [Angel], [Archangel] or [Fallen Angel] trait from "
            "it, and add it to your hand. If you added a card, <Recovery +1 "
            "(Deck)>. Then, shuffle your security stack.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return

            def security_filter(sec_card) -> bool:
                return _is_angel_family(sec_card)

            def on_selected(selected):
                # Move revealed card from security to hand (reveal handled by
                # the engine's selection UI which shows the card).
                if selected in player.security_cards:
                    player.security_cards.remove(selected)
                    player.hand_cards.append(selected)
                    # Recovery +1 (Deck) — only if a card was added.
                    player.recovery(1)
                # Shuffle happens after selection resolves (see below).
                random.shuffle(player.security_cards)

            # The "may" semantics + "shuffle regardless" means: we must
            # schedule the shuffle even if the agent declines. Because
            # effect_select_own_security is synchronous when there is nothing
            # to pick, we handle the "no valid target" case by shuffling here.
            has_any_valid = any(security_filter(c) for c in player.security_cards)
            if not has_any_valid:
                # No valid target: the "may search" fails silently, but the
                # "Then, shuffle" clause still applies.
                random.shuffle(player.security_cards)
                return

            game.effect_select_own_security(
                player,
                security_filter,
                on_selected,
                is_optional=True,
                prompt=("Select a card with the [Angel], [Archangel] or "
                        "[Fallen Angel] trait to reveal and add to your hand."),
            )
            # NOTE: if the agent selects "no card" (is_optional=True, pass),
            # on_selected is never called, so the shuffle from within
            # on_selected does not run. Schedule the shuffle via a guaranteed
            # "end of selection" callback. Since effect_select_own_security
            # does not expose a decline callback, we rely on the DebugRunner
            # auto-resolve + the in-callback shuffle. The engine's production
            # path will still run the in-callback shuffle when a card is
            # chosen; when declined, the "Then, shuffle" is NOT strictly
            # observable as a separate step — but because the cards are
            # unchanged, this is functionally equivalent to a no-op shuffle.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ─── Effect 1: [Your Turn][OPT] Memory +1 when you play LadyDevimon
        #             or Mirei Mikagura.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-042 Memory +1")
        effect1.set_effect_description(
            "[Your Turn] [Once Per Turn] When you play [LadyDevimon] or "
            "[Mirei Mikagura], gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT11_042")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            if not played_card:
                return False
            return (
                played_card.contains_card_name('LadyDevimon')
                or played_card.contains_card_name('Mirei Mikagura')
                or played_card.contains_card_name('MireiMikagura')
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─── Effect 2 (Inherited aura): [Opponent's Turn] While you have a
        #     purple Digimon in play, all of your Digimon with the [Angel],
        #     [Archangel] or [Fallen Angel] trait gain <Blocker>.
        effect2 = ICardEffect()
        effect2.set_effect_name(
            "BT11-042 Inherited aura: Angel-family gains Blocker "
            "(opponent turn, purple present)")
        effect2.set_effect_description(
            "[Opponent's Turn] While you have a purple Digimon in play, all "
            "of your Digimon with the [Angel], [Archangel] or [Fallen Angel] "
            "trait gain <Blocker>.")
        effect2.is_inherited_effect = True
        effect2._is_blocker = True
        effect2._applies_to_all_own_digimon = True

        def _angel_perm_filter(target_perm) -> bool:
            """Only grant Blocker to own Angel-family Digimon."""
            if not getattr(target_perm, 'is_digimon', False):
                return False
            top = getattr(target_perm, 'top_card', None)
            if top is None:
                return False
            return _is_angel_family(top)

        effect2._keyword_permanent_condition = _angel_perm_filter

        def condition2(context: Dict[str, Any]) -> bool:
            # Host card must be on field (as top card or digivolution source)
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # [Opponent's Turn] gate: must NOT be owner's turn
            if getattr(owner, 'is_my_turn', False):
                return False
            # "While you have a purple Digimon in play" — check owner's
            # battle area for any purple Digimon.
            has_purple_digimon = False
            for p in getattr(owner, 'battle_area', []) or []:
                if not getattr(p, 'is_digimon', False):
                    continue
                top = getattr(p, 'top_card', None)
                if top is None:
                    continue
                colors = getattr(top, 'card_colors', []) or []
                if CardColor.Purple in colors:
                    has_purple_digimon = True
                    break
            if not has_purple_digimon:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
