"""Opt-in, isolated teacher comparison. Never mutates a running campaign.

Collection must finish without crashes/caps before a fitting manifest exists.
Both pure controls use identical decks, game seeds, and rows per player-game.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import random
import time

TEACHERS = ("greedy", "development_v1")
ARMS = ("greedy", "development", "mixed", "mixed_v2")
PROFILE = "standard_lite_v2"
# mixed_v2 additions. Random seats broaden value coverage only: their rows are
# saved with policy_weight 0, so imitation never learns to play randomly.
RANDOM_TEACHER = "random_v1"
TAMER_FIRST = {"greedy": "greedy_tamer_first_v1", "development_v1": "development_tamer_first_v1"}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def make_schedule(archetypes, *, seed, arm="mixed", rounds=32, tamer_first_archetypes=(),
                  random_fraction=0.0):
    """Every round has each archetype once in each seat; 8-round blocks balance
    teacher pairings and opening seats independently. No game RNG is consumed.
    Input maps archetype -> distinct stable deck IDs.

    ``mixed_v2`` keeps the mixed greedy/development pairing, then (a) swaps a
    seat's teacher for its Tamer-first variant when the seat's archetype is in
    ``tamer_first_archetypes`` and (b) independently replaces a seat with the
    uniform-random teacher with probability ``random_fraction``. Both are
    deterministic in (seed, game, seat), so the schedule stays reproducible.
    """
    if arm != "mixed_v2" and (tamer_first_archetypes or random_fraction):
        raise ValueError("Tamer-first and random seats are mixed_v2 options")
    if not 0.0 <= random_fraction < 1.0:
        raise ValueError("random_fraction must be in [0, 1)")
    unknown = set(tamer_first_archetypes) - set(archetypes)
    if unknown:
        raise ValueError(f"Tamer-first archetypes not in the pool: {sorted(unknown)}")
    if arm not in ARMS or rounds < 8 or rounds % 8:
        raise ValueError("expected known arm and a positive multiple of eight rounds")
    names = sorted(archetypes)
    if not names or any(not archetypes[n] for n in names):
        raise ValueError("nonempty archetypes and lists required")
    assignment = random.Random(seed ^ 0xD3E10)
    assignment.shuffle(names)
    pairings = ((0, 0), (1, 1), (0, 1), (1, 0))
    schedule = []
    for round_id in range(rounds):
        offset = 1 + (round_id // 8) % max(1, len(names) - 1)
        teachers = (pairings[round_id % 4] if arm in ("mixed", "mixed_v2")
                    else ((0, 0) if arm == "greedy" else (1, 1)))
        for index, first in enumerate(names):
            second = names[(index + offset) % len(names)]
            game_id = len(schedule)
            seat_teachers = [TEACHERS[t] for t in teachers]
            if arm == "mixed_v2":
                for seat, archetype in enumerate((first, second)):
                    if random.Random(f"mixed_v2:{seed}:{game_id}:{seat}").random() < random_fraction:
                        seat_teachers[seat] = RANDOM_TEACHER
                    elif archetype in tamer_first_archetypes:
                        seat_teachers[seat] = TAMER_FIRST[seat_teachers[seat]]
            schedule.append(dict(game_id=game_id, seed=2 * (seed + game_id) + (round_id // 4) % 2,
                opening_seat=(round_id // 4) % 2, archetypes=[first, second],
                decks=[sorted(set(archetypes[first]))[(round_id + round_id // 8) % len(set(archetypes[first]))],
                       sorted(set(archetypes[second]))[(round_id + round_id // 8 + 1) % len(set(archetypes[second]))]],
                teachers=seat_teachers))
    return schedule


def coverage(schedule):
    counts = Counter((g["archetypes"][seat], g["teachers"][seat], seat, int(g["opening_seat"] == seat))
                     for g in schedule for seat in (0, 1))
    return [dict(archetype=a, teacher=t, seat=s, opened=bool(o), appearances=n)
            for (a, t, s, o), n in sorted(counts.items())]


def grouped_decks(snapshot):
    """Consume the canonical GeneralistDeckPool snapshot, not slot-name guesses."""
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    import digimon_engine
    pool = GeneralistDeckPool.from_snapshot(snapshot,
        implemented_card_ids=set(digimon_engine.load_implemented_card_ids()))
    groups, cards = {}, {}
    for archetype, entries in pool.archetypes.items():
        groups[archetype] = sorted({entry.deck_id for entry in entries})
        for entry in entries:
            cards[entry.deck_id] = list(entry.card_ids)
    return groups, cards


def balanced_indices(movers, rows_per_seat, seed):
    """Equal mass per player-game, then uniform decisions within that seat.
    Replacement is explicit; duplicate rows retain original whole-game IDs.
    """
    import numpy as np
    if rows_per_seat <= 0:
        raise ValueError("positive rows_per_seat required")
    rng = np.random.default_rng(seed)
    result = []
    for seat in (1, 2):
        indices = np.flatnonzero(np.asarray(movers) == seat)
        if not len(indices):
            raise ValueError(f"seat {seat} has no decision rows; coverage qualification failed")
        result.extend(rng.choice(indices, size=rows_per_seat, replace=True).tolist())
    return result


def save_shard(path, rows, *, game_id, winner, chosen=None, value_only_movers=()):
    """``value_only_movers`` (1/2) are seats whose rows train the value head
    only (policy_weight 0). Legacy corpora pass none and stay byte-identical."""
    import numpy as np
    selected = rows if chosen is None else [rows[i] for i in chosen]
    path.mkdir(parents=True, exist_ok=False)
    actions, visits, offsets = [], [], [0]
    for row in selected:
        actions.extend(row[1])
        visits.extend([float(a == row[2]) for a in row[1]])
        offsets.append(len(actions))
    arrays = dict(obs=np.asarray([r[0] for r in selected], np.float32),
        policy_actions=np.asarray(actions, np.int32), policy_visits=np.asarray(visits, np.float32),
        policy_offsets=np.asarray(offsets, np.int64),
        z=np.asarray([0.0 if winner in (None, 0) else (1.0 if r[3] == winner else -1.0) for r in selected], np.float32),
        terminated=np.ones(len(selected), np.uint8), mover=np.asarray([r[3] for r in selected], np.uint8),
        game_id=np.full(len(selected), game_id, np.int32))
    if value_only_movers:
        arrays["policy_weight"] = np.asarray([0.0 if r[3] in value_only_movers else 1.0 for r in selected],
                                             np.float32)
    for name, array in arrays.items():
        np.save(path / f"{name}.npy", array)
    return dict(dir=path.name, rows=len(selected), sha256={p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                for p in sorted(path.glob("*.npy"))})


def collect(snapshot, output, *, arm, seed, rounds=32, rows_per_seat=64, decision_cap=1200,
            tamer_first_archetypes=(), random_fraction=0.0):
    import numpy as np
    import digimon_engine
    from model_contract import current_contract
    from digimon_gym.agents.card_eval import CardEvalCollector
    groups, cards = grouped_decks(snapshot)
    database_path = Path(os.environ.get("DIGIMON_CARDS_JSON", Path(__file__).resolve().parents[3] / "data/cards.json"))
    database = json.loads(database_path.read_text(encoding="utf-8"))
    dna_lists = {deck: {cid: count for cid, count in Counter(ids).items() if database[cid].get("dna_costs")}
                 for deck, ids in cards.items()}
    schedule = make_schedule(groups, seed=seed, arm=arm, rounds=rounds,
                             tamer_first_archetypes=tuple(tamer_first_archetypes), random_fraction=random_fraction)
    output.mkdir(parents=True, exist_ok=False)
    contract = current_contract(PROFILE)
    provenance = dict(arm=arm, seed=seed, schedule=schedule, coverage=coverage(schedule),
        schedule_sha256=digest(schedule), decks_sha256=digest(cards),
        contract=contract, rows_per_player_game=rows_per_seat,
        row_sampling="uniform with replacement within each player-game; equal rows per player-game",
        teachers=list(TEACHERS) if arm != "mixed_v2" else sorted({t for g in schedule for t in g["teachers"]}),
        tensor_action_contract_changed=False)
    if arm == "mixed_v2":
        provenance["mixed_v2"] = dict(tamer_first_archetypes=sorted(tamer_first_archetypes),
            random_fraction=random_fraction, random_rows="policy_weight 0 (value targets only)",
            random_rng="numpy default_rng([seed, game_id]) uniform over legal actions")
    provenance["dna_capable_lists"] = {deck: targets for deck, targets in dna_lists.items() if targets}
    provenance["dna_scope"] = "Printed DNA targets by exact deck list; effect-only DNA remains a separate route"
    provenance["cards_sha256"] = hashlib.sha256(database_path.read_bytes()).hexdigest()
    write_json(output / "provenance.json", provenance)
    raw, balanced, reports = [], [], []
    telemetry = CardEvalCollector(step=0, window=0)
    started = time.monotonic()
    for entry in schedule:
        game_id = entry["game_id"]
        game = digimon_engine.RustHeadlessGame(*(cards[d] for d in entry["decks"]),
            seed=entry["seed"], observation_profile=PROFILE, record_actions=True)
        rows, diagnostics = [], []
        random_rng = np.random.default_rng([seed, game_id])
        telemetry.context = dict(agent_archetype=entry["archetypes"][0], opponent_archetype=entry["archetypes"][1],
            panel="bootstrap", anchor=arm, evaluation_mode="teacher", teachers=entry["teachers"],
            agent_deck_id=entry["decks"][0], opponent_deck_id=entry["decks"][1],
            dna_targets=[dna_lists[d] for d in entry["decks"]])
        ui = game.get_card_eval_state()
        telemetry.start_game(ui, tuple(cards[d] for d in entry["decks"]), entry["seed"])
        events_log, hand_changes = [], []
        try:
            for step in range(decision_cap):
                if game.is_game_over:
                    break
                actor = game.current_player_id
                mask = np.asarray(game.get_action_mask())
                legal = np.flatnonzero(mask > .5).tolist()
                if entry["teachers"][actor - 1] == RANDOM_TEACHER:
                    decision = dict(version=RANDOM_TEACHER, player=actor - 1,
                                    action=int(random_rng.choice(legal)))
                else:
                    decision = json.loads(game.teacher_decision(entry["teachers"][actor - 1]))
                action = int(decision["action"])
                if decision["player"] != actor - 1 or action not in legal:
                    raise RuntimeError("teacher selected illegal action or wrong decision player")
                diagnostics.append(decision)
                if len(legal) > 1:
                    rows.append((np.asarray(game.get_board_tensor(actor), np.float32).copy(), legal, action, actor))
                before = game.get_rl_state()
                telemetry.before_step(before, ui, mask, action)
                game.step(action)
                after_ui, after = game.get_card_eval_state(), game.get_rl_state()
                events = game.get_events_since_last_step()
                events_log.extend(events)
                telemetry.after_step(after, after_ui, events, game.is_game_over, False)
                hand_changes.append(dict(step=step, actor=actor, action=action,
                    net=[len(after_ui.get(f"player{p}", {}).get("handIds", [])) - len(ui.get(f"player{p}", {}).get("handIds", [])) for p in (1, 2)]))
                ui = after_ui
            state = game.get_rl_state()
            if not game.is_game_over or state.get("terminal_reason") in ("step_limit", "crash"):
                raise RuntimeError("qualification game hit decision cap or engine truncation")
            indices = balanced_indices([r[3] for r in rows], rows_per_seat, seed + game_id)
        except BaseException as error:
            write_json(output / "failure.json", dict(game=entry, error=repr(error),
                recording=game.get_recording(), decisions=diagnostics))
            # No manifest exists, so neither policy nor value trainer can ingest partial games.
            raise
        shard = f"game-{game_id:05d}"
        value_only = tuple(seat + 1 for seat in (0, 1) if entry["teachers"][seat] == RANDOM_TEACHER)
        raw.append(save_shard(output / "raw" / shard, rows, game_id=game_id, winner=game.winner_id,
                              value_only_movers=value_only))
        balanced.append(save_shard(output / "bootstrap_shards" / shard, rows,
            game_id=game_id, winner=game.winner_id, chosen=indices, value_only_movers=value_only))
        write_json(output / "decisions" / f"{shard}.json", diagnostics)
        write_json(output / "events" / f"{shard}.json", dict(events=events_log, hand_changes=hand_changes,
            resource_route_attribution="Unmeasured where native events do not identify draw/search/retrieval; net change is per action, not gross draws"))
        reports.append(dict(game_id=game_id, decisions=len(diagnostics), raw_rows=len(rows),
            sampled_rows=len(indices), sampled_unique_rows=len(set(indices)), terminal=state))
        write_json(output / "progress.json", dict(completed=len(reports), requested=len(schedule),
            capped=0, elapsed_seconds=time.monotonic()-started))
    for name, shards in (("raw", raw), ("bootstrap_shards", balanced)):
        write_json(output / name / "manifest.json", dict(schema="selfplay-shards-v1",
            observation_profile=PROFILE, model_contract=contract, obs_dim=contract["tensor_size"],
            action_dim=contract["action_space_size"], value_target="terminal_outcome",
            policy_semantics="teacher demonstrations; NEVER pool into search replay",
            games=len(schedule), truncated_games=0, shards=shards,
            bootstrap_provenance_sha256=digest(provenance)))
    write_json(output / "qualification.json", dict(qualified=True, games=reports,
        elapsed_seconds=time.monotonic()-started, coverage=coverage(schedule)))
    write_json(output / "card-evaluation.json", telemetry.report())


def fit(output, seed):
    from tools.english_meta_search_job import bootstrap
    qualification = json.loads((output / "qualification.json").read_text())
    manifest = json.loads((output / "bootstrap_shards/manifest.json").read_text())
    if not qualification.get("qualified") or manifest["truncated_games"] or (output / "failure.json").exists():
        raise ValueError("unqualified corpus; fitting refused")
    provenance = json.loads((output / "provenance.json").read_text())
    if digest(provenance) != manifest.get("bootstrap_provenance_sha256") or seed != provenance["seed"]:
        raise ValueError("bootstrap provenance/initialization seed mismatch")
    if len(qualification["games"]) != len(provenance["schedule"]) or manifest["games"] != len(provenance["schedule"]):
        raise ValueError("incomplete qualification schedule")
    for shard in manifest["shards"]:
        for name, expected in shard["sha256"].items():
            if hashlib.sha256((output / "bootstrap_shards" / shard["dir"] / name).read_bytes()).hexdigest() != expected:
                raise ValueError("bootstrap shard hash mismatch")
    bootstrap(output, games=manifest["games"], seed=seed, device="cpu", teacher_label=provenance["arm"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("collect", "fit"))
    parser.add_argument("--snapshot", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--arm", choices=ARMS, default="mixed")
    parser.add_argument("--seed", type=int, default=60261025)
    parser.add_argument("--rounds", type=int, default=32)
    parser.add_argument("--tamer-first-archetype", action="append", default=[],
                        help="mixed_v2: archetype whose seats use the Tamer-first teacher variant (repeatable)")
    parser.add_argument("--random-fraction", type=float, default=0.0,
                        help="mixed_v2: share of seats played by the uniform-random teacher (value targets only)")
    args = parser.parse_args()
    if args.phase == "collect":
        if args.snapshot is None:
            parser.error("collect requires --snapshot")
        collect(args.snapshot, args.output, arm=args.arm, seed=args.seed, rounds=args.rounds,
                tamer_first_archetypes=args.tamer_first_archetype, random_fraction=args.random_fraction)
    else:
        fit(args.output, args.seed)


if __name__ == "__main__":
    main()
