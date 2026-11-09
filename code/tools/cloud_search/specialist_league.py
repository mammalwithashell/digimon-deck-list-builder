"""Specialist league: one search model per archetype (2026-11-08 plan).

Spec: docs/superpowers/plans/2026-11-08-specialist-league.md.

Every specialist warm-starts from one base generalist and only ever pilots its
own archetype's lists (engine specialist mode: ``learner_decks`` in the
opponent pool). The other seat never pilots that archetype, so there are no
mirrors. Generations are synchronous: generation g of every specialist trains
against frozen opponents from generation g-1.

  gen 1   the base pilots every opponent deck (the current model on both
          seats, learner rows only) + CP8 piloting the original campaign
          archetypes;
  gen 2+  every other archetype's champion piloting its own lists, plus the
          base generalist and CP8 as frozen anchors on every generation.

Champion: without ``plan['gate']`` (the original plan) the latest
generation. With it (``enable-gate``, 2026-11-09) a paired promotion gate:
after generation g trains, the new model and the current champion each pilot
the specialist's lists in the same seeded games against the frozen base model
on every other archetype (no root noise, no temperature, fresh seeds each
generation); the new model becomes champion only if it scores strictly
higher. Opponent pools and the next warm start use champions.

Subcommands:
  build-decks  local: league deck pool from the campaign pool + DigiLab roster,
               every card admissible under the training gate;
  prepare      in the image: per-specialist run folders, hashes, health check;
  run          in the image: the generation loop (resumable at specialist
               granularity; a partial specialist generation needs recovery);
  enable-gate  in the image: add the promotion gate to a prepared league
               (gates already-trained generations in order, then continues);
  enable-remote-gates  in the image: let helper hosts run gates (claims under
               gates/claims; the controller trains, settles each generation's
               gates after training it, and runs any gate no helper claimed);
  gate-one     in the image, on a helper: run one claimed gate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

SCHEMA = 'specialist-league-v1'
# DigiLab roster archetype -> campaign archetype name (same deck family).
ROSTER_ALIASES = {'Beelstarmon (3M)': 'Three Musketeers', 'Jupitermon': 'TS Jupitermon',
                  'Medusamon/Styracomon': 'Medusamon'}
MODEL_SUFFIXES = ('.zip', '.onnx', '.onnx.meta.json')
CLI_BIN = '/usr/local/bin/digimon-engine-cli'
CARDS_JSON = '/app/data/cards.json'
GATE_SCHEMA = 'specialist-gate-v1'


def archetype(name: str) -> str:
    return name.split(':sha256:', 1)[0]


def slug(name: str) -> str:
    return ''.join(c.lower() if c.isalnum() else '-' for c in name).strip('-').replace('--', '-')


def sha256(path: Path) -> str:
    from tools.selfplay_scheduler import sha256 as digest
    return digest(path)


def build_decks(campaign: list[dict], roster: dict, admissible: dict, extra: list[str]) -> list[dict]:
    """Campaign archetypes keep their lists and gain their roster lists;
    ``extra`` roster archetypes join with their roster lists. Every card must
    be admissible; duplicates (same multiset) are dropped."""
    seen, out = set(), []

    def add(arch, cards, source):
        bad = sorted({c for c in cards if not admissible.get(c, {}).get('admissible')})
        if bad:
            raise ValueError(f'{arch} {source}: not admissible {bad}')
        key = hashlib.sha256('\n'.join(sorted(cards)).encode()).hexdigest()
        if key in seen:
            return
        seen.add(key)
        out.append(dict(name=f'{arch}:sha256:{key}:{source}', cards=list(cards)))

    campaign_archetypes = sorted({archetype(d['name']) for d in campaign})
    for index, deck in enumerate(campaign):
        add(archetype(deck['name']), deck['cards'], f'campaign{index}')
    wanted = set(campaign_archetypes) | set(extra)
    for raw, lists in roster.items():
        arch = ROSTER_ALIASES.get(raw, raw)
        if arch not in wanted:
            continue
        for row in lists:
            if row['blockers'] or any(not admissible.get(c, {}).get('admissible') for c in row['cards']):
                print(f'skip {arch} list {row["result_id"]}: not admissible', file=sys.stderr)
                continue
            add(arch, row['cards'], f'digilab{row["result_id"]}')
    missing = wanted - {archetype(d['name']) for d in out}
    if missing:
        raise ValueError(f'No lists for {sorted(missing)}')
    return out


def specialist_dir(root: Path, arch: str) -> Path:
    return root / 'specialists' / slug(arch) / 'generation'


def opponent_pool(root: Path, plan: dict, arch: str, gen: int) -> dict:
    from tools.search_opponents import SCHEMA as POOL_SCHEMA, checkpoint_entry
    inputs = root / 'inputs'
    anchors = [a for a in plan['cp8_archetypes'] if a != arch]
    cp8 = dict(checkpoint_entry('cp8', inputs / 'opponents/cp8.onnx'), decks=anchors)
    if gen == 1:
        mix = plan['mixture']['gen1']
        cp8['weight'] = 1.0
        return dict(schema=POOL_SCHEMA, selfplay_probability=mix['base_selfplay'],
                    learner_decks=[arch], opponents=[cp8])
    mix = plan['mixture']['later']
    others = [a for a in plan['archetypes'] if a != arch]
    if plan.get('gate'):
        return gated_pool(root, arch, gen, cp8, others, mix)
    entries = []
    for other in others:
        model = specialist_dir(root, other) / f'gen{gen - 1}' / 'model.onnx'
        entries.append(dict(checkpoint_entry(f'champion-{slug(other)}-gen{gen - 1}', model,
                                             weight=mix['champions'] / len(others)), decks=[other]))
    entries.append(checkpoint_entry('base', inputs / 'opponents/base.onnx', weight=mix['base']))
    entries.append(dict(cp8, weight=mix['cp8']))
    return dict(schema=POOL_SCHEMA, selfplay_probability=0.0, learner_decks=[arch], opponents=entries)


def gate_path(root: Path, arch: str, gen: int) -> Path:
    return root / 'gates' / f'gen{gen}' / f'{slug(arch)}.json'


def champion(root: Path, arch: str, gen: int) -> dict:
    """The specialist's champion after generation ``gen`` (gen 0 = the base)."""
    if gen == 0:
        gen0 = specialist_dir(root, arch) / 'gen0'
        return dict(generation=0, onnx=str(gen0 / 'model.onnx'), zip=str(gen0 / 'model.zip'))
    record = gate_path(root, arch, gen)
    if not record.exists():
        raise RuntimeError(f'{arch} generation {gen} has no gate record')
    return json.loads(record.read_text())['champion_after']


