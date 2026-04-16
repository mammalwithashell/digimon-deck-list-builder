from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_072(CardScript):
    """BT15-072 Vilemon | Lv.4 Purple (Evil) | 4000 DP

    <Blocker>
    [All Turns] When one of your [Apocalymon] or Digimon with the [Dark Masters]
    trait would leave the battle area other than by one of your effects, by
    deleting this Digimon, prevent 1 of those Digimon from leaving.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: <Blocker> ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-072 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [All Turns] Protect Apocalymon / Dark Masters from leaving ---
        # Uses WhenPermanentWouldBeDeleted timing + _will_not_be_removed flag
        # Pattern: BT24-012 Dimetromon (protect other Digimon by trait/name)
        # C# source: BT15_072.cs — WhenRemoveField timing with
        #   CanTriggerWhenPermanentRemoveField + PermanentCondition
        #   Cost: delete this Digimon. Success: willBeRemoveField = false
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect1.set_effect_name("BT15-072 Delete this Digimon to prevent 1 other Digimon from leaving Battle Area")
        effect1.set_effect_description(
            "[All Turns] When one of your [Apocalymon] or Digimon with the "
            "[Dark Masters] trait would leave the battle area other than by "
            "one of your effects, by deleting this Digimon, prevent 1 of "
            "those Digimon from leaving play."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Substitute_BT15_072")

        def _is_apocalymon_or_dark_masters(perm) -> bool:
            """Check if a permanent is [Apocalymon] by name or has the [Dark Masters] trait."""
            if not perm or not perm.top_card:
                return False
            # Check name: [Apocalymon]
            if perm.contains_card_name('Apocalymon'):
                return True
            # Check trait: [Dark Masters] (C# also checks "DarkMasters" variant)
            if perm.has_trait('Dark Masters') or perm.has_trait('DarkMasters'):
                return True
            return False

        def condition1(context: Dict[str, Any]) -> bool:
            # Vilemon must be on the field
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()

            # Must be triggered by opponent's effect (not own)
            if not context.get('is_opponent_effect', False):
                return False

            # The leaving permanent (mapped to event_permanent by execute_effects)
            leaving_perm = context.get('event_permanent')
            if leaving_perm is None:
                return False

            # The leaving permanent must belong to Vilemon's owner
            owner = getattr(card, 'owner', None)
            event_player = context.get('event_player')
            if not owner or not event_player or event_player is not owner:
                return False

            # The leaving permanent must NOT be Vilemon itself
            if leaving_perm is my_perm:
                return False

            # The leaving permanent must be Apocalymon or have Dark Masters trait
            if not _is_apocalymon_or_dark_masters(leaving_perm):
                return False

            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Delete Vilemon to prevent 1 Apocalymon / Dark Masters from leaving."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            vilemon_perm = card.permanent_of_this_card() if card else None
            if not vilemon_perm:
                return

            # Cost: delete Vilemon (this Digimon)
            # Use delete_permanent so all deletion effects fire normally
            player.delete_permanent(vilemon_perm, removal_cause='cost')

            # Prevent the target from being removed
            leaving_perm = ctx.get('event_permanent')
            if leaving_perm:
                leaving_perm._will_not_be_removed = True
                game.logger.log(
                    "[BT15-072] Vilemon deleted to prevent "
                    f"{game._perm_ref(leaving_perm) if hasattr(game, '_perm_ref') else 'ally'} "
                    "from leaving the battle area."
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
