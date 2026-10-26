"""Join a frozen completed evaluation with list and external-meta audits.

Read-only toward the experiment. Rates are descriptive; no promotion, list
replacement, causal card ranking or real-world calibration is inferred.
"""
import argparse
from collections import Counter, defaultdict
import csv
import html
import json
from pathlib import Path


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def aggregate(rows, archetype, anchor, dimension):
    result = defaultdict(Counter)
    for row in rows:
        if row['agent_archetype'] != archetype or row['anchor'] != anchor or row['player'] != 1:
            continue
        result[row[dimension]].update({key: value for key, value in row.items()
            if isinstance(value, (int, float)) and key != 'player'})
    return {key: dict(value) for key, value in result.items()}


def outcome_rates(outcome):
    wins, losses, draws = (outcome[k] for k in ('wins', 'losses', 'draws'))
    games, decided = wins + losses + draws, wins + losses
    return dict(wins=wins, losses=losses, draws=draws, games=games, decided_games=decided,
        win_percent=100 * wins / games if games else None,
        score_percent=100 * (wins + .5 * draws) / games if games else None,
        decided_win_percent=100 * wins / decided if decided else None)


def build(snapshot, lists, meta):
    reports = [r for r in snapshot['reports'] if r['checkpoint'] == 0 and r['status'] == 'complete']
    if len(reports) != 1:
        raise ValueError('Expected exactly one completed baseline report')
    report = reports[0]
    provenance = report['provenance']
    snapshot_hashes = [digest for path, digest in lists['hashes'].items()
                       if path.replace('\\', '/').endswith('/inputs/combined_snapshot.json')]
    if len(snapshot_hashes) != 1 or provenance.get('snapshot_sha256') != snapshot_hashes[0]:
        raise ValueError('Evaluation snapshot differs from frozen list audit')
    if provenance.get('engine_image') != lists['deployed_image']:
        raise ValueError('Evaluation engine differs from frozen list audit')
    anchor = 'parent-checkpoint4'
    if (provenance['candidate_sha256'] != provenance['anchor_model_hashes'][anchor]
            or provenance['evaluation_mode'] != 'policy_only'
            or provenance['card_eval_event_version'] < 2):
        raise ValueError('Expected identical model weights and current decision telemetry')
    rows, details = [], {}
    external = {a['archetype']: a for a in meta['archetypes']}
    for item in lists['archetypes']:
        name = item['archetype']
        outcomes = {r['anchor']: r for r in report['outcomes'] if r['archetype'] == name}
        if set(outcomes) != {'greedy', anchor, 'development-fixed'}:
            raise ValueError(f'Incomplete anchor coverage for {name}')
        for outcome in outcomes.values():
            if outcome['incomplete'] or sum(outcome[k] for k in ('wins', 'losses', 'draws')) != 200:
                raise ValueError(f'Unexpected evaluation denominator for {name}')
        routes = aggregate(report['route_decisions'], name, anchor, 'route')
        cards = aggregate(report['cards'], name, anchor, 'card_id')
        effects = aggregate(report.get('effect_sequences', []), name, anchor, 'card_id')
        signatures = [dict(card, metrics=cards.get(card['card_id'])) for card in item['signature_cards']]
        raising = routes.get('digivolve_raising', {})
        ext = external[name]
        rates = {key: outcome_rates(outcome) for key, outcome in outcomes.items()}
        same = rates[anchor]
        row = dict(archetype=name, same_model_wins=same['wins'], same_model_games=same['games'],
            same_model_losses=same['losses'], same_model_draws=same['draws'],
            same_model_decided_games=same['decided_games'], same_model_percent=same['win_percent'],
            same_model_score_percent=same['score_percent'],
            same_model_decided_win_percent=same['decided_win_percent'],
            greedy_percent=rates['greedy']['win_percent'], greedy_draws=rates['greedy']['draws'],
            development_percent=rates['development-fixed']['win_percent'],
            development_draws=rates['development-fixed']['draws'],
            tournament_percent=float(ext['format_row'][3].rstrip('%')),
            tournament_record=ext['format_row'][2], tournament_entries=int(ext['format_row'][1]),
            training_lists=item['training_distinct_lists'], evaluation_scope=item['evaluation_scope'],
            raising_choices=raising.get('choices'), raising_opportunities=raising.get('opportunities'))
        rows.append(row)
        details[name] = dict(routes=routes, cards=cards, signatures=signatures,
            effect_sequences=effects, outcomes=outcomes, anchor_rates=rates, tournament_url=ext['url'])
    return dict(schema='archetype-behavior-audit-v1', provenance=provenance,
        report_id=report['id'], summary=rows, archetypes=details,
        cautions=['Diagnostic data: confirmed End of Attack carrier-scoping defects affect ST23-04 and EX4-074; six additional pool cards require follow-up. These results cannot certify rules-faithful strength.',
            'One evaluation list per archetype; five pet lists were also used in training.',
            'Tournament rates cover global EX13 matches; simulator rates cover games against an equally weighted 13-deck pool.',
            'Simulator percent columns count wins / all completed games; score_percent counts draws as half, and decided_win_percent excludes draws as DigiLab does. Full anchor denominators are retained.',
            'All behaviors below use the same-model parent anchor only: 200 games per archetype.',
            'Route choices are conditional on reached states, not correctness labels. Card entry outcomes are correlational.',
            'Missing sequence instrumentation is unmeasured, not evidence that a mechanic never occurs.'])