def gated_pool(root: Path, arch: str, gen: int, cp8: dict, others: list, mix: dict) -> dict:
    """Generation ``gen`` trains against every other archetype's champion
    after ``gen - 1``. A champion that is still the base model merges into
    the base anchor (one model, one entry, decks of every other archetype);
    when the learner itself is still the base, the base share becomes
    self-play (the same model on the other seat)."""
    from tools.search_opponents import SCHEMA as POOL_SCHEMA, checkpoint_entry
    base_hash = sha256(root / 'inputs/opponents/base.onnx')
    current_is_base = sha256(Path(champion(root, arch, gen - 1)['onnx'])) == base_hash
    entries, base_weight = [], mix['base']
    for other in others:
        champ = champion(root, other, gen - 1)
        weight = mix['champions'] / len(others)
        if sha256(Path(champ['onnx'])) == base_hash:
            base_weight += weight
            continue
        entries.append(dict(checkpoint_entry(f'champion-{slug(other)}-gen{champ["generation"]}',
                                             Path(champ['onnx']), weight=weight), decks=[other]))
    selfplay = 0.0
    if current_is_base:
        selfplay = base_weight
    else:
        entries.append(checkpoint_entry('base', root / 'inputs/opponents/base.onnx', weight=base_weight))
    entries.append(dict(cp8, weight=mix['cp8']))
    rest = sum(e['weight'] for e in entries)
    for entry in entries:  # weights are relative within the historical share
        entry['weight'] = entry['weight'] / rest
    return dict(schema=POOL_SCHEMA, selfplay_probability=selfplay, learner_decks=[arch], opponents=entries)


