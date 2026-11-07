"""Freeze the finite A-D comparison schedule before scoring; never launch games."""
import argparse
from collections import Counter, defaultdict
import hashlib
import itertools
import json
from pathlib import Path
import shutil
import time

CHECKPOINTS = (8, 12, 16, 20)
PRIMARY_GAME_SEED = 11426103000
PRIMARY_SEARCH_SEED = 11526103000
HELDOUT_GAME_SEED = 11626103000
HELDOUT_SEARCH_SEED = 11726103000


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, allow_nan=False), encoding="utf-8")


def normalize(snapshot, archetypes, lists):
    decks = [dict(name=d["archetype"], cards=d["card_ids"], source_deck_id=d["deck_id"])
             for d in snapshot["decks"]]
    require(len(decks) == lists and len({d["name"] for d in decks}) == archetypes, "partition denominator")
    require(len({d["source_deck_id"] for d in decks}) == lists, "duplicate list identity")
    content = {tuple(sorted(Counter(d["cards"]).items())) for d in decks}
    require(len(content) == lists, "duplicate list content")
    return decks


def blocks(decks, archetypes, game_seed, search_seed):
    """Existing ten cyclic rounds, generalized only to the 12-archetype extension.

    Two opener requests per block, each with two swapped-model-seat games.
    Each archetype has twenty block endpoints -> forty candidate appearances.
    """
    require(game_seed % 2 == 0 and search_seed % 2 == 0, "even seed namespace required")
    groups = defaultdict(list)
    for i, deck in enumerate(decks):
        groups[deck["name"]].append(i)
    names = sorted(groups)
    require(len(names) == archetypes, "archetype denominator")
    seen = Counter()
    endpoint_use = Counter()
    result = []
    for round_index in range(10):
        for index, a in enumerate(names):
            b = names[(index + round_index) % len(names)]
            low, high = sorted((a, b))
            variants = ([(d, d) for d in groups[a]] if a == b else
                        list(itertools.product(groups[low], groups[high])))
            if archetypes == 12:
                # The held-out extension has variants even in the last sorted
                # archetype. Pair-local cycling can miss those entirely. Rotate
                # each archetype's endpoints across its unchanged source lists.
                indices = []
                for name in (a,b):
                    indices.append(groups[name][endpoint_use[name] % len(groups[name])])
                    endpoint_use[name] += 1
            else:
                x, y = variants[seen[low, high] % len(variants)]
                seen[low, high] += 1
                indices = [x, y] if a == low else [y, x]
            block = len(result) // 2
            for opener in (0, 1):
                result.append(dict(id=f"match-{block:03d}-{opener}",block=block,deck_indices=indices,
                    expected_first_player=opener, game_seed=game_seed+block*10+opener,
                    search_seed=search_seed+block*10+opener, candidate_seats=[0,1]))
    validate_blocks(decks,result,archetypes)
    return result


def validate_blocks(decks, rows, archetypes):
    require(len(rows) == 20*archetypes and len({r["id"] for r in rows}) == len(rows), "request count/ids")
    require({r["block"] for r in rows} == set(range(10*archetypes)), "block count")
    require({i for r in rows for i in r["deck_indices"]} == set(range(len(decks))), "missing list coverage")
    coverage = {role:Counter() for role in ("candidate","opponent")}
    list_appearances = Counter()
    for i in range(0,len(rows),2):
        first,second=rows[i:i+2]
        require(first["block"] == second["block"] and first["deck_indices"] == second["deck_indices"], "broken paired block")
        require([first["expected_first_player"],second["expected_first_player"]] == [0,1], "opener coverage")
        require(second["game_seed"] == first["game_seed"]+1 and second["search_seed"] == first["search_seed"]+1, "paired seeds")
    for row in rows:
        require(row["candidate_seats"] == [0,1] and row["expected_first_player"] == row["game_seed"]%2, "seat/opener convention")
        for candidate_seat in (0,1):
            for role,seat in (("candidate",candidate_seat),("opponent",1-candidate_seat)):
                deck=decks[row["deck_indices"][seat]]
                coverage[role][deck["name"],seat,seat==row["expected_first_player"]]+=1
                if role=="candidate":list_appearances[deck["source_deck_id"]]+=1
    expected={(name,seat,first):10 for name in {d["name"] for d in decks} for seat in (0,1) for first in (False,True)}
    require(all(dict(c) == expected for c in coverage.values()), "archetype/seat/opener imbalance")
    return dict(games=2*len(rows),receipts=len(rows),four_game_blocks=len(rows)//2,
        archetypes=archetypes,lists=len(decks),candidate_games_per_archetype=40,
        opponent_games_per_archetype=40,first_per_archetype=20,second_per_archetype=20,
        physical_seat_per_archetype=20,list_appearances=dict(sorted(list_appearances.items())))


