"""Evaluation-only card telemetry. Never used to select actions or shape rewards.

Opportunities are decision exposures, deduplicated by (card id, route) within
one mask. Choices count selected actions, independently of resolved entries.
Selection prompts are excluded because their action ids reuse main ranges.
"""
from __future__ import annotations

from collections import Counter
import json
import logging
from pathlib import Path
import uuid

log = logging.getLogger(__name__)
SCHEMA_VERSION = 1
CARD_COUNTERS = ("plays", "plays_direct", "plays_effect", "hatches", "digivolves_battle",
                 "digivolves_raising", "dna_digivolves", "promotions", "attacks", "memory_paid")
EFFECT_COUNTERS = ("opportunities", "order_selected", "order_declined", "partner_accepted",
                   "partner_declined", "target_accepted", "target_declined", "dna_completed",
                   "attack_opportunities", "attack_accepted", "attack_declined", "attacks_declared")


def decision_routes(state: dict, ui: dict, mask) -> dict[int, tuple[str, str]]:
    """Decode only documented phase-scoped actions, using the legal mask."""
    if ui.get("pendingSelection"):
        return {}
    player = ui.get(f"player{state['current_player_id']}", {})
    hand = player.get("handIds", [])
    field = player.get("battleArea", [])
    breeding = player.get("breedingArea") or {}
    phase = state.get("phase")
    routes = {}
    for action, legal in enumerate(mask):
        if not legal:
            continue
        card, route = None, None
        if phase == "Main" and 0 <= action < 30 and action < len(hand):
            card, route = hand[action], "play"
        elif phase in ("Main", "EndOfTurnAction") and 63 <= action < 93 and action - 63 < len(hand):
            card, route = hand[action - 63], "dna_digivolve"
        elif phase in ("Main", "CounterTiming") and 400 <= action < 1000:
            hand_idx, slot = divmod(action - 400, 15)
            if hand_idx < len(hand):
                card = hand[hand_idx]
                route = "digivolve_raising" if slot == 14 else "digivolve_battle"
        elif phase == "Breeding" and action == 61:
            card, route = breeding.get("topCardId"), "promote"
        elif phase in ("Main", "EndOfTurnAction") and 100 <= action < 400:
            slot = (action - 100) // 15
            if slot < len(field):
                card, route = field[slot].get("topCardId"), "attack"
        if card and route:
            routes[action] = (card, route)
    return routes