def gate_pool(root: Path, arch: str, model: Path) -> dict:
    """The gate field: the frozen base model piloting every other archetype.
    The base itself plays it as self-play: identical games, because the pair
    seed picks decks the same way and both seats search with the same model."""
    from tools.search_opponents import SCHEMA as POOL_SCHEMA, checkpoint_entry
    base = root / 'inputs/opponents/base.onnx'
    if sha256(model) == sha256(base):
        unused = checkpoint_entry('unused-cp8', root / 'inputs/opponents/cp8.onnx')
        return dict(schema=POOL_SCHEMA, selfplay_probability=1.0, learner_decks=[arch], opponents=[unused])
    return dict(schema=POOL_SCHEMA, selfplay_probability=0.0, learner_decks=[arch],
                opponents=[checkpoint_entry('base', base)])


def gate_scores(dirs: list) -> dict:
    """Learner score per game id (win 1, loss 0, no winner 0.5) with its decks."""
    out = {}
    for directory in dirs:
        manifest = json.loads((Path(directory) / 'manifest.json').read_text())
        names = manifest['deck_names']
        for pair, game in zip(manifest['game_decks'], manifest['opponent_games']):
            winner, seat = game['winner'], game['learner_seat']
            score = 0.5 if winner is None else float(winner == seat)
            out[game['game_id']] = dict(score=score, learner_seat=seat, truncated=game['truncated'],
                                        decks=[names[i] for i in pair])
    return out


def decide(challenger: dict, holder: dict, games: int) -> dict:
    """Paired comparison on identical games; promotion needs a strictly
    higher score."""
    if set(challenger) != set(holder) or len(challenger) != games:
        raise ValueError('Gate games are incomplete or unpaired')
    for game_id, row in challenger.items():
        other = holder[game_id]
        if (row['decks'], row['learner_seat']) != (other['decks'], other['learner_seat']):
            raise ValueError(f'Gate game {game_id} differs between challenger and champion')
    diffs = [challenger[g]['score'] - holder[g]['score'] for g in sorted(challenger)]
    mean = sum(diffs) / games
    var = sum((d - mean) ** 2 for d in diffs) / (games - 1)
    return dict(games=games,
                challenger_score=sum(r['score'] for r in challenger.values()) / games,
                champion_score=sum(r['score'] for r in holder.values()) / games,
                difference=mean, difference_se=(var / games) ** 0.5,
                truncated=dict(challenger=sum(r['truncated'] for r in challenger.values()),
                               champion=sum(r['truncated'] for r in holder.values())),
                promoted=mean > 0)


def run_gate(root: Path, plan: dict, arch: str, gen: int, index: int) -> dict:
    from tools.run_selfplay_generation import run_selfplay_stage
    from tools.selfplay_scheduler import atomic_json
    record_path = gate_path(root, arch, gen)
    if record_path.exists():
        return json.loads(record_path.read_text())
    g = plan['gate']
    run_dir = specialist_dir(root, arch)
    new = dict(generation=gen, onnx=str(run_dir / f'gen{gen}' / 'model.onnx'),
               zip=str(run_dir / f'gen{gen}' / 'model.zip'))
    old = champion(root, arch, gen - 1)
    seed = g['seed'] + 1_000_003 * gen + 1009 * index
    folder = record_path.parent / slug(arch)
    games = {}
    for role, model in (('challenger', new), ('champion', old)):
        pool_path = folder / f'{role}-pool.json'
        pool = gate_pool(root, arch, Path(model['onnx']))
        if pool_path.exists():
            if json.loads(pool_path.read_text()) != pool:
                raise ValueError(f'Frozen gate pool differs: {pool_path}')
        else:
            pool_path.parent.mkdir(parents=True, exist_ok=True)
            atomic_json(pool_path, pool)
        out = folder / role
        print(f'--- gate gen {gen} {arch}: {role} gen{model["generation"]} ({g["games"]} games) ---', flush=True)
        dirs = run_selfplay_stage(
            cli_bin=CLI_BIN, gen_dir=out, model_onnx=model['onnx'], uniform=False,
            decks=root / 'inputs/decks.json', games=g['games'], procs=plan['workers'], sims=g['sims'],
            worlds=g['worlds'], mode='pimc', seed_base=seed, temperature_decisions=0, decision_cap=1200,
            shard_rows=4096, cards_json=Path(CARDS_JSON), pool='implemented', eval_batch_size=4,
            chunk_games=4, resume=out.exists(), worker_timeout_seconds=7200,
            budget_flags=['--root-noise-epsilon', '0'], opponent_pool=pool_path)
        games[role] = gate_scores(dirs)
    result = decide(games['challenger'], games['champion'], g['games'])
    new, old = dict(new, sha256=sha256(Path(new['onnx']))), dict(old, sha256=sha256(Path(old['onnx'])))
    record = dict(schema=GATE_SCHEMA, archetype=arch, generation=gen, seed=seed, field='base',
                  challenger=new, champion_before=old, **result,
                  champion_after=new if result['promoted'] else old)
    atomic_json(record_path, record)
    print(f'gate gen {gen} {arch}: challenger {result["challenger_score"]:.3f} vs champion '
          f'gen{old["generation"]} {result["champion_score"]:.3f} -> '
          f'{"PROMOTED" if result["promoted"] else "kept champion"}', flush=True)
    return record