def panels():
    result=[]
    for low,high in itertools.combinations(CHECKPOINTS,2):
        queue = "cloud-a" if low == 8 else "cloud-b"
        for stage,mode in (("A","closed-list"),("B","known-list"),("D","closed-list")):
            result.append(dict(id=f"{stage.lower()}-cp{high}-vs-cp{low}",stage=stage,checkpoint=high,
                opponent_checkpoint=low,candidate_mode=mode,opponent_mode=mode,
                partition="heldout" if stage=="D" else "train",games=480 if stage=="D" else 520,
                preferred_queue=queue,paired_control=f"{'b' if stage=='A' else 'a'}-cp{high}-vs-cp{low}" if stage in ('A','B') else None))
    for cp in CHECKPOINTS:
        result.append(dict(id=f"c-cp{cp}-open-vs-closed",stage="C",checkpoint=cp,opponent_checkpoint=cp,
            candidate_mode="known-list",opponent_mode="closed-list",partition="train",games=520,
            preferred_queue="local",paired_control=None))
    return result


def freeze(train, heldout, prior, original, out):
    require(not out.exists(), "preserve frozen schedule")
    train_decks=normalize(read(train),13,21);heldout_decks=normalize(read(heldout),12,23)
    content=lambda ds:{tuple(sorted(Counter(d['cards']).items())) for d in ds}
    require(not content(train_decks)&content(heldout_decks), "partition content overlap")
    require("Light Hybrid" not in {d['name'] for d in heldout_decks}, "heldout scope changed")
    rows={"train":blocks(train_decks,13,PRIMARY_GAME_SEED,PRIMARY_SEARCH_SEED),
          "heldout":blocks(heldout_decks,12,HELDOUT_GAME_SEED,HELDOUT_SEARCH_SEED)}
    matrix=panels();jobs=[]
    for panel in matrix:
        for row in rows[panel['partition']]:
            jobs.append(dict(row,panel=panel['id'],stage=panel['stage'],partition=panel['partition'],
                checkpoint=panel['checkpoint'],opponent_checkpoint=panel['opponent_checkpoint'],
                candidate_mode=panel['candidate_mode'],opponent_mode=panel['opponent_mode']))
    require(len(jobs)*2 == sum(p['games'] for p in matrix) == 11200, "finite scored budget")
    assets=read(original/'campaign-plan.json')['files'];models={}
    for cp in CHECKPOINTS:
        path=f'assets/checkpoint{cp}.onnx'
        require(sha(original/path)==assets[path],"original model hash changed")
        models[str(cp)]=assets[path]
    out.mkdir(parents=True);(out/'sources').mkdir()
    for source,name in ((train,'train_snapshot.json'),(heldout,'heldout_snapshot.json'),(prior,'prior.json')):
        shutil.copy2(source,out/'sources'/name)
    shutil.copy2(__file__,out/'preparer.py')
    write(out/'decks.json',dict(train=train_decks,heldout=heldout_decks))
    write(out/'panels.json',matrix);write(out/'jobs.json',jobs)
    coverage={part:validate_blocks(ds,rows[part],count) for part,ds,count in
              (('train',train_decks,13),('heldout',heldout_decks,12))}
    write(out/'coverage.json',coverage)
    # Queue preference is fixed; capacity reservations occur only after live checks.
    queues={q:[] for q in ('cloud-a','cloud-b','local')}
    for low,high in itertools.combinations(CHECKPOINTS,2):
        q='cloud-a' if low==8 else 'cloud-b'
        queues[q].extend([f'a-cp{high}-vs-cp{low}',f'b-cp{high}-vs-cp{low}'])
    for p in matrix:
        if p['stage']=='C':queues['local'].append(p['id'])
    for p in matrix:
        if p['stage']=='D':queues[p['preferred_queue']].append(p['id'])
    write(out/'queue-preferences.json',queues)
    write(out/'protocol.json',dict(schema='frozen-mode-scored-schedule-v1',created_at=time.time(),
        model_hashes=models,prior_file_sha256=sha(prior),total_scored_games=11200,total_receipts=5600,
        stage_games={'A':3120,'B':3120,'C':2080,'D':2880},stage_panels={'A':6,'B':6,'C':4,'D':6},
        seed_namespaces=dict(primary_game=PRIMARY_GAME_SEED,primary_search=PRIMARY_SEARCH_SEED,
                             heldout_game=HELDOUT_GAME_SEED,heldout_search=HELDOUT_SEARCH_SEED),
        simulations=320,worlds=4,batch=4,root_noise=0,every_nonforced_root=True,
        pairing='A/B and C use aligned primary blocks. D uses a separate namespace. Shared-seed panels are correlated.',
        analysis='Four-game-block score intervals; A/B use paired block differences. No independent panel pooling.',
        heldout_light_hybrid='unmeasured',heldout_weight_training_provenance='pending; stage D blocked',
        launch_allowed=False,qualification='both complete frozen preflights and exact Windows/Linux comparison required',
        runtime_and_limits='freeze qualified runtime, measured deadline/decision caps and live resource allocation before launch',
        seed_opener_contract='Game first_player must equal game_seed modulo2; validate actual output',
        no_training=True,no_automatic_promotion=True,no_retries=True))
    write(out/'manifest.json',dict(files={p.relative_to(out).as_posix():sha(p) for p in sorted(out.rglob('*')) if p.is_file()}))
    return dict(root=str(out),games=11200,receipts=5600,panels=22,launch_allowed=False,coverage=coverage)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('train','heldout','prior','original','out'):
        parser.add_argument('--'+name,type=Path,required=True)
    result=freeze(**vars(parser.parse_args()))
    print(json.dumps({k:v for k,v in result.items() if k!='coverage'}))