class CardEvalCollector:
    """One checkpoint window; observes inner games, including opponent steps."""

    def __init__(self, step: int, window: int, card_lookup=None):
        self.identity = dict(schema_version=SCHEMA_VERSION, checkpoint_step=step,
                             eval_window=window, window_id=uuid.uuid4().hex)
        self.games: list[dict] = []
        self.current = None
        self.context = {}
        self.provenance = {}
        self.card_lookup = card_lookup
        self.effect_context_supported = False
        self._effect_actions = []
        self._effect_seen = set()

    def start_game(self, ui: dict, decks: tuple, seed):
        # A reset without a terminal is visible as incomplete, never a loss.
        if self.current is not None:
            self.finish_game(None, "reset_before_terminal")
        self._effect_seen = set()
        self._effect_actions = []
        self.current = dict(game_id=len(self.games), seed=seed, cards={}, routes={},
                            route_decisions={}, effect_sequences={},
                            context=dict(self.context), steps=0)
        for pid, deck in enumerate(decks, 1):
            for card, copies in Counter(deck).items():
                self.card(pid, card)["deck_copies"] = copies
        self.observe_hand(ui)

    def card(self, pid, card):
        key = (pid, card)
        cards = self.current["cards"]
        if key not in cards:
            data = self.card_lookup(card) if self.card_lookup is not None else None
            name = data.card_name if data is not None else card
            cards[key] = dict(player=pid, card_id=card, card_name=name,
                              deck_copies=0, seen_in_hand=False,
                              **dict.fromkeys(CARD_COUNTERS, 0), first_entry_turn=None)
        return cards[key]

    def observe_hand(self, ui):
        for pid in (1, 2):
            for card in ui.get(f"player{pid}", {}).get("handIds", []):
                self.card(pid, card)["seen_in_hand"] = True

    def before_step(self, state, ui, mask, action, effect_context=None):
        if self.current is None:
            return
        pid = state["current_player_id"]
        self.observe_hand(ui)
        routes = decision_routes(state, ui, mask)
        for route in {route for _, route in routes.values()}:
            row = self.current["route_decisions"].setdefault((pid, route), dict(
                player=pid, route=route, opportunities=0, choices=0))
            row["opportunities"] += 1
            row["choices"] += int(routes.get(int(action), (None, None))[1] == route)
        for card, route in set(routes.values()):
            key = (pid, card, route)
            row = self.current["routes"].setdefault(key, dict(
                player=pid, card_id=card, route=route, opportunities=0, choices=0))
            row["opportunities"] += 1
            if routes.get(int(action)) == (card, route):
                row["choices"] += 1
        self._effect_actions = []
        if effect_context and effect_context.get("schema_version") == 1:
            self.effect_context_supported = True
            for effect in effect_context.get("effects", []):
                if effect["card_id"] != "EX12-001":
                    continue
                owner = effect["player"]
                key = (owner, effect["card_id"])
                row = self.current["effect_sequences"].setdefault(key, dict(
                    player=owner, card_id=effect["card_id"], **dict.fromkeys(EFFECT_COUNTERS, 0)))
                sequence = (owner, effect["sequence"])
                stage = effect["stage"]
                def once(counter):
                    token = (*sequence, counter)
                    if token not in self._effect_seen:
                        row[counter] += 1
                        self._effect_seen.add(token)
                if stage in {"trigger_order", "dna_partner", "dna_result"}:
                    once("opportunities")
                elif stage == "followup_attack":
                    once("attack_opportunities")
                accepted = int(action) in effect["accept_actions"] and bool(mask[int(action)])
                declined = int(action) == 62 and bool(mask[62])
                prefix = {"trigger_order": "order", "dna_partner": "partner",
                          "dna_result": "target", "followup_attack": "attack"}.get(stage)
                if prefix and (accepted or declined):
                    counter = ("order_selected" if stage == "trigger_order" else prefix+"_accepted") if accepted else prefix+"_declined"
                    once(counter)
                if accepted and stage in {"dna_result", "followup_attack"}:
                    self._effect_actions.append((effect, row, sequence, int(action)))

    def after_step(self, state, ui, events, terminated, truncated):
        if self.current is None:
            return
        self.current["steps"] += 1
        self.observe_hand(ui)
        for effect, row, sequence, action in self._effect_actions:
            stage = effect["stage"]
            if stage == "dna_result":
                target = effect.get("result_cards", {}).get(str(action))
                completed = target is not None and any(e.get("type") in {"Digivolve", "EvalDigivolve"}
                    and e.get("player") == effect["player"] and e.get("source_card_id") == target
                    and (e.get("meta") or {}).get("was_dna") for e in events)
                counter = "dna_completed"
            else:
                completed = any(e.get("type") == "Attack" and e.get("player") == effect["player"]
                    and e.get("source_slot") == effect.get("attacker_slot") for e in events)
                counter = "attacks_declared"
            token = (*sequence, counter)
            if completed and token not in self._effect_seen:
                row[counter] += 1
                self._effect_seen.add(token)
        self._effect_actions = []
        for ev in events:
            kind = ev.get("type")
            card = ev.get("source_card_id")
            pid = ev.get("player")
            if not card or pid not in (1, 2):
                continue
            row = self.card(pid, card)
            row["card_name"] = ev.get("source_card_name") or row["card_name"]
            meta = ev.get("meta") or {}
            entry = False
            if kind == "Play":
                row["plays"] += 1
                row["plays_effect" if meta.get("effect_initiated") else "plays_direct"] += 1
                row["memory_paid"] += meta.get("cost_paid", 0)
                entry = True
            elif kind in ("Digivolve", "EvalDigivolve"):
                route = "digivolves_raising" if ev.get("source_slot") == 14 else "digivolves_battle"
                row[route] += 1
                row["dna_digivolves"] += int(bool(meta.get("was_dna")))
                row["memory_paid"] += meta.get("memory_paid", 0)
                entry = True
            elif kind == "Hatch":
                row["hatches"] += 1
                entry = True
            elif kind == "BreedingMove":
                row["promotions"] += 1
                entry = True
            elif kind == "Attack":
                row["attacks"] += 1
            if entry and row["first_entry_turn"] is None:
                row["first_entry_turn"] = state.get("turn_count")
        if terminated or truncated:
            self.finish_game(state.get("winner_id"), state.get("terminal_reason") or "truncated")

    def finish_game(self, winner, reason):
        if self.current is None:
            return
        game = self.current
        game.update(winner_id=winner, terminal_reason=reason)
        game["cards"] = list(game["cards"].values())
        game["routes"] = list(game["routes"].values())
        game["route_decisions"] = list(game["route_decisions"].values())
        game["effect_sequences"] = list(game["effect_sequences"].values())
        self.games.append(game)
        self.current = None

    def report(self):
        cards, routes = {}, {}
        decisions, effects = {}, {}
        for game in self.games:
            context = game["context"]
            scope = {name: context.get(name, "") for name in ("panel", "anchor", "evaluation_mode")}
            group = (context.get("agent_archetype", ""), context.get("opponent_archetype", ""), *scope.values())
            for name, target_rows, dimensions, counters in [
                    ("route_decisions", decisions, ("player", "route"), ("opportunities", "choices")),
                    ("effect_sequences", effects, ("player", "card_id"), EFFECT_COUNTERS)]:
                for row in game.get(name, []):
                    key = (*group, *(row[k] for k in dimensions))
                    target = target_rows.setdefault(key, dict(agent_archetype=group[0], opponent_archetype=group[1],
                        **scope, **{k: row[k] for k in dimensions}, **dict.fromkeys(counters, 0)))
                    for counter in counters:
                        target[counter] += row[counter]
            for row in game["cards"]:
                key = (*group, row["player"], row["card_id"])
                target = cards.setdefault(key, dict(agent_archetype=group[0], opponent_archetype=group[1],
                    **scope, player=row["player"], card_id=row["card_id"], card_name=row["card_name"], games=0,
                    wins=0, losses=0, draws=0, incomplete=0, games_seen_in_hand=0,
                    games_entered=0, completed_games_entered=0, wins_when_entered=0,
                    first_entry_turn_sum=0, **dict.fromkeys(CARD_COUNTERS, 0)))
                target["games"] += 1
                incomplete = game["terminal_reason"] in ("reset_before_terminal", "evaluation_end")
                outcome = "incomplete" if incomplete else "draws" if game["winner_id"] is None else "wins" if game["winner_id"] == row["player"] else "losses"
                target[outcome] += 1
                target["games_seen_in_hand"] += int(row["seen_in_hand"])
                entered = row["first_entry_turn"] is not None
                target["games_entered"] += int(entered)
                target["completed_games_entered"] += int(entered and not incomplete)
                target["wins_when_entered"] += int(entered and outcome == "wins")
                target["first_entry_turn_sum"] += row["first_entry_turn"] or 0
                for k in CARD_COUNTERS:
                    target[k] += row[k]
                if row["card_name"] != row["card_id"]:
                    target["card_name"] = row["card_name"]
            for row in game["routes"]:
                key = (*group, row["player"], row["card_id"], row["route"])
                target = routes.setdefault(key, dict(agent_archetype=group[0], opponent_archetype=group[1],
                    **scope,
                    **{k: row[k] for k in ("player", "card_id", "route")}, opportunities=0, choices=0))
                target["opportunities"] += row["opportunities"]
                target["choices"] += row["choices"]
        return dict(**self.identity, provenance=self.provenance,
                    games=self.games, cards=list(cards.values()), routes=list(routes.values()),
                    route_decisions=list(decisions.values()), effect_sequences=list(effects.values()),
                    coverage={"opportunities": "main play/digivolve/DNA/attack, counter digivolve, breeding promote; pending selections excluded",
                              "route_decisions": "one opportunity per player/route/decision, regardless of distinct legal cards",
                              "effect_sequences": "EX12-001 legal DNA trigger-order/partner/result and follow-up attack prompts; per physical egg per global turn" if self.effect_context_supported else "unmeasured",
                              "entries": "resolved Play, Digivolve, EvalDigivolve, Hatch and BreedingMove events; promotion counts top card only",
                              "hand": "seen at decision boundaries; not a count of draws",
                              "turn": "engine global turn count at end of resolving step"})