def warm_start_from_champion(root: Path, arch: str, gen: int):
    """After the gate on generation ``gen``, the next generation warm-starts
    from the champion (idempotent; only while ``gen`` is the latest trained)."""
    from tools.selfplay_scheduler import atomic_json
    run_dir = specialist_dir(root, arch)
    state = json.loads((run_dir / 'state.json').read_text())
    if state['next_gen'] != gen + 1:
        return
    champ = champion(root, arch, gen)
    prev = dict(model_zip=champ['zip'], model_onnx=champ['onnx'])
    if state.get('prev') != prev:
        atomic_json(run_dir / 'state.json', dict(state, prev=prev))


def generation_command(root: Path, plan: dict, arch: str, pool: Path, index: int) -> list[str]:
    t = plan['training']
    base = root / 'inputs/base'
    return [sys.executable, '-m', 'tools.run_selfplay_generation',
            '--run-dir', str(specialist_dir(root, arch)),
            '--seed-model', f'{base}.zip', '--seed-model-onnx', f'{base}.onnx',
            '--decks', str(root / 'inputs/decks.json'), '--opponent-pool', str(pool),
            '--cli-bin', CLI_BIN, '--cards-json', CARDS_JSON,
            '--generations', '1', '--games', str(plan['games']), '--procs', str(plan['workers']),
            '--chunk-games', '4', '--worker-timeout-seconds', '7200', '--sims', '320',
            '--worlds', '4', '--eval-batch-size', '4', '--decision-cap', '1200',
            '--fail-on-truncation', '--quarantine-truncation', '--window', str(plan['replay_window']),
            '--replay-games-per-generation', str(plan['games']),
            '--epochs', str(t['epochs']), '--batch-size', str(t['batch_size']), '--lr', str(t['lr']),
            '--value-holdout-fraction', str(t['value_holdout_fraction']),
            '--value-activation', t['value_activation'], '--train-device', 'cpu',
            '--seed', str(plan['seed'] + 7919 * index), '--skip-eval']


