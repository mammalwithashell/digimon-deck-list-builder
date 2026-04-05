"""Effect helper mixin for the Game class.

Common effect patterns used by card scripts: selection helpers, token creation,
play-from-zone, digivolve effects, DNA digivolve flows, etc.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, Optional, List, Callable

from .constants import (
    FIELD_SLOTS, EFFECTS_PER_PERM, ACTION_SPACE_SIZE,
    MAX_HAND, MAX_TRASH,
    SEL_HAND_START, SEL_REVEALED_START, SEL_TRASH_START,
    SEL_MY_FIELD_START, SEL_OPP_FIELD_START, SEL_MY_BREEDING,
    SEL_MY_SECURITY_START, SEL_OPP_SECURITY_START,
    SEL_EFFECT_CHOICE_START,
    PendingSelection,
)
from ..data.enums import GamePhase, EffectTiming
from ..interfaces.modifiers import ModifierType, ModifierEntry
from ..validation.digivolve_validator import (
    has_valid_dna_targets, get_valid_dna_first_targets,
    get_valid_dna_second_targets, get_dna_stacking_order,
)
from ..validation.digixros_validator import get_valid_digixros_materials

if TYPE_CHECKING:
    from ..core.card_source import CardSource
    from ..core.permanent import Permanent
    from ..core.player import Player


class EffectHelpersMixin:
    """Effect helper methods used by card scripts."""

    def request_selection(self, phase: GamePhase, player: "Player",
                          callback: Callable[[int], None],
                          valid_indices: Optional[List[int]] = None,
                          is_optional: bool = False,
                          prompt: str = "",
                          effect_choices: Optional[List[dict]] = None,
                          keyword_prompt: Optional[dict] = None,
                          on_decline: Optional[Callable[[], None]] = None):
        """Pause the game to request a selection from the given player."""
        self.pending_selection = PendingSelection(
            callback=callback,
            selecting_player=player,
            previous_phase=self.current_phase,
            valid_indices=valid_indices or [],
            is_optional=is_optional,
            prompt=prompt,
            effect_choices=effect_choices,
            keyword_prompt=keyword_prompt,
            on_decline=on_decline,
        )
        self.current_phase = phase
        self.active_player = player

    def effect_select_opponent_permanent(
        self, player: "Player", callback: Callable[["Permanent"], None],
        filter_fn: Optional[Callable[["Permanent"], bool]] = None,
        is_optional: bool = False,
        prompt: str = "Select an opponent's Digimon.",
    ):
        """Request selection of an opponent's permanent."""
        opponent = self.player2 if player is self.player1 else self.player1
        valid = []
        for i, perm in enumerate(opponent.battle_area):
            if not self.modifiers.can_be_selected_by_effect(perm):
                continue
            if filter_fn is None or filter_fn(perm):
                valid.append(SEL_OPP_FIELD_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_OPP_FIELD_START
            opp = self.player2 if player is self.player1 else self.player1
            if 0 <= idx < len(opp.battle_area):
                callback(opp.battle_area[idx])

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_own_permanent(
        self, player: "Player", callback: Callable[["Permanent"], None],
        filter_fn: Optional[Callable[["Permanent"], bool]] = None,
        is_optional: bool = False,
        prompt: str = "Select one of your Digimon.",
    ):
        """Request selection of one of the player's own permanents."""
        valid = []
        for i, perm in enumerate(player.battle_area):
            if filter_fn is None or filter_fn(perm):
                valid.append(SEL_MY_FIELD_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_MY_FIELD_START
            if 0 <= idx < len(player.battle_area):
                callback(player.battle_area[idx])

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_reveal_and_select(
        self, player: "Player", count: int,
        filter_fn: Callable[["CardSource"], bool],
        on_selected: Callable[["CardSource", List["CardSource"]], None],
        is_optional: bool = False,
        prompt: str = "",
    ):
        """Reveal top N cards, let agent pick one matching filter, return rest to bottom."""
        revealed = player.library_cards[:count]
        if not revealed:
            return

        self.revealed_cards = list(revealed)

        valid = []
        for i, card in enumerate(revealed):
            if filter_fn(card):
                valid.append(SEL_REVEALED_START + i)
        if not valid:
            for card in revealed:
                player.library_cards.remove(card)
                player.library_cards.append(card)
            self.revealed_cards = []
            return

        def on_select(action_id: int):
            idx = action_id - SEL_REVEALED_START
            if 0 <= idx < len(revealed):
                selected = revealed[idx]
                remaining = [c for c in revealed if c is not selected]
                for c in revealed:
                    if c in player.library_cards:
                        player.library_cards.remove(c)
                on_selected(selected, remaining)
            self.revealed_cards = []

        def on_decline_reveal():
            for card in revealed:
                if card in player.library_cards:
                    player.library_cards.remove(card)
                    player.library_cards.append(card)
            self.revealed_cards = []

        auto_prompt = prompt or f"Select a card from the {count} revealed cards."
        self.request_selection(
            GamePhase.SelectReveal, player, on_select, valid, is_optional,
            prompt=auto_prompt, on_decline=on_decline_reveal)

    def effect_reveal_and_select_multi(
        self, player: "Player", count: int,
        passes: list,
        remaining_placement: str = 'deck_bottom',
        is_optional: bool = False,
    ):
        """Reveal top N cards, then run multiple sequential selection passes."""
        revealed = player.library_cards[:count]
        if not revealed:
            return

        for c in revealed:
            player.library_cards.remove(c)

        pool = list(revealed)

        def run_pass(pass_idx: int):
            if pass_idx >= len(passes) or not pool:
                self.revealed_cards = []
                for c in pool:
                    if remaining_placement == 'deck_bottom':
                        player.library_cards.append(c)
                    elif remaining_placement == 'hand':
                        player.hand_cards.append(c)
                    elif remaining_placement == 'trash':
                        player.trash_cards.append(c)
                return

            filter_fn, placement = passes[pass_idx]
            self.revealed_cards = list(pool)

            valid = []
            for i, card in enumerate(pool):
                if filter_fn(card):
                    valid.append(SEL_REVEALED_START + i)

            if not valid:
                run_pass(pass_idx + 1)
                return

            def on_select(action_id: int):
                idx = action_id - SEL_REVEALED_START
                if 0 <= idx < len(pool):
                    selected = pool.pop(idx)
                    if placement == 'hand':
                        player.hand_cards.append(selected)
                    elif placement == 'trash':
                        player.trash_cards.append(selected)
                self.revealed_cards = []
                run_pass(pass_idx + 1)

            def on_decline_pass():
                self.revealed_cards = []
                run_pass(pass_idx + 1)

            self.request_selection(
                GamePhase.SelectReveal, player, on_select, valid, is_optional,
                prompt=f"Select a card from the revealed cards (pass {pass_idx + 1}).",
                on_decline=on_decline_pass)

        run_pass(0)

    def effect_play_token(
        self, player: "Player", token_type: str,
        on_opponent_field: bool = False, count: int = 1,
    ):
        """Create and play token permanent(s) onto a player's field."""
        from ..data.token_registry import create_token_card_source
        from ..core.permanent import Permanent

        target_player = player.enemy if on_opponent_field else player
        if target_player is None:
            return

        for _ in range(count):
            if len(target_player.battle_area) >= FIELD_SLOTS:
                self.logger.log(f"[Token] Field full — cannot play {token_type} token")
                break

            card_source = create_token_card_source(token_type, target_player)
            perm = Permanent([card_source])
            perm.turn_played = self.turn_count
            perm._owner_game = self
            target_player.battle_area.append(perm)

            # Register CANNOT_SUSPEND modifier for petrification
            if token_type == 'petrification':
                self.modifiers.register(ModifierEntry(
                    modifier_type=ModifierType.CANNOT_SUSPEND,
                    condition=lambda t, ctx, p=perm, o=target_player: (
                        t is p and o.is_my_turn and p in o.battle_area
                    ),
                    source_permanent=perm,
                    expiry='permanent',
                ))

            self.logger.log(
                f"[Token] {token_type.title()} Token played on "
                f"{target_player.player_name}'s field")

            self.execute_effects(
                EffectTiming.OnEnterFieldAnyone,
                {
                    "played_card": card_source,
                    "played_permanent": perm,
                    "event_permanent": perm,
                    "event_player": target_player,
                },
            )
            self._fire_play_observers(perm, target_player)

    def effect_play_from_security(self, player: "Player", card: "CardSource"):
        """Play a card from security without paying the cost.

        Used by [Security] effects like "Play this card without paying the cost."
        Creates a Permanent, adds to battle_area, marks the card so
        security_attack() won't trash it, and fires OnEnterFieldAnyone.
        """
        from ..core.permanent import Permanent

        if len(player.battle_area) >= FIELD_SLOTS:
            self.logger.log(f"[Security] Field full — cannot play {self._card_ref(card)}")
            return

        card._security_played = True
        perm = Permanent([card])
        perm.turn_played = self.turn_count
        perm._owner_game = self
        player.battle_area.append(perm)

        self.logger.log(
            f"[Security] {player.player_name} played "
            f"{self._card_ref(card)} from security without paying the cost.")

        self.execute_effects(
            EffectTiming.OnEnterFieldAnyone,
            {
                "played_card": card,
                "played_permanent": perm,
                "event_permanent": perm,
                "event_player": player,
            },
        )
        self._fire_play_observers(perm, player)

    def _is_play_blocked_by_modifier(self, card: "CardSource") -> bool:
        """Check if CANNOT_PLAY_CARD modifiers block playing this card (hand or effect)."""
        if not hasattr(self, 'modifiers'):
            return False
        ctx = {'card': card}
        for entry in self.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_CARD, []):
            if entry.condition is None or entry.condition(None, ctx):
                return True
        return False

    def _is_effect_play_blocked(self, card: "CardSource") -> bool:
        """Check if CANNOT_PLAY_BY_EFFECT modifiers block playing this card via effect.

        This is separate from _is_play_blocked_by_modifier which also blocks
        normal hand plays. CANNOT_PLAY_BY_EFFECT only blocks effect-based plays.
        """
        if not hasattr(self, 'modifiers'):
            return False
        ctx = {'card': card}
        for entry in self.modifiers._modifiers.get(ModifierType.CANNOT_PLAY_BY_EFFECT, []):
            if entry.condition is None or entry.condition(None, ctx):
                return True
        return False

    def effect_play_from_zone(
        self, player: "Player",
        zone: str,
        filter_fn: Callable[["CardSource"], bool],
        free: bool = True,
        manual_reduction: int = 0,
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let agent pick a card from a zone to play onto the field."""
        if not prompt:
            cost_text = "without paying its cost" if free else "by paying its cost"
            prompt = f"Select a card from {zone} to play {cost_text}."
        if zone == 'hand_or_trash':
            valid = []
            for i, card in enumerate(player.hand_cards):
                if filter_fn(card) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE and not self._is_play_blocked_by_modifier(card) and not self._is_effect_play_blocked(card):
                    valid.append(SEL_HAND_START + i)
            for i, card in enumerate(player.trash_cards):
                if filter_fn(card) and (SEL_TRASH_START + i) < ACTION_SPACE_SIZE and not self._is_play_blocked_by_modifier(card) and not self._is_effect_play_blocked(card):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_select_hot(action_id: int):
                if SEL_TRASH_START <= action_id:
                    idx = action_id - SEL_TRASH_START
                    source = player.trash_cards
                    src_name = 'trash'
                else:
                    idx = action_id - SEL_HAND_START
                    source = player.hand_cards
                    src_name = 'hand'
                if 0 <= idx < len(source):
                    card = source[idx]
                    played_perm = player.play_card_from_source(card, pay_cost=not free)
                    if free:
                        cost = 0
                    else:
                        cost = self.calculate_play_cost(
                            player, card, source_zone=src_name,
                            free=False, manual_reduction=manual_reduction,
                            commit=True,
                        )
                        player.lose_memory(cost)
                        if hasattr(player, "_temp_play_cost_reduction"):
                            player._temp_play_cost_reduction = 0
                    self.logger.log(f"[Effect] {player.player_name} played "
                                    f"{self._card_ref(card)} from {src_name}")
                    if card.is_option:
                        self.execute_effects(
                            EffectTiming.OnUseOption,
                            {"played_card": card, "played_permanent": played_perm, "event_player": player},
                        )
                    self.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": card, "played_permanent": played_perm, "event_player": player},
                    )
                    self._fire_play_observers(played_perm, player)
                    if card.is_option and not self._option_stays_on_field(card):
                        self._trash_option_after_resolution(player, played_perm)

            self.request_selection(GamePhase.SelectTarget, player,
                                   on_select_hot, valid, is_optional,
                                   prompt=prompt)
            return

        if zone == 'hand':
            source_list = player.hand_cards
            offset = SEL_HAND_START
        elif zone == 'trash':
            source_list = player.trash_cards
            offset = SEL_TRASH_START
        elif zone == 'revealed':
            source_list = list(self.revealed_cards)
            offset = SEL_REVEALED_START
        else:
            return

        valid = []
        for i, card in enumerate(source_list):
            if filter_fn(card) and (offset + i) < ACTION_SPACE_SIZE and not self._is_play_blocked_by_modifier(card) and not self._is_effect_play_blocked(card):
                valid.append(offset + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - offset
            if 0 <= idx < len(source_list):
                card = source_list[idx]
                played_perm = player.play_card_from_source(card, pay_cost=not free)
                if free:
                    cost = 0
                else:
                    cost = self.calculate_play_cost(
                        player, card, source_zone=zone,
                        free=False, manual_reduction=manual_reduction,
                        commit=True,
                    )
                    player.lose_memory(cost)
                    if hasattr(player, "_temp_play_cost_reduction"):
                        player._temp_play_cost_reduction = 0
                self.logger.log(f"[Effect] {player.player_name} played "
                                f"{self._card_ref(card)} from {zone}")
                if card.is_option:
                    self.execute_effects(
                        EffectTiming.OnUseOption,
                        {"played_card": card, "played_permanent": played_perm, "event_player": player},
                    )
                self.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": card, "played_permanent": played_perm, "event_player": player},
                )
                self._fire_play_observers(played_perm, player)
                if card.is_option and not self._option_stays_on_field(card):
                    self._trash_option_after_resolution(player, played_perm)

        phase = GamePhase.SelectReveal if zone == 'revealed' else GamePhase.SelectTarget
        self.request_selection(phase, player, on_select, valid, is_optional,
                               prompt=prompt)

    def effect_digivolve_from_hand(
        self, player: "Player", permanent: "Permanent",
        filter_fn: Callable[["CardSource"], bool],
        cost_override: Optional[int] = None,
        cost_reduction: int = 0,
        ignore_requirements: bool = False,
        is_optional: bool = True,
        prompt: str = "",
        include_trash: bool = False,
    ):
        """Let agent pick a hand (or trash) card to digivolve a permanent into via effect.

        Args:
            include_trash: If True, also offer qualifying cards from trash as digivolve sources.
        """
        # DCGO: ICannotIgnoreDigivolutionConditionEffect — check all field perms
        if ignore_requirements:
            for p_check in [self.player1, self.player2]:
                for perm_check in p_check.battle_area:
                    for source in perm_check.card_sources:
                        for eff in source.effect_list(EffectTiming.NoTiming):
                            if getattr(eff, '_cannot_ignore_evo_requirements', False):
                                ctx = {'game': self, 'player': p_check}
                                if eff.can_use_condition is None or eff.can_use_condition(ctx):
                                    ignore_requirements = False
                                    break
                        if not ignore_requirements:
                            break
                    if not ignore_requirements:
                        break
                if not ignore_requirements:
                    break
        if not prompt:
            perm_name = self._perm_ref(permanent)
            zone_text = "hand or trash" if include_trash else "hand"
            prompt = f"Select a card from {zone_text} to digivolve {perm_name}."
        valid = []
        for i, card in enumerate(player.hand_cards):
            if filter_fn(card):
                valid.append(SEL_HAND_START + i)
        if include_trash:
            for i, card in enumerate(player.trash_cards):
                if filter_fn(card):
                    valid.append(SEL_TRASH_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            chosen_card = None
            if action_id >= SEL_TRASH_START:
                idx = action_id - SEL_TRASH_START
                if 0 <= idx < len(player.trash_cards):
                    chosen_card = player.trash_cards[idx]
                    player.trash_cards.remove(chosen_card)
            else:
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    chosen_card = player.hand_cards[idx]
                    player.hand_cards.remove(chosen_card)
            if chosen_card is None:
                return
            if cost_override is not None:
                cost = cost_override
            else:
                base = chosen_card.get_cost_itself
                cost = max(0, base - cost_reduction)
            permanent.add_card_source(chosen_card)
            permanent.turn_digivolved = self.turn_count
            player.lose_memory(cost)
            self.logger.log(
                f"[Effect Digivolve] {self._card_ref(chosen_card)} onto "
                f"{self._perm_ref(permanent)} "
                f"(cost: {cost})")
            player.draw()
            self.execute_effects(EffectTiming.WhenDigivolving,
                                 {"digivolved_permanent": permanent})

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_hand_card(
        self, player: "Player",
        filter_fn: Callable[["CardSource"], bool],
        callback: Callable[["CardSource"], None],
        is_optional: bool = False,
        prompt: str = "Select a card from your hand.",
    ):
        """Let agent pick a card from hand."""
        valid = []
        for i, card in enumerate(player.hand_cards):
            if filter_fn(card):
                valid.append(SEL_HAND_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_HAND_START
            if 0 <= idx < len(player.hand_cards):
                callback(player.hand_cards[idx])

        self.request_selection(
            GamePhase.SelectHand, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_choose_branch(
        self, player: "Player", num_choices: int,
        callback: Callable[[int], None],
        prompt: str = "Choose an effect to activate.",
        branch_labels: Optional[List[str]] = None,
    ):
        """Let agent choose between N effect branches."""
        valid = [SEL_EFFECT_CHOICE_START + i for i in range(num_choices)]

        def on_select(action_id: int):
            branch = action_id - SEL_EFFECT_CHOICE_START
            if 0 <= branch < num_choices:
                callback(branch)

        effect_choices = None
        if branch_labels:
            effect_choices = [
                {"index": SEL_EFFECT_CHOICE_START + i, "label": label}
                for i, label in enumerate(branch_labels)
            ]

        self.request_selection(
            GamePhase.SelectEffectChoice, player, on_select, valid,
            prompt=prompt, effect_choices=effect_choices)

    def effect_choose_deck_placement(
        self, player: "Player", card: "CardSource",
        callback: Optional[Callable] = None,
    ):
        """Let agent choose to place a card on top or bottom of deck.

        Branch 0 = top of deck (insert at index 0).
        Branch 1 = bottom of deck (append).
        """
        def on_choice(branch: int):
            if branch == 0:
                player.library_cards.insert(0, card)
                self.logger.log(f"[Effect] {player.player_name} placed "
                                f"{self._card_ref(card)} on top of deck.")
            else:
                player.library_cards.append(card)
                self.logger.log(f"[Effect] {player.player_name} placed "
                                f"{self._card_ref(card)} on bottom of deck.")
            if callback:
                callback()

        self.effect_choose_branch(
            player, 2, on_choice,
            prompt="Place on top or bottom of deck?",
            branch_labels=["Top of deck", "Bottom of deck"],
        )

    def effect_select_own_security(
        self, player: "Player",
        filter_fn: Callable[["CardSource"], bool],
        callback: Callable[["CardSource"], None],
        is_optional: bool = True,
        prompt: str = "Select a card from your security stack.",
    ):
        """Let agent select a card from their own security stack."""
        valid = []
        for i, card in enumerate(player.security_cards):
            if filter_fn(card):
                valid.append(SEL_MY_SECURITY_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_MY_SECURITY_START
            if 0 <= idx < len(player.security_cards):
                callback(player.security_cards[idx])

        self.request_selection(
            GamePhase.SelectSecurity, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_select_opponent_security(
        self, player: "Player",
        filter_fn: Optional[Callable[["CardSource"], bool]],
        callback: Callable[["CardSource"], None],
        is_optional: bool = True,
        prompt: str = "Select a card from opponent's security stack.",
    ):
        """Let agent select a card from the opponent's security stack."""
        opponent = self.player2 if player is self.player1 else self.player1
        valid = []
        for i, card in enumerate(opponent.security_cards):
            if filter_fn is None or filter_fn(card):
                valid.append(SEL_OPP_SECURITY_START + i)
        if not valid:
            return

        def on_select(action_id: int):
            idx = action_id - SEL_OPP_SECURITY_START
            opp = self.player2 if player is self.player1 else self.player1
            if 0 <= idx < len(opp.security_cards):
                callback(opp.security_cards[idx])

        self.request_selection(
            GamePhase.SelectSecurity, player, on_select, valid, is_optional,
            prompt=prompt)

    def effect_link_to_permanent(
        self, player: "Player", card_to_link: "CardSource",
        filter_fn: Optional[Callable[["Permanent"], bool]] = None,
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let agent choose a Digimon to link an option card to."""
        if not prompt:
            prompt = f"Select a Digimon to link {self._card_ref(card_to_link)} to."
        valid = []

        for i, perm in enumerate(player.battle_area):
            if perm.is_token:
                continue
            if not perm.is_digimon:
                continue
            if filter_fn is not None and not filter_fn(perm):
                continue
            valid.append(SEL_MY_FIELD_START + i)

        ba = player.breeding_area
        if ba is not None and ba.is_digimon and (ba.level or 0) > 2:
            if filter_fn is None or filter_fn(ba):
                valid.append(SEL_MY_BREEDING)

        if not valid:
            return

        def on_select(action_id: int):
            if action_id == SEL_MY_BREEDING:
                target = player.breeding_area
            else:
                idx = action_id - SEL_MY_FIELD_START
                if 0 <= idx < len(player.battle_area):
                    target = player.battle_area[idx]
                else:
                    return
            if target is None:
                return
            target.link_card(card_to_link)
            self.logger.log(
                f"[Link] {self._card_ref(card_to_link)} linked to "
                f"{self._perm_ref(target)}")

        self.request_selection(
            GamePhase.SelectTarget, player, on_select, valid, is_optional,
            prompt=prompt)

    # ─── DNA Digivolve ────────────────────────────────────────────────

    def effect_dna_digivolve_from_hand(
        self, player: "Player",
        filter_fn: Callable[["CardSource"], bool],
        is_optional: bool = True,
        prompt: str = "",
    ):
        """Let an effect trigger a DNA digivolve from hand."""
        candidates = []
        for i, card in enumerate(player.hand_cards):
            if not filter_fn(card):
                continue
            if not card.is_digimon or not card.c_entity_base:
                continue
            if not card.c_entity_base.dna_costs:
                continue
            if has_valid_dna_targets(card, player.battle_area):
                candidates.append(i)
        if not candidates:
            return

        if len(candidates) == 1:
            self._effect_dna_pick_first(player, candidates[0])
        else:
            valid = [SEL_HAND_START + i for i in candidates]
            if not prompt:
                prompt = "Select a card from hand to DNA digivolve."
            def on_pick_card(action_id: int):
                hand_idx = action_id - SEL_HAND_START
                self._effect_dna_pick_first(player, hand_idx)
            self.request_selection(
                GamePhase.SelectTarget, player, on_pick_card, valid,
                is_optional, prompt=prompt)

    def _effect_dna_pick_first(self, player: "Player", hand_idx: int):
        """Effect DNA step 1: pick first field target."""
        if hand_idx >= len(player.hand_cards):
            return
        card = player.hand_cards[hand_idx]
        valid_first = get_valid_dna_first_targets(card, player.battle_area)
        if not valid_first:
            return

        valid = [SEL_MY_FIELD_START + i for i in valid_first]

        def on_first(action_id: int):
            first_field_idx = action_id - SEL_MY_FIELD_START
            self._effect_dna_pick_second(player, hand_idx, first_field_idx)

        self.request_selection(
            GamePhase.SelectMaterial, player, on_first, valid,
            is_optional=False,
            prompt="Select first Digimon for DNA digivolve.")

    def _effect_dna_pick_second(self, player: "Player", hand_idx: int,
                                 first_field_idx: int):
        """Effect DNA step 2: pick second field target."""
        if hand_idx >= len(player.hand_cards):
            return
        if first_field_idx >= len(player.battle_area):
            return
        card = player.hand_cards[hand_idx]
        valid_second = get_valid_dna_second_targets(
            card, first_field_idx, player.battle_area)
        if not valid_second:
            return

        valid = [SEL_MY_FIELD_START + i for i in valid_second]

        def on_second(action_id: int):
            second_field_idx = action_id - SEL_MY_FIELD_START
            self._effect_dna_execute(player, hand_idx, first_field_idx,
                                      second_field_idx)

        self.request_selection(
            GamePhase.SelectMaterial, player, on_second, valid,
            is_optional=False,
            prompt="Select second Digimon for DNA digivolve.")

    def _effect_dna_execute(self, player: "Player", hand_idx: int,
                             first_field_idx: int, second_field_idx: int):
        """Effect DNA step 3: execute the DNA digivolve."""
        if hand_idx >= len(player.hand_cards):
            return
        if first_field_idx >= len(player.battle_area):
            return
        if second_field_idx >= len(player.battle_area):
            return

        card = player.hand_cards[hand_idx]
        perm1 = player.battle_area[first_field_idx]
        perm2 = player.battle_area[second_field_idx]

        stacking = get_dna_stacking_order(card, perm1, perm2)
        if stacking is None:
            return

        top_perm, bottom_perm, dna_cost = stacking

        self.logger.log(
            f"[Effect DNA Digivolve] {self._card_ref(card)} from "
            f"{self._perm_ref(top_perm)} + {self._perm_ref(bottom_perm)} "
            f"(cost: {dna_cost.memory_cost})")

        cost = player.dna_digivolve(top_perm, bottom_perm, card, dna_cost)
        player.lose_memory(cost)

        new_perm = player.battle_area[-1] if player.battle_area else None
        self.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": new_perm, "is_dna_digivolve": True})

    def _initiate_dna_digivolve(self, hand_idx: int):
        """Start DNA digivolve: enter SelectMaterial to pick first field target."""
        if self.current_phase != GamePhase.Main:
            return
        if hand_idx >= len(self.turn_player.hand_cards):
            return

        card = self.turn_player.hand_cards[hand_idx]
        if not card.is_digimon or not card.c_entity_base or not card.c_entity_base.dna_costs:
            return

        valid_first = get_valid_dna_first_targets(card, self.turn_player.battle_area)
        if not valid_first:
            return

        self.request_selection(
            GamePhase.SelectMaterial,
            self.turn_player,
            lambda first_idx: self._dna_select_second(hand_idx, first_idx),
            valid_indices=valid_first,
        )

    def _dna_select_second(self, hand_idx: int, first_field_idx: int):
        """DNA digivolve step 2: select second field target."""
        if hand_idx >= len(self.turn_player.hand_cards):
            return
        if first_field_idx >= len(self.turn_player.battle_area):
            return

        card = self.turn_player.hand_cards[hand_idx]
        valid_second = get_valid_dna_second_targets(
            card, first_field_idx, self.turn_player.battle_area,
        )
        if not valid_second:
            return

        self.request_selection(
            GamePhase.SelectMaterial,
            self.turn_player,
            lambda second_idx: self._execute_dna_digivolve(
                hand_idx, first_field_idx, second_idx,
            ),
            valid_indices=valid_second,
        )

    def _execute_dna_digivolve(self, hand_idx: int, first_field_idx: int,
                                second_field_idx: int):
        """Execute the actual DNA digivolve after both targets are selected."""
        player = self.turn_player
        if hand_idx >= len(player.hand_cards):
            return
        if first_field_idx >= len(player.battle_area):
            return
        if second_field_idx >= len(player.battle_area):
            return

        card = player.hand_cards[hand_idx]
        perm1 = player.battle_area[first_field_idx]
        perm2 = player.battle_area[second_field_idx]

        stacking = get_dna_stacking_order(card, perm1, perm2)
        if stacking is None:
            return

        top_perm, bottom_perm, dna_cost = stacking

        self.logger.log(
            f"[DNA Digivolve] {self._card_ref(card)} from "
            f"{self._perm_ref(top_perm)} + {self._perm_ref(bottom_perm)} (cost: {dna_cost.memory_cost})"
        )

        cost = player.dna_digivolve(top_perm, bottom_perm, card, dna_cost)
        player.lose_memory(cost)

        new_perm = player.battle_area[-1] if player.battle_area else None

        self.execute_effects(EffectTiming.WhenDigivolving, {
            "digivolved_permanent": new_perm, "is_dna_digivolve": True})
        self.check_turn_end()

    # ─── DigiXros ────────────────────────────────────────────────────

    def _initiate_digixros_play(self, card_index: int):
        """Start DigiXros: store state and enter material selection loop."""
        player = self.turn_player
        card = player.hand_cards[card_index]
        xros_cost = card.digixros_cost
        if xros_cost is None:
            # Fallback: play normally
            self._execute_play_card(card)
            return

        self._pending_digixros = {
            'card_index': card_index,
            'card': card,
            'player': player,
            'materials': [],  # List of (source, zone, index) tuples
            'xros_cost': xros_cost,
        }

        self.logger.log(
            f"[DigiXros] {self._card_ref(card)} — entering material selection "
            f"(reduce {xros_cost.reduce_cost_per_card} per card)")

        self._digixros_select_next_material()

    def _digixros_select_next_material(self):
        """Offer optional material selection or finish."""
        pending = self._pending_digixros
        if pending is None:
            return

        player = pending['player']
        card = pending['card']
        xros_cost = pending['xros_cost']
        already = [m[0] for m in pending['materials']]

        valid_ids = get_valid_digixros_materials(card, player, already, xros_cost)

        if not valid_ids:
            # No more valid materials — finish
            self._execute_digixros_play()
            return

        self.request_selection(
            GamePhase.SelectMaterial,
            player,
            self._digixros_on_material_selected,
            valid_indices=valid_ids,
            is_optional=True,
            prompt=f"Select DigiXros material ({len(already)} selected so far).",
            on_decline=self._execute_digixros_play,
        )

    def _digixros_on_material_selected(self, action_id: int):
        """Record selected material and loop for more."""
        pending = self._pending_digixros
        if pending is None:
            return

        player = pending['player']

        # Decode which card was selected (using zone range constants)
        if SEL_HAND_START <= action_id <= SEL_HAND_START + MAX_HAND - 1:
            idx = action_id - SEL_HAND_START
            if idx < len(player.hand_cards):
                card = player.hand_cards[idx]
                pending['materials'].append((card, 'hand'))
                self.logger.log(f"[DigiXros] Selected material from hand: {self._card_ref(card)}")
        elif SEL_MY_FIELD_START <= action_id <= SEL_MY_FIELD_START + FIELD_SLOTS - 1:
            idx = action_id - SEL_MY_FIELD_START
            if idx < len(player.battle_area):
                perm = player.battle_area[idx]
                top = perm.top_card
                if top:
                    pending['materials'].append((top, 'field'))
                    self.logger.log(f"[DigiXros] Selected material from field: {self._card_ref(top)}")
        elif SEL_TRASH_START <= action_id <= SEL_TRASH_START + MAX_TRASH - 1:
            idx = action_id - SEL_TRASH_START
            if idx < len(player.trash_cards):
                card = player.trash_cards[idx]
                pending['materials'].append((card, 'trash'))
                self.logger.log(f"[DigiXros] Selected material from trash: {self._card_ref(card)}")

        # Loop for next material
        self._digixros_select_next_material()

    def _execute_digixros_play(self):
        """Stack materials under card, calculate reduced cost, play.

        Cannot delegate to _execute_play_card because DigiXros requires:
        1. Creating the permanent first
        2. Stacking materials under it (including consuming field permanents)
        3. Only then calculating and paying cost with the material reduction
        This ordering differs from normal play where cost is paid before placement.
        Keep in sync with _execute_play_card for effect firing (OnEnterFieldAnyone, etc).
        """
        pending = self._pending_digixros
        self._pending_digixros = None
        if pending is None:
            return

        player = pending['player']
        card = pending['card']
        xros_cost = pending['xros_cost']
        materials = pending['materials']
        digixros_count = len(materials)
        manual_reduction = digixros_count * xros_cost.reduce_cost_per_card

        self.logger.log(
            f"[DigiXros] Executing with {digixros_count} materials, "
            f"reduction={manual_reduction}")

        # 1. Remove card from hand and create permanent
        if card not in player.hand_cards:
            return
        player.hand_cards.remove(card)
        from ..core.permanent import Permanent
        new_perm = Permanent([card])
        if self:
            new_perm.turn_played = self.turn_count
            new_perm._owner_game = self
        player.battle_area.append(new_perm)

        # 2. Stack materials under the played card
        # Process in reverse to handle index shifts for same-zone removals
        # Collect field permanents to remove (need special handling)
        field_perms_to_remove = []
        hand_cards_to_remove = []
        trash_cards_to_remove = []

        for mat_card, zone in materials:
            if zone == 'hand':
                hand_cards_to_remove.append(mat_card)
            elif zone == 'field':
                # Find the permanent containing this top card
                for perm in player.battle_area:
                    if perm is not new_perm and perm.top_card is mat_card:
                        field_perms_to_remove.append(perm)
                        break
            elif zone == 'trash':
                trash_cards_to_remove.append(mat_card)

        # Remove hand materials and stack under
        for mat_card in hand_cards_to_remove:
            if mat_card in player.hand_cards:
                player.hand_cards.remove(mat_card)
                new_perm.add_card_source_bottom(mat_card)

        # Remove field permanents: per rules 7-2-2-7 and DCGO DiscardEvoRoots(),
        # only the TOP card is placed under the new permanent.
        # All digivolution cards (the stack below top) are trashed first.
        for perm in field_perms_to_remove:
            if perm in player.battle_area:
                top_card = perm.top_card
                # Trash digivolution cards (all sources except top card)
                digi_cards = [s for s in perm.card_sources if s is not top_card]
                player.battle_area.remove(perm)
                # Clean up modifiers for the removed permanent
                if hasattr(self, 'cleanup_modifiers_for_permanent'):
                    self.cleanup_modifiers_for_permanent(perm)
                # Trash the digi cards (ACE overflow applies)
                for src in digi_cards:
                    player.trash_cards.append(src)
                player._apply_ace_overflow(digi_cards)
                # Place only top card under the new permanent
                if top_card:
                    new_perm.add_card_source_bottom(top_card)
                # Fire WhenRemoveField with digixros cause
                self.execute_effects(
                    EffectTiming.WhenRemoveField,
                    {"permanent": perm, "player": player,
                     "removal_cause": "digixros"})

        # Remove trash materials and stack under
        for mat_card in trash_cards_to_remove:
            if mat_card in player.trash_cards:
                player.trash_cards.remove(mat_card)
                new_perm.add_card_source_bottom(mat_card)

        # 3. Calculate cost with manual reduction and pay
        cost = self.calculate_play_cost(player, card, commit=True,
                                         manual_reduction=manual_reduction)
        self.logger.log(
            f"[Play] {player.player_name} plays {self._card_ref(card)} "
            f"via DigiXros (cost: {cost}, materials: {digixros_count})")
        self._emit(
            'play_card',
            source_card_id=self._card_id(card),
            source_slot=self._perm_slot(new_perm),
            cost=cost,
            card_name=self._card_name(card),
        )
        player.lose_memory(cost)
        if hasattr(player, "_temp_play_cost_reduction"):
            player._temp_play_cost_reduction = 0

        # 4. Fire effects
        is_option = card.is_option
        if is_option:
            self.execute_effects(
                EffectTiming.OnUseOption,
                {"played_card": card, "played_permanent": new_perm,
                 "event_player": player},
            )
        ctx = {"played_card": card, "played_permanent": new_perm,
               "event_player": player}
        if digixros_count > 0:
            ctx["digixros_count"] = digixros_count
        self.execute_effects(EffectTiming.OnEnterFieldAnyone, ctx)
        self._fire_play_observers(new_perm, player)
        if is_option and not self._option_stays_on_field(card):
            self._trash_option_after_resolution(player, new_perm)
        self.check_turn_end()