def save_report(report: dict, directory: Path) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"checkpoint_{report['checkpoint_step']}_{report['window_id']}.json"
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    temp.replace(path)
    return path


def publish_report(report: dict, path: Path, run=None, *, step=None, wait=False,
                   skip_artifact=False, on_artifact_logged=None):
    """Version each checkpoint artifact, and log queryable card/route tables.

    PPO leaves the W&B step implicit because TensorBoard sync owns that clock.
    A standalone monitor can supply its own step and wait for artifact upload.
    Returns True on publication; failure retains the local report for retry.
    A caller retrying after the artifact was already logged passes
    ``skip_artifact`` (``on_artifact_logged`` reports that point) so a retry
    never creates a second artifact version for the same report.
    """
    try:
        import wandb
    except ImportError:
        return
    try:
        run = run if run is not None else wandb.run
        if run is None:
            return
        artifact = wandb.Artifact(f"card-eval-{run.id}", type="card-evaluation", metadata=report_header(report))
        artifact.add_file(str(path), name="report.json")
        run.define_metric("card_eval/*", step_metric="card_eval/checkpoint_step")
        payload = {"card_eval/checkpoint_step": report["checkpoint_step"]}
        for key in ("cards", "routes", "route_decisions", "effect_sequences"):
            rows = report.get(key, [])
            # Stay under W&B's default per-table row limit. The artifact's
            # report.json is always complete, regardless of the number of parts.
            for offset in range(0, len(rows), 10000):
                part = rows[offset:offset + 10000]
                name = key if len(rows) <= 10000 else f"{key}_{offset // 10000}"
                provenance = report.get("provenance", {})
                identity = dict(checkpoint_step=report["checkpoint_step"], eval_window=report["eval_window"],
                    window_id=report["window_id"], candidate_sha256=provenance.get("candidate_sha256", ""),
                    evaluation_status=provenance.get("status", "legacy"),
                    training_games_collected=provenance.get("training_games_collected"))
                columns = [*identity, *rows[0].keys()]
                table = wandb.Table(columns=columns, data=[
                    [*identity.values(), *r.values()] for r in part])
                artifact.add(table, name)
                payload[f"card_eval/{name}"] = table
                if provenance.get("panel"):
                    payload[f"card_eval/{provenance['panel']}/{name}"] = table
        if not skip_artifact:
            logged = run.log_artifact(artifact, aliases=["latest", f"step-{report['checkpoint_step']}-{report['window_id']}"])
            if wait:
                logged.wait()
            if on_artifact_logged is not None:
                on_artifact_logged()
        run.log(payload, **({"step": step} if step is not None else {}))
        return True
    except Exception:
        log.warning("W&B card evaluation upload failed; local report retained at %s", path, exc_info=True)
        return False


def report_header(report):
    return {**{k: report[k] for k in ("schema_version", "checkpoint_step", "eval_window", "window_id")},
            "provenance": report.get("provenance", {})}


def load_local_reports(directory):
    reports = [json.loads(p.read_text(encoding="utf-8")) for p in Path(directory).glob("checkpoint_*.json")]
    return sorted(reports, key=lambda r: (r["checkpoint_step"], r["eval_window"], r["window_id"]))


def load_wandb_reports(run_path, api=None):
    """Read all checkpoint artifact versions for entity/project/run_id."""
    if api is None:
        import wandb
        api = wandb.Api()
    reports = []
    for artifact in api.run(run_path).logged_artifacts():
        if artifact.type == "card-evaluation":
            local = artifact.get_path("report.json").download()
            reports.append(json.loads(Path(local).read_text(encoding="utf-8")))
    return sorted(reports, key=lambda r: (r["checkpoint_step"], r["eval_window"], r["window_id"]))