def prepare(args):
    from tools.selfplay_scheduler import atomic_json
    from digimon_gym.agents.belief_contract import bind_prepared_run
    root, image = args.root, os.environ['DIGIMON_DEPLOYED_IMAGE']
    inputs = root / 'inputs'
    decks = json.loads((inputs / 'decks.json').read_text())
    archetypes = sorted({archetype(d['name']) for d in decks})
    cp8_archetypes = sorted(json.loads((inputs / 'cp8_archetypes.json').read_text()))
    if not set(cp8_archetypes) <= set(archetypes):
        raise ValueError('CP8 archetypes must be in the league pool')
    if sha256(inputs / 'opponents/cp8.onnx') != args.cp8_sha256:
        raise ValueError('CP8 export hash differs')
    for suffix in MODEL_SUFFIXES:
        if sha256(inputs / f'base{suffix}') != sha256(inputs / f'opponents/base{suffix}'):
            raise ValueError('Frozen base opponent must be the base checkpoint')
    for arch in archetypes:
        run = specialist_dir(root, arch)
        gen0 = run / 'gen0'
        if (run / 'state.json').exists():
            raise FileExistsError(f'{run} already prepared')
        gen0.mkdir(parents=True)
        for suffix in MODEL_SUFFIXES:
            shutil.copy2(inputs / f'base{suffix}', gen0 / f'model{suffix}')
        atomic_json(run / 'state.json', dict(next_gen=1, prev=dict(
            model_zip=str(gen0 / 'model.zip'), model_onnx=str(gen0 / 'model.onnx'))))
        bind_prepared_run(run)
    plan = dict(schema=SCHEMA, image=image, controller_commit=args.source_commit,
                spec='docs/superpowers/plans/2026-11-08-specialist-league.md',
                base=args.base_label, archetypes=archetypes, cp8_archetypes=cp8_archetypes,
                generations=args.generations, games=args.games, workers=8, seed=args.seed,
                replay_window=3,
                training=dict(epochs=4, lr=0.0001, batch_size=512, value_holdout_fraction=0.2,
                              value_activation='tanh'),
                mixture=dict(gen1=dict(base_selfplay=0.8, cp8=0.2),
                             later=dict(champions=0.7, base=0.15, cp8=0.15)),
                promotion='none: champion = latest generation',
                files={p.relative_to(root).as_posix(): sha256(p) for p in inputs.rglob('*') if p.is_file()})
    health = root / 'validation/health'
    started = time.monotonic()
    pool_path = root / 'validation/health-pool.json'
    pool_path.parent.mkdir(parents=True, exist_ok=True)
    atomic_json(pool_path, opponent_pool(root, plan, archetypes[0], 1))
    from tools.run_selfplay_generation import run_selfplay_stage
    dirs = run_selfplay_stage(cli_bin='/usr/local/bin/digimon-engine-cli', gen_dir=health,
        model_onnx=str(inputs / 'base.onnx'), uniform=False, decks=inputs / 'decks.json', games=8,
        procs=4, sims=64, worlds=4, mode='pimc', seed_base=7, temperature_decisions=16,
        decision_cap=1200, shard_rows=4096, cards_json=Path('/app/data/cards.json'), pool='implemented',
        eval_batch_size=4, scheduling='dynamic', chunk_games=2, opponent_pool=pool_path)
    manifests = [json.loads((d / 'manifest.json').read_text()) for d in dirs]
    names = manifests[0]['deck_names']
    learner_ok = all(archetype(names[pair[r['learner_seat']]]) == archetypes[0]
                     and archetype(names[pair[1 - r['learner_seat']]]) != archetypes[0]
                     for m in manifests for pair, r in zip(m['game_decks'], m['opponent_games']))
    games = sum(m['games'] for m in manifests)
    if games != 8 or not learner_ok or any(m['truncated_games'] for m in manifests):
        raise RuntimeError(f'Health check failed: games={games} learner_ok={learner_ok}')
    plan['health'] = dict(games=games, specialist=archetypes[0], seconds=round(time.monotonic() - started))
    atomic_json(root / 'league-plan.json', plan)
    atomic_json(root / 'league_state.json', dict(stage='prepared', image=image))
    print(json.dumps(dict(status='prepared', archetypes=len(archetypes), decks=len(decks),
                          health=plan['health'])), flush=True)


def claim_path(root: Path, arch: str, gen: int) -> Path:
    return root / 'gates' / 'claims' / f'gen{gen}' / slug(arch)


def claim(root: Path, arch: str, gen: int, owner: str) -> bool:
    """Exclusive claim on one gate; re-entrant for the same owner."""
    path = claim_path(root, arch, gen)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
    except FileExistsError:
        try:
            return path.read_text().strip() == owner
        except FileNotFoundError:
            return False
    with os.fdopen(fd, 'w') as handle:
        handle.write(owner)
    return True


def settle_gates(root: Path, plan: dict, gen: int, poll_seconds: float = 30.0):
    """Every specialist's gate on ``gen`` is decided and applied: run the
    unclaimed ones here, wait for the ones a helper holds (a helper that fails
    releases its claim, and the gate then runs here)."""
    from tools.selfplay_scheduler import atomic_json
    pending = list(enumerate(plan['archetypes']))
    while pending:
        waiting = []
        for index, arch in pending:
            if not gate_path(root, arch, gen).exists():
                if not claim(root, arch, gen, 'controller'):
                    waiting.append((index, arch))
                    continue
                gate(root, plan, arch, gen, index)
            warm_start_from_champion(root, arch, gen)
        if waiting:
            atomic_json(root / 'league_state.json', dict(stage='gate-wait', generation=gen,
                        waiting=[a for _, a in waiting], time_unix=time.time(), image=plan['image']))
            time.sleep(poll_seconds)
        pending = waiting


