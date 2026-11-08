"""Matched-root descriptive report; repeated searches are never independent games."""
import argparse
from collections import defaultdict
import hashlib
import html
import importlib.util
from itertools import combinations
import json
from pathlib import Path
import random
import statistics
import time

from tools.cloud_search.mulligan_diagnostic_controller import digest, read, require, write
from tools.cloud_search.opening_hand_review import describe, features


SEEDS = (9826103100, 9826103101, 9826103102)


def estimate(nodes, values, repeats=2000):
    """Ratio bootstrap of source blocks within each original seed namespace."""
    require(len(nodes) == len(values) and bool(nodes), "estimate length")
    strata = defaultdict(lambda: defaultdict(list))
    for node, value in zip(nodes, values):
        strata[node["namespace"]][node["block"]].append(float(value))
    groups = [list(blocks.values()) for _, blocks in sorted(strata.items())]
    rng = random.Random(9826103199)
    draws = []
    for _ in range(repeats):
        total, count = 0., 0
        for blocks in groups:
            for _ in blocks:
                sample = rng.choice(blocks)
                total += sum(sample)
                count += len(sample)
        draws.append(total / count)
    draws.sort()
    return dict(mean=statistics.mean(values), ci95=[draws[int(.025*repeats)], draws[min(repeats-1, int(.975*repeats))]],
                roots=len(nodes), source_blocks=sum(len(blocks) for blocks in groups),
                method="2000 source-block bootstrap resamples stratified by original seed namespace; repeats averaged per root")


