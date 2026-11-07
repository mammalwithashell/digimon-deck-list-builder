"""Build a read-only opening-hand review from committed mulligan audit rows.

Text categories are review heuristics, not engine legality or expert keep labels.
This never modifies a game, model, frozen asset, or evaluation result.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def describe(card):
    text = card.get('effect_description_eng') or ''
    # Each frozen text paragraph describes one effect. Require the relevant
    # activation in that paragraph; a later start-of-main draw is not On Play.
    immediate = [p.strip() for p in text.splitlines()
                 if re.match(r'^(?:\[[^\]]+\]\s*)*\[(?:On Play|Main)\]', p.strip())]
    search = [p for p in immediate if re.search(r'reveal the top \d+', p, re.I)
              and re.search(r'add .+?hand', p, re.I)]
    draw = [p for p in immediate if re.search(r'Draw\s+\d+', p, re.I)]
    draw2 = [p for p in draw if re.search(r'Draw\s+2\b', p, re.I)]
    evidence = list(dict.fromkeys(search + draw))
    return {'name': card['card_name_eng'], 'level': card.get('level'),
            'tamer': card.get('card_kind') == 1, 'cost': card.get('play_cost'),
            'search': bool(search), 'draw': bool(draw), 'draw2': bool(draw2),
            'training': 'training' in card['card_name_eng'].lower() and bool(search),
            'evidence': evidence,
            'conditional': any(re.search(r'\b(by|if|may trash)\b', p, re.I) for p in evidence),
            'option': card.get('card_kind') == 2}


def features(hand, cards, cap=3):
    rookies = [i for i, cid in enumerate(hand) if cards[cid]['level'] == 3]
    tamers = [i for i, cid in enumerate(hand) if cards[cid]['tamer']]
    access = [i for i, cid in enumerate(hand) if
              (cards[cid]['search'] or cards[cid]['draw']) and
              cards[cid]['cost'] is not None and cards[cid]['cost'] <= cap]
    distinct = lambda choices: any(i != j for i in rookies for j in choices)
    return {'level3': bool(rookies), 'tamer': bool(tamers),
            'level3_tamer': distinct(tamers), 'level3_access': distinct(access),
            'level3_training': distinct([i for i in access if cards[hand[i]]['training']]),
            'level3_draw2': distinct([i for i in access if cards[hand[i]]['draw2']]),
            'level3_search': distinct([i for i in access if cards[hand[i]]['search']]),
            'access': bool(access)}


HTML = r'''<!doctype html><meta charset="utf-8"><title>Opening hand review</title>
<style>body{margin:0;background:#101721;color:#e8eef6;font:16px system-ui}main{max-width:1450px;margin:auto;padding:32px}h1{margin-bottom:8px}p{line-height:1.6;color:#b7c8da}.filters{display:flex;flex-wrap:wrap;gap:14px;background:#1a2635;padding:18px;border-radius:12px;position:sticky;top:0}label{display:grid;gap:6px;font-size:13px;color:#b7c8da}select,input,button{font:inherit;background:#101721;color:#e8eef6;border:1px solid #536880;border-radius:6px;padding:9px}button{cursor:pointer}table{border-collapse:collapse;width:100%;margin:20px 0}td,th{text-align:left;padding:12px;border-bottom:1px solid #304052;vertical-align:top}th{color:#8dddc8}.hand{display:flex;flex-wrap:wrap;gap:6px}.card{background:#213244;padding:8px;border-radius:6px;width:155px;font-size:13px}.card small{display:block;color:#9eb5ce;margin-top:5px}.tag{color:#8dddc8}.pill{color:#ffd18b}summary{cursor:pointer}details p{font-size:13px}a{color:#8dddc8}.count{font-size:21px;margin-top:22px}.muted{font-size:13px}footer{margin:20px 0}</style>
<main><a href="../index.html">Campaign report</a><h1>Opening hand review</h1>
<p>Find recorded keep/redraw decisions using your opening-hand criteria. Level 3 + Training, level 3 + Draw 2, level 3 + search, and level 3 + Tamer are separate filters. Hybrid Tamer access has its own filter. Pairs require distinct card copies.</p>
<details><summary>What these filters establish</summary><p>These are card-presence and printed-effect candidates, not proven playable sequences or automatic mistakes. A level 3 may not evolve over the deck’s egg. Option color/use requirements, discard fodder, available memory, search hits, and archetype-specific Tamer compatibility still need review. “Access” counts Main/On Play draw or reveal-and-add effects up to your chosen printed play cost; it excludes inherited, attack, later start-of-main and evolution-only effects. A search rookie used to evolve in raising cannot also trigger its On Play effect. Draw-and-discard is access, not necessarily card advantage. Only four representative card images were manually checked; other classifications use frozen text and expose their evidence.</p><p>Counts are descriptive. Reused seeds across panels are correlated. Keep panels separate; no pooled strength or optimal-mulligan inference. Final hand means the hand immediately after the keep/redraw decision, before normal turn draws. Snapshot provenance is in opening-hand-review.json.</p></details>
<div class="filters"><label>Panel<select id="panel"></select></label><label>Model<select id="cp"><option value="">All models</option></select></label><label>Archetype<select id="arch"><option value="">All archetypes</option></select></label><label>Decision<select id="choice"><option value="">Keep + redraw</option><option>keep</option><option>redraw</option></select></label><label>Hand to filter<select id="which"><option value="initial_hand">Original hand</option><option value="after_hand">Final hand</option></select></label><label>Pattern<select id="pattern"><option value="all">All hands</option><option value="level3_access">Level 3 + draw/search</option><option value="level3_training">Level 3 + Training</option><option value="level3_draw2">Level 3 + Draw 2</option><option value="level3_search">Level 3 + search</option><option value="level3_tamer">Level 3 + Tamer</option><option value="no_level3">No level 3</option><option value="no_access">No immediate draw/search candidate</option><option value="hybrid_tamer">Hybrid with Tamer</option><option value="hybrid_no_tamer">Hybrid without Tamer</option></select></label><label>Access printed cost ≤<select id="cap"><option>2</option><option selected>3</option><option>4</option><option>5</option><option>10</option></select></label><label>Card ID or name<input id="query" placeholder="e.g. Strabimon"></label></div>
<div class="count" id="count"></div><p class="muted" id="snapshot"></p><button id="export">Download filtered rows (JSON)</button><table><thead><tr><th>Recorded decision</th><th>Original hand</th><th>Final hand</th></tr></thead><tbody id="rows"></tbody></table><footer><button id="prev">Previous</button> <span id="page"></span> <button id="next">Next</button></footer></main>
<script>const DATA=__DATA__;const C=DATA.cards;const $=id=>document.getElementById(id);let page=0,filtered=[];const size=30;const esc=s=>String(s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function feats(h){let r=[],t=[],a=[];h.forEach((id,i)=>{let c=C[id];if(c.level===3)r.push(i);if(c.tamer)t.push(i);if((c.search||c.draw)&&c.cost!==null&&c.cost<=+$('cap').value)a.push(i)});let pair=x=>r.some(i=>x.some(j=>i!==j));return {level3:r.length>0,tamer:t.length>0,access:a.length>0,level3_access:pair(a),level3_training:pair(a.filter(i=>C[h[i]].training)),level3_draw2:pair(a.filter(i=>C[h[i]].draw2)),level3_search:pair(a.filter(i=>C[h[i]].search)),level3_tamer:pair(t)}}
for(let p of DATA.panels){let o=new Option(p.label,p.key);$('panel').add(o)}for(let cp of [...new Set(DATA.rows.map(r=>r.checkpoint))].sort((a,b)=>a-b))$('cp').add(new Option('CP'+cp,cp));for(let a of [...new Set(DATA.rows.map(r=>r.archetype))].sort())$('arch').add(new Option(a,a));
function hand(h){return '<div class="hand">'+h.map(id=>{let c=C[id],tags=[c.level===3?'Level 3':'',c.tamer?'Tamer':'',c.training?'Training':'',c.search?'Search candidate':'',c.draw2?'Draw 2 candidate':c.draw?'Draw candidate':''].filter(Boolean);return '<div class="card"><strong>'+esc(c.name)+'</strong><small>'+esc(id)+' · cost '+c.cost+'</small><small class="tag">'+tags.join(' · ')+'</small>'+(c.evidence.length?'<details><summary>Effect / conditions</summary><p>'+c.evidence.map(esc).join('<br>')+'</p><p>Legality/fodder not checked.</p></details>':'')+'</div>'}).join('')+'</div>'}
function render(reset=true){if(reset)page=0;let pat=$('pattern').value,q=$('query').value.toLowerCase();filtered=DATA.rows.filter(r=>{if(r.panel_key!==$('panel').value||($('cp').value&&r.checkpoint!==+$('cp').value)||($('arch').value&&r.archetype!==$('arch').value)||($('choice').value&&r.choice!==$('choice').value))return false;let h=r[$('which').value],f=feats(h);if(q&&!h.some(id=>(id+' '+C[id].name).toLowerCase().includes(q)))return false;return pat==='all'||(pat==='no_level3'?!f.level3:pat==='no_access'?!f.access:pat==='hybrid_tamer'?/hybrid/i.test(r.archetype)&&f.tamer:pat==='hybrid_no_tamer'?/hybrid/i.test(r.archetype)&&!f.tamer:f[pat])});let keeps=filtered.filter(r=>r.choice==='keep').length;$('count').textContent=filtered.length+' decisions · '+keeps+' kept · '+(filtered.length-keeps)+' redrawn';let p=DATA.panels.find(p=>p.key===$('panel').value);$('snapshot').textContent='Snapshot '+p.snapshot+' · '+p.committed_games+' committed games in this panel · two player decisions per game.';let pages=Math.max(1,Math.ceil(filtered.length/size));page=Math.min(page,pages-1);$('rows').innerHTML=filtered.slice(page*size,(page+1)*size).map(r=>'<tr><td><strong>CP'+r.checkpoint+' · '+esc(r.choice)+'</strong><p>'+esc(r.archetype)+'</p><small>'+esc(r.request)+'<br>Candidate seat '+(r.candidate_seat+1)+' · actor seat '+(r.seat+1)+'</small></td><td>'+hand(r.initial_hand)+'</td><td>'+hand(r.after_hand)+'</td></tr>').join('');$('page').textContent='Page '+(page+1)+' / '+pages;$('prev').disabled=page===0;$('next').disabled=page>=pages-1}
document.querySelectorAll('select,input').forEach(e=>e.addEventListener('input',()=>render()));$('prev').onclick=()=>{page--;render(false)};$('next').onclick=()=>{page++;render(false)};$('export').onclick=()=>{let b=new Blob([JSON.stringify({filters:Object.fromEntries([...document.querySelectorAll('select,input')].map(e=>[e.id,e.value])),note:DATA.note,rows:filtered},null,2)],{type:'application/json'});let a=document.createElement('a');a.href=URL.createObjectURL(b);a.download='opening-hand-filter.json';a.click();setTimeout(()=>URL.revokeObjectURL(a.href),1000)};render();</script>'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, required=True)
    args = parser.parse_args()
    root = args.root
    audit = root / 'mulligan-audit'
    cards_path = root / 'assets/cards.json'
    raw_cards = read(cards_path)
    rows, panels, provenance = [], [], []
    for name in ('local.json', 'cloud.json'):
        path = audit / name
        stamp = datetime.fromtimestamp(path.stat().st_mtime, timezone.utc).isoformat()
        provenance.append({'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'mtime_utc': stamp})
        for panel in read(path)['panels']:
            if not panel['decisions']:
                continue
            key = panel['queue'] + '/' + panel['panel']
            panels.append({'key': key, 'label': panel['panel'] + ' (' + panel['queue'] + ')',
                           'committed_games': panel['committed_games'], 'snapshot': stamp})
            for row in panel['decisions']:
                rows.append(dict(row, panel_key=key))
    ids = sorted({cid for row in rows for key in ('initial_hand', 'after_hand') for cid in row[key]})
    cards = {cid: describe(raw_cards[cid]) for cid in ids}
    data = {'version': 1, 'generated_at': datetime.now(timezone.utc).isoformat(),
            'note': 'Descriptive card-presence candidates, not legal-sequence proof or optimal keep labels. Panels share seeds; do not pool as independent evidence.',
            'sources': provenance, 'cards_sha256': hashlib.sha256(cards_path.read_bytes()).hexdigest(),
            'image_spot_checks': ['LM-062', 'BT17-020', 'P-204', 'BT16-082'],
            'panels': panels, 'cards': cards, 'rows': rows}
    payload = json.dumps(data, ensure_ascii=False)
    (audit / 'opening-hand-review.json').write_text(payload, encoding='utf-8')
    (audit / 'index.html').write_text(HTML.replace('__DATA__', payload.replace('<', '\\u003c')), encoding='utf-8')
    summary = []
    for panel in panels:
        for cp in sorted({r['checkpoint'] for r in rows if r['panel_key'] == panel['key']}):
            subset = [r for r in rows if r['panel_key'] == panel['key'] and r['checkpoint'] == cp]
            counts = Counter()
            for row in subset:
                f = features(row['initial_hand'], cards)
                counts['decisions'] += 1
                for k in ('level3_access', 'level3_training', 'level3_draw2', 'level3_tamer'):
                    if f[k]:
                        counts[k] += 1
                        counts[k + '_' + row['choice']] += 1
                if not f['level3']:
                    counts['no_level3'] += 1
                    counts['no_level3_' + row['choice']] += 1
            summary.append({'panel': panel['key'], 'checkpoint': cp, **counts})
    (audit / 'opening-hand-summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(json.dumps({'rows': len(rows), 'cards': len(cards), 'panels': len(panels), 'report': str(audit / 'index.html')}))


if __name__ == '__main__':
    main()