def gate(root: Path, plan: dict, arch: str, gen: int, index: int):
    from tools.selfplay_scheduler import atomic_json
    if not gate_path(root, arch, gen).exists():
        atomic_json(root / 'league_state.json', dict(stage='gate', generation=gen, specialist=arch,
                    position=f'{index + 1}/{len(plan["archetypes"])}', time_unix=time.time(),
                    image=plan['image']))
    run_gate(root, plan, arch, gen, index)
    warm_start_from_champion(root, arch, gen)


def run(args):
    from tools.selfplay_scheduler import atomic_json, controller_lock
    root = args.root
    plan = json.loads((root / 'league-plan.json').read_text())
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image']:
        raise ValueError('Image differs from the prepared league')
    for name, digest in plan['files'].items():
        if sha256(root / name) != digest:
            raise ValueError(f'Input hash mismatch: {name}')
    remote = bool(plan.get('gate', {}).get('remote'))
    inline = plan.get('gate') and not remote
    with controller_lock(root / 'league.lock'):
        for gen in range(1, plan['generations'] + 1):
            for index, arch in enumerate(plan['archetypes']):
                run_dir = specialist_dir(root, arch)
                state = json.loads((run_dir / 'state.json').read_text())
                if state['next_gen'] > gen:
                    if inline:
                        gate(root, plan, arch, gen, index)
                    continue
                if state['next_gen'] != gen:
                    raise RuntimeError(f'{arch} is at generation {state["next_gen"]}, expected {gen}')
                pool = root / 'pools' / f'gen{gen}' / f'{slug(arch)}.json'
                expected = opponent_pool(root, plan, arch, gen)
                if pool.exists():
                    if json.loads(pool.read_text()) != expected:
                        raise ValueError(f'Frozen pool differs: {pool}')
                else:
                    pool.parent.mkdir(parents=True, exist_ok=True)
                    atomic_json(pool, expected)
                atomic_json(root / 'league_state.json', dict(stage='generation', generation=gen,
                            specialist=arch, position=f'{index + 1}/{len(plan["archetypes"])}',
                            time_unix=time.time(), image=plan['image']))
                print(f'=== LEAGUE gen {gen} specialist {arch} ({index + 1}/{len(plan["archetypes"])}) ===',
                      flush=True)
                command = generation_command(root, plan, arch, pool, index)
                if (run_dir / f'gen{gen}').exists():
                    # Self-play stopped before fitting; the scheduler resumes
                    # only an identical, unfinished-or-complete stage.
                    command.append('--resume-selfplay')
                subprocess.run(command, check=True)
                if inline:
                    gate(root, plan, arch, gen, index)
            if remote:
                settle_gates(root, plan, gen)
        atomic_json(root / 'league_state.json', dict(stage='complete', generation=plan['generations'],
                    time_unix=time.time(), image=plan['image']))


def enable_gate(args):
    """Add the promotion gate to a prepared league and extend it. Records the
    change (and the new image) in the plan; the next ``run`` gates the
    already-trained generations in order, then continues."""
    from tools.selfplay_scheduler import atomic_json
    root = args.root
    plan = json.loads((root / 'league-plan.json').read_text())
    if plan.get('gate'):
        raise ValueError('The league already has a promotion gate')
    if args.generations < plan['generations']:
        raise ValueError('Cannot shorten the league')
    if args.games < 4 or args.games % 2:
        raise ValueError('Gate games must be even (paired openings)')
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    plan.setdefault('recoveries', []).append(dict(
        kind='enable-promotion-gate', time_unix=time.time(), previous_image=plan['image'], image=image,
        previous_generations=plan['generations'], generations=args.generations,
        previous_promotion=plan['promotion'], controller_commit=args.source_commit, reason=args.reason))
    plan.update(image=image, generations=args.generations,
                gate=dict(games=args.games, sims=320, worlds=4, temperature_decisions=0, root_noise_epsilon=0.0,
                          field='base', seed=args.seed, rule='promote iff paired mean difference > 0'),
                promotion='paired gate vs the current champion on the base field (see gate)')
    atomic_json(root / 'league-plan.json', plan)
    print(json.dumps(dict(status='gate-enabled', generations=args.generations, gate=plan['gate'])), flush=True)