def verify_frozen(root):
    spec = importlib.util.spec_from_file_location("frozen_diagnostic_controller", root / "controller.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.verify(root)
    protocol = read(root / "protocol.json")
    require(protocol.get("stage") == "full_diagnostic", "excluded qualification cannot be scored as full diagnostic")
    return protocol


def summarize(windows, linux, out):
    require(not out.exists(), "preserve prior report")
    all_rows, source_hashes = {}, {}
    for root in (windows, linux):
        protocol = verify_frozen(root)
        source_hashes[str(root)] = digest(root / "manifest.json")
        for job in protocol["jobs"]:
            request = read(root / f"requests/{job}.json")
            checkpoint = read(root / request["plan"])["checkpoint"]
            for row in read(root / f"results/{job}.json")["rows"]:
                key = (checkpoint, request["worlds"], request["seed"], row["node"])
                require(key not in all_rows, "duplicate root/config/seed decision")
                all_rows[key] = dict(row, host=protocol["platform"])
    require(digest(windows / "inputs/corpus.json") == digest(linux / "inputs/corpus.json"), "cohort mismatch")
    nodes = read(windows / "inputs/corpus.json")["nodes"]
    require(len(nodes) == 208 and len(all_rows) == 7488, "full diagnostic coverage incomplete")
    raw_cards = read(windows / "assets/cards.json")
    cards = {cid: describe(value) for cid, value in raw_cards.items()}
    original_features = {n["id"]: features(n["hand"], cards) for n in nodes}
    groups, root_metrics, review = [], {}, []
    for cp in (8, 12, 16, 20):
        for worlds in (4, 8, 16):
            metrics = []
            for node in nodes:
                rows = [all_rows[(cp, worlds, seed, node["id"])] for seed in SEEDS]
                # Raw policy input is identical; qualification established exact platform parity.
                require(len({(r["keep_prior"], r["redraw_prior"]) for r in rows}) == 1, "raw policy changed across repeats")
                policy = int(rows[0]["redraw_prior"] > rows[0]["keep_prior"])
                choices = [r["selected_plan"][0] for r in rows]
                metric = dict(node=node["id"], redraw=statistics.mean(choices), policy_redraw=policy,
                              policy_disagreement=statistics.mean(c != policy for c in choices),
                              seed_unstable=int(len(set(choices)) > 1), redraw_prior=rows[0]["redraw_prior"],
                              replacement_diversity=statistics.mean(r["unique_replacement_multisets"] for r in rows),
                              evaluated_value_including_roots=statistics.mean(r["evaluated_nodes_including_roots"]["mean"] for r in rows))
                metrics.append(metric)
                if metric["seed_unstable"] or metric["policy_disagreement"]:
                    review.append(dict(checkpoint=cp, worlds=worlds, root=node, choices=choices,
                                       policy_choice=policy, selected_after_results=True))
            root_metrics[(cp, worlds)] = metrics
            group = dict(checkpoint=cp, worlds=worlds, unique_roots=208, decisions=624,
                         metrics={k: estimate(nodes, [m[k] for m in metrics]) for k in
                                  ("redraw", "policy_redraw", "policy_disagreement", "seed_unstable")},
                         mean_redraw_prior=statistics.mean(m["redraw_prior"] for m in metrics),
                         mean_unique_replacements=statistics.mean(m["replacement_diversity"] for m in metrics),
                         mean_evaluated_value_including_roots=statistics.mean(m["evaluated_value_including_roots"] for m in metrics))
            group["by_archetype"] = []
            for archetype in sorted({n["archetype"] for n in nodes}):
                indices = [i for i,n in enumerate(nodes) if n["archetype"] == archetype]
                selected = [nodes[i] for i in indices]
                require(len(indices) == 16, "archetype root balance")
                group["by_archetype"].append(dict(archetype=archetype, roots=16,
                    redraw=estimate(selected, [metrics[i]["redraw"] for i in indices]),
                    policy_disagreement=estimate(selected, [metrics[i]["policy_disagreement"] for i in indices])))
            group["opening_presence_patterns"] = []
            for pattern in ("level3", "tamer", "level3_tamer", "level3_access", "level3_training", "level3_draw2", "level3_search"):
                for present in (False, True):
                    indices = [i for i,n in enumerate(nodes) if original_features[n["id"]][pattern] == present]
                    if indices:
                        group["opening_presence_patterns"].append(dict(pattern=pattern, present=present,
                            redraw=estimate([nodes[i] for i in indices], [metrics[i]["redraw"] for i in indices])))
            group["timing_by_host"] = {}
            group["sampled_replacement_presence"] = {}
            for host in ("windows", "linux"):
                host_rows = [r for (c,w,_,_),r in all_rows.items() if c == cp and w == worlds and r["host"] == host]
                times = sorted(r["seconds"] for r in host_rows)
                group["timing_by_host"][host] = dict(decisions=len(times), mean=statistics.mean(times),
                    median=statistics.median(times), p95=times[int(.95*(len(times)-1))], maximum=max(times))
            for pattern in ("level3", "tamer", "level3_access"):
                per_root = []
                for node in nodes:
                    rows = [all_rows[(cp, worlds, seed, node["id"])] for seed in SEEDS]
                    per_root.append(statistics.mean(features(s["replacement_hand"], cards)[pattern]
                        for r in rows for s in r["sampled_worlds"]))
                group["sampled_replacement_presence"][pattern] = estimate(nodes, per_root)
            groups.append(group)
    paired = []
    for cp in (8, 12, 16, 20):
        for worlds in (8, 16):
            baseline, variant = root_metrics[(cp,4)], root_metrics[(cp,worlds)]
            flips = [statistics.mean(all_rows[(cp,4,s,n["id"])]["selected_plan"] != all_rows[(cp,worlds,s,n["id"])]["selected_plan"] for s in SEEDS) for n in nodes]
            paired.append(dict(checkpoint=cp, baseline_worlds=4, variant_worlds=worlds,
                redraw_difference=estimate(nodes, [b["redraw"]-a["redraw"] for a,b in zip(baseline,variant)]),
                paired_choice_disagreement=estimate(nodes, flips)))
    model_pairs = []
    for a,b in combinations((8,12,16,20),2):
        for worlds in (4,8,16):
            flips=[statistics.mean(all_rows[(a,worlds,s,n["id"])]["selected_plan"] != all_rows[(b,worlds,s,n["id"])]["selected_plan"] for s in SEEDS) for n in nodes]
            model_pairs.append(dict(checkpoints=[a,b], worlds=worlds, paired_choice_disagreement=estimate(nodes,flips)))
    result=dict(created_at=time.time(), source_manifests=source_hashes, roots=208, decisions=7488,
                groups=groups, paired_world_effects=paired, model_pair_disagreements=model_pairs,
                classification_source_sha256=digest(Path(__file__).with_name("opening_hand_review.py")),
                notes=["Known exact deck composition; hidden placements sampled. No closed-deck or human strength evidence.",
                       "Rates are descriptive over 208 selected recorded roots. The three seeds and paired seats are correlated.",
                       "Archetype summaries contain only 16 roots each; intervals are exploratory, with sparse support and multiple comparisons.",
                       "Opening patterns are card-presence and printed-effect candidates. Egg compatibility, option requirements and available fodder are not established; no playable-sequence or correctness claim.",
                       "Sampled replacement hands are forced-redraw clones of actual search worlds, not the actual game's final hand or independent MCTS visits. Actual counterfactual final hands are unmeasured here.",
                       "Evaluated-node means include roots and leaves and are uncalibrated model values.",
                       "Windows uses repeat seed3100, Linux3101/3102. Timing is separated by host; no causal speed comparison."])
    out.mkdir(parents=True)
    write(out/'summary.json',result)
    write(out/'selected-review.json',dict(scope="Outcome-selected disagreements, separate from representative rates",rows=review))
    write(out/'original-hand-features.json',original_features)
    table=''
    for g in groups:
        def rate(key):
            m=g['metrics'][key]
            return f"{m['mean']:.1%} ({m['ci95'][0]:.1%}–{m['ci95'][1]:.1%})"
        table += f"<tr><td>CP{g['checkpoint']}</td><td>{g['worlds']}</td><td>{rate('redraw')}</td><td>{rate('policy_disagreement')}</td><td>{rate('seed_unstable')}</td></tr>"
    page='<!doctype html><meta charset="utf-8"><title>Matched mulligan diagnostic</title><style>body{font:16px system-ui;max-width:1200px;margin:40px auto}td,th{padding:12px;text-align:left;border-bottom:1px solid #ccc}p{line-height:1.5}</style><h1>Matched mulligan diagnostic</h1><p>208 saved hands · four frozen checkpoints · 7,488 search decisions. Intervals resample original source blocks, keeping repeated searches together.</p><table><tr><th>Model</th><th>Worlds</th><th>Redraw</th><th>Policy/search disagreement</th><th>Unstable across seeds</th></tr>'+table+'</table>'+''.join('<p>'+html.escape(n)+'</p>' for n in result['notes'])+'<p><a href="summary.json">Detailed paired and archetype summaries</a> · <a href="selected-review.json">Separate disagreement review cohort</a></p>'
    (out/'index.html').write_text(page,encoding='utf-8')
    return dict(roots=208,decisions=7488,groups=12,review_rows=len(review))


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--windows',type=Path,required=True)
    parser.add_argument('--linux',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args()
    print(json.dumps(summarize(args.windows,args.linux,args.out)))
