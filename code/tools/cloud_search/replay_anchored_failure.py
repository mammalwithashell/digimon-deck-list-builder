"""Replay one fixed-seed anchored evaluation game with inner-engine traces."""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path

import numpy as np
from digimon_engine import load_implemented_card_ids
from digimon_gym.agents.anchored_eval import _seat_balanced_seed, play_one
from digimon_gym.agents.gauntlet import GeneralistDeckPool
from digimon_gym.digimon_gym import DigimonEnv, greedy_policy
from tools.anchored_eval_cli import _load_candidate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', required=True)
    parser.add_argument('--snapshot', required=True)
    parser.add_argument('--base-seed', type=int, required=True)
    parser.add_argument('--game-index', type=int, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    pool = GeneralistDeckPool.from_snapshot(args.snapshot, implemented_card_ids=load_implemented_card_ids())
    rng = np.random.default_rng(args.base_seed)
    for _ in range(args.game_index + 1):
        first, second = pool.sample_uniform_archetype(rng), pool.sample_uniform_archetype(rng)
    model = _load_candidate(args.candidate, 'cpu')
    seed = _seat_balanced_seed(args.base_seed, args.game_index)
    states, steps = {}, []
    original = DigimonEnv.step

    def traced_step(env, action):
        ui = env.runner.to_ui_json()
        digest = hashlib.sha256(json.dumps(ui, sort_keys=True).encode()).hexdigest()
        if digest not in states:
            states[digest] = dict(ui=ui, first_step=len(steps), legal=np.flatnonzero(env.action_mask()).tolist())
        steps.append(dict(step=len(steps), action=int(action), player=env.current_player_id, state=digest))
        return original(env, action)

    DigimonEnv.step = traced_step
    result = dict(seed=seed, index=args.game_index, decks=[first.archetype_name, second.archetype_name],
                  deck_ids=[first.deck_id, second.deck_id], anchor='greedy',
                  candidate_sha256=hashlib.sha256(Path(args.candidate).read_bytes()).hexdigest())
    try:
        result['winner'] = play_one(model, greedy_policy, first.card_ids, second.card_ids, seed, 'standard_lite_v2')
        result['status'] = 'complete'
    except Exception as error:
        result.update(status='failed', error=str(error))
    finally:
        DigimonEnv.step = original
        counts = Counter(s['state'] for s in steps)
        result.update(steps=len(steps), unique_states=len(states),
                      repeated_states=[dict(hash=k, count=v, **states[k]) for k, v in counts.most_common(4)])
        (args.out / 'trace.json').write_text(json.dumps(dict(steps=steps, states=states)))
        (args.out / 'result.json').write_text(json.dumps(result, indent=2))
        print(json.dumps({k:v for k,v in result.items() if k != 'repeated_states'}), flush=True)


if __name__ == '__main__':
    main()