def render(result, root):
    rows = result['summary']
    (root / 'behavior-audit.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    with (root / 'behavior-summary.csv').open('w', newline='', encoding='utf-8') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    esc = html.escape
    def table(items):
        if not items:
            return '<p>Unmeasured.</p>'
        columns = list(dict.fromkeys(k for row in items for k in row))
        return ('<div class="scroll"><table><tr>' + ''.join(f'<th>{esc(k)}</th>' for k in columns)
                + '</tr>' + ''.join('<tr>' + ''.join(f'<td>{esc("Unmeasured" if row.get(k) is None else str(row[k]))}</td>' for k in columns)
                    + '</tr>' for row in items) + '</table></div>')
    content = ['<!doctype html><meta charset="utf-8"><title>13-archetype behavior audit</title>',
        '<style>body{font:16px system-ui;margin:30px;background:#f8fafc;color:#18212e;max-width:1400px}'
        'table{border-collapse:collapse;background:white}td,th{padding:8px;border:1px solid #ccd5df;text-align:left}'
        'th{background:#e6edf5}.scroll{overflow:auto}details{margin:20px 0}summary{font-size:20px;cursor:pointer}'
        'p,li{max-width:1100px;line-height:1.5}pre{white-space:pre-wrap}'
        '.warning{border-left:5px solid #be123c;padding:16px;background:#fff1f2;color:#881337}</style>',
        '<h1>13-archetype behavior audit</h1><p>Fixed Nyaromon engine; baseline checkpoint 4 weights; '
        'October 26, 2026. This is the newly completed 7,800-game baseline, with 2,600 same-model games '
        '(200 candidate games per archetype), plus greedy and development anchors. No search.</p>',
        '<p class="warning"><strong>Diagnostic only.</strong> Confirmed timing defects affect Murasamemon '
        '(ST23-04) and ShineGreymon: Ruin Mode (EX4-074). The cloud run continues at the user\'s direction; '
        'these results do not qualify a model for promotion.</p>',
        '<ul>' + ''.join(f'<li>{esc(s)}</li>' for s in result['cautions']) + '</ul>', table(rows)]
    for name, data in result['archetypes'].items():
        content += [f'<details><summary>{esc(name)}</summary><h3>Route opportunities and choices</h3>',
            table([dict(route=k, **v) for k, v in data['routes'].items()]),
            '<h3>Signature card entry counts</h3><p>Entries include engine events; they are distinct from selecting '
            'a main-phase evolution action. Zero plays does not mean an Option was never used.</p>',
            table([dict(card_id=c['card_id'], name=c['name'], **{k:(c['metrics'] or {}).get(k) for k in (
                'games_seen_in_hand', 'games_entered', 'digivolves_raising', 'digivolves_battle',
                'dna_digivolves', 'plays_direct', 'plays_effect', 'attacks')}) for c in data['signatures']]),
            '<h3>Instrumented effect sequences</h3>', table([dict(card_id=k, **v) for k,v in data['effect_sequences'].items()]),
            f'<p><a href="{esc(data["tournament_url"])}">DigiLab source</a></p></details>']
    content += ['<h2>Frozen evaluation provenance</h2><pre>' + esc(json.dumps(result['provenance'], indent=2)) + '</pre>']
    (root / 'behavior-audit.html').write_text('\n'.join(content), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--snapshot', type=Path, required=True)
    args = parser.parse_args()
    result = build(read(args.snapshot), read(args.root / 'list-audit.json'), read(args.root / 'meta-evidence.json'))
    render(result, args.root)
    print(json.dumps(result['summary']), flush=True)


if __name__ == '__main__':
    main()