def enable_remote_gates(args):
    """Let helper hosts run gates. Gate games and decisions are unchanged
    (same seeds, settings and rule); only where they run changes."""
    from tools.selfplay_scheduler import atomic_json
    root = args.root
    plan = json.loads((root / 'league-plan.json').read_text())
    if not plan.get('gate'):
        raise ValueError('The league has no promotion gate')
    if plan['gate'].get('remote'):
        raise ValueError('Remote gates are already enabled')
    image = os.environ['DIGIMON_DEPLOYED_IMAGE']
    plan.setdefault('recoveries', []).append(dict(
        kind='enable-remote-gates', time_unix=time.time(), previous_image=plan['image'], image=image,
        controller_commit=args.source_commit, reason=args.reason))
    plan['image'] = image
    plan['gate'] = dict(plan['gate'], remote=True)
    atomic_json(root / 'league-plan.json', plan)
    print(json.dumps(dict(status='remote-gates-enabled', image=image)), flush=True)


def gate_one(args):
    """Helper side: run one gate the coordinator claimed for this host."""
    root = args.root
    plan = json.loads((root / 'league-plan.json').read_text())
    if os.environ.get('DIGIMON_DEPLOYED_IMAGE') != plan['image']:
        raise ValueError('Image differs from the league plan')
    if not plan.get('gate', {}).get('remote'):
        raise ValueError('Remote gates are not enabled')
    index = plan['archetypes'].index(args.arch)
    run_gate(root, plan, args.arch, args.gen, index)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    b = sub.add_parser('build-decks')
    b.add_argument('--campaign-decks', type=Path, required=True)
    b.add_argument('--roster', type=Path, required=True)
    b.add_argument('--admissibility', type=Path, required=True)
    b.add_argument('--extra', action='append', default=[])
    b.add_argument('--out', type=Path, required=True)
    p = sub.add_parser('prepare')
    p.add_argument('--root', type=Path, default=Path('/experiment'))
    p.add_argument('--source-commit', required=True)
    p.add_argument('--base-label', required=True)
    p.add_argument('--cp8-sha256', required=True)
    p.add_argument('--generations', type=int, default=4)
    p.add_argument('--games', type=int, default=600)
    p.add_argument('--seed', type=int, default=90261108)
    r = sub.add_parser('run')
    r.add_argument('--root', type=Path, default=Path('/experiment'))
    e = sub.add_parser('enable-gate')
    e.add_argument('--root', type=Path, default=Path('/experiment'))
    e.add_argument('--source-commit', required=True)
    e.add_argument('--generations', type=int, required=True)
    e.add_argument('--games', type=int, default=120)
    e.add_argument('--seed', type=int, default=61109000)
    e.add_argument('--reason', required=True)
    m = sub.add_parser('enable-remote-gates')
    m.add_argument('--root', type=Path, default=Path('/experiment'))
    m.add_argument('--source-commit', required=True)
    m.add_argument('--reason', required=True)
    o = sub.add_parser('gate-one')
    o.add_argument('--root', type=Path, default=Path('/experiment'))
    o.add_argument('--arch', required=True)
    o.add_argument('--gen', type=int, required=True)
    args = parser.parse_args()
    if args.command == 'build-decks':
        campaign = json.loads(args.campaign_decks.read_text(encoding='utf-8'))
        decks = build_decks(campaign, json.loads(args.roster.read_text(encoding='utf-8')),
                            json.loads(args.admissibility.read_text(encoding='utf-8'))['cards'], args.extra)
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / 'decks.json').write_text(json.dumps(decks, indent=1), encoding='utf-8')
        (args.out / 'cp8_archetypes.json').write_text(
            json.dumps(sorted({archetype(d['name']) for d in campaign})), encoding='utf-8')
        from collections import Counter
        print(json.dumps(Counter(archetype(d['name']) for d in decks), indent=1))
    elif args.command == 'prepare':
        prepare(args)
    elif args.command == 'enable-gate':
        enable_gate(args)
    elif args.command == 'enable-remote-gates':
        enable_remote_gates(args)
    elif args.command == 'gate-one':
        gate_one(args)
    else:
        run(args)


if __name__ == '__main__':
    main()
