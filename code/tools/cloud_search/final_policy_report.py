"""Render and publish completed frozen policy evaluations, outside gameplay."""
import argparse
import csv
import html
import json
from pathlib import Path
import time


def read(path):
    with path.open(encoding='utf-8') as stream:
        return json.load(stream)


def render(root):
    plan = read(root/'plan.json')
    results = {panel: read(root/f'{panel}-result.json') for panel in ['self', 'adjacent']}
    out = root/'report'
    out.mkdir(exist_ok=True)
    sections = []
    for panel, result in results.items():
        for kind in ['decks', 'matrix']:
            rows = result[kind]
            columns = list(dict.fromkeys(k for row in rows for k in row))
            with (out/f'{panel}-{kind}.csv').open('w', newline='', encoding='utf-8') as f:
                writer = csv.DictWriter(f, fieldnames=columns)
                writer.writeheader()
                writer.writerows(rows)
        names = sorted(row['deck'] for row in result['decks'])
        matrix = {(r['deck'], r['opponent_deck']): r for r in result['matrix']}
        title = 'Checkpoint 4 versus itself' if panel == 'self' else 'Checkpoint 4 versus checkpoint 3'
        intro = ('Same checkpoint on both sides. 40 games per distinct pairing; 480 appearances per deck. '
                 'Diagonal mirrors omitted. Rates reflect deck balance under this model, not intrinsic piloting skill.'
                 if panel == 'self' else
                 f"Checkpoint 4 score: {100*result['score']:.2f}% across {result['games']:,} games. "
                 f"95% paired-block bootstrap interval: {100*result['ci95'][0]:.2f}–{100*result['ci95'][1]:.2f}%. "
                 'Rows are checkpoint 4 decks; columns are checkpoint 3 decks. Draws score one half.')
        table = '<tr><th>Deck / opponent</th>'+''.join(f'<th>{html.escape(n)}</th>' for n in names)+'</tr>'
        for a in names:
            table += '<tr><th>'+html.escape(a)+'</th>'
            for b in names:
                cell = matrix.get((a, b))
                if cell is None:
                    table += '<td>—</td>'
                else:
                    rate = (cell['wins']+.5*cell['draws'])/cell['games']
                    color = f'hsl({round(120*rate)},45%,87%)'
                    table += f'<td style="background:{color}" title="{cell["wins"]}W / {cell["losses"]}L / {cell["draws"]}D">{100*rate:.1f}%<small>n={cell["games"]}</small></td>'
            table += '</tr>'
        rankings = '<tr><th>Deck</th><th>Score</th><th>Games</th></tr>'
        for row in sorted(result['decks'], key=lambda r: r['score'], reverse=True):
            rankings += f'<tr><td>{html.escape(row["deck"])}</td><td>{100*row["score"]:.1f}%</td><td>{row["games"]}</td></tr>'
        sections.append(f'<h2>{title}</h2><p>{intro}</p><table>{rankings}</table><div class="scroll"><table>{table}</table></div>')
    text = '''<!doctype html><html lang="en"><meta charset="utf-8"><title>Frozen policy evaluation</title>
<style>body{font:16px system-ui;margin:32px;color:#18212e;background:#f8fafc}h1,h2{color:#123451}
table{border-collapse:collapse;margin:24px 0;background:white}td,th{border:1px solid #d6dce3;padding:10px;text-align:center}
th{font-size:13px}small{display:block;font-size:10px;color:#485465}.scroll{overflow:auto}p{max-width:1000px;line-height:1.6}</style>
<h1>Frozen policy evaluation — October 26, 2026</h1><p>Policy only: no search or training. Frozen deck lists, models and native engine.
Player seats and opening order balanced; incomplete games fail evaluation rather than counting as draws.
Per-matchup samples are small: treat individual cell differences cautiously. No model automatically promoted.</p>'''
    text += ''.join(sections)+'<h2>Provenance</h2><pre>'+html.escape(json.dumps(plan, indent=2))+'</pre></html>'
    (out/'index.html').write_text(text, encoding='utf-8')
    return results, out


def publish(root, results, out):
    import gc
    import wandb
    from digimon_gym.agents.card_eval import publish_report
    plan = read(root/'plan.json')
    run = wandb.init(project='digimon-rl', entity='james-peters97-mammal',
        id='final-policy-eval-20261026', name='Checkpoint 4: self-match matrix and vs checkpoint 3',
        job_type='evaluation', resume='allow', config=plan)
    try:
        artifact = wandb.Artifact('final-policy-eval-20261026-results', type='evaluation', metadata=plan)
        artifact.add_dir(str(out), name='report')
        for name in ['plan.json', 'self-result.json', 'adjacent-result.json', 'self-schedule.json', 'adjacent-schedule.json']:
            artifact.add_file(str(root/name), name=name)
        run.log_artifact(artifact).wait()
        payload = {'evaluation/report': wandb.Html((out/'index.html').read_text(encoding='utf-8'), inject=False)}
        for panel, result in results.items():
            for kind in ['decks', 'matrix']:
                rows = result[kind]
                columns = list(dict.fromkeys(k for row in rows for k in row))
                payload[f'{panel}/{kind}'] = wandb.Table(columns=columns, data=[[r.get(k) for k in columns] for r in rows])
        run.log(payload)
        run.summary.update(dict(status='complete', adjacent_score=results['adjacent']['score'],
            adjacent_ci95=results['adjacent']['ci95'], self_games=results['self']['games'],
            adjacent_games=results['adjacent']['games'], automatic_promotion=False))
        for path in sorted(root.glob('*/shard*/reports/checkpoint_*.json')):
            report = read(path)
            if not publish_report(report, path, run, wait=True):
                raise RuntimeError(f'Card report upload failed: {path}')
            del report
            gc.collect()
        (root/'published.json').write_text(json.dumps(dict(url=run.url, status='complete', time_unix=time.time())))
    finally:
        run.finish()


if __name__ == '__main__':
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--root', type=Path, default=Path('/evaluation'))
    ap.add_argument('--wait', action='store_true')
    ap.add_argument('--publish', action='store_true')
    args = ap.parse_args()
    while True:
        state = read(args.root/'state.json')
        if state['stage'] == 'complete':
            break
        if state['stage'] == 'failed' or not args.wait:
            raise RuntimeError(f'Evaluation not complete: {state}')
        time.sleep(10)
    results, out = render(args.root)
    if args.publish:
        publish(args.root, results, out)
    print(json.dumps(dict(report=str(out/'index.html'), status='complete')))
