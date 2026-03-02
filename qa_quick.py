"""Quick game runner - writes results to file."""
import urllib.request
import urllib.error
import json

BASE = 'http://localhost:8000'

def api_get(path):
    req = urllib.request.Request(f'{BASE}{path}')
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read())

def api_post(path, data):
    body = json.dumps(data).encode('utf-8')
    req = urllib.request.Request(f'{BASE}{path}', data=body,
                                 headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.status, json.loads(resp.read())
    except urllib.error.HTTPError as e:
        return e.code, {'error': str(e)}

DECKS = {
    'CS Mastemon': ["BT15-037","BT15-037","EX6-020","EX6-020","ST10-04","ST10-04","ST10-04","ST10-04","BT22-034","BT22-034","BT14-033","BT14-033","BT14-033","BT14-033","ST10-02","ST10-02","BT9-033","BT23-067","BT23-067","BT23-067","BT11-083","BT11-083","BT11-083","BT23-031","BT23-031","BT23-031","BT23-102","BT23-102","BT23-102","BT11-042","BT11-042","BT11-094","BT11-094","BT11-094","EX6-074","EX6-074","EX6-022","BT22-089","BT22-089","BT22-089","BT22-089","P-187","P-187","P-187","EX6-029","EX4-074","BT10-042","BT4-104","LM-035","BT9-082","BT14-003","BT14-003","BT14-003","BT14-003"],
    'TS Neptune': ["BT24-002","BT24-002","BT24-002","BT24-002","P-196","P-196","P-196","P-196","P-197","BT24-020","BT24-020","BT24-020","BT24-020","BT24-031","BT24-031","BT24-023","BT24-023","BT24-023","BT24-027","BT24-027","BT24-034","BT24-034","BT24-034","BT24-034","BT24-028","BT24-028","BT24-028","BT24-029","BT24-059","BT24-059","BT24-059","BT24-030","BT24-030","BT24-030","BT24-040","BT24-040","BT24-040","BT24-051","BT24-085","BT24-085","BT24-085","BT24-085","BT24-102","P-104","P-104","P-104","P-104","BT24-090","BT24-090","BT24-090","BT24-100","BT24-100","BT24-100","BT24-100"],
    'Millennium': ["BT15-006","BT15-006","BT15-006","BT15-006","EX2-046","EX2-046","EX8-056","EX8-056","EX8-056","EX8-056","EX10-040","EX10-040","EX10-040","EX10-040","EX9-059","EX9-060","EX9-060","BT19-069","BT19-069","BT18-015","BT18-015","BT18-015","BT18-015","BT19-070","BT19-070","BT18-073","BT18-073","BT18-073","BT18-073","BT19-065","BT19-065","P-220","P-220","P-220","P-220","BT18-019","BT18-019","BT18-019","BT18-019","BT19-075","BT19-075","BT19-101","EX1-066","EX1-066","EX1-066","EX1-066","P-193","P-193","P-205","P-205","P-205","P-205","ST6-15","BT19-099"],
    'Diaboromon': ["EX6-036","EX6-036","EX6-036","BT17-053","BT17-053","BT17-053","BT22-053","BT22-053","BT22-053","BT22-053","BT24-052","BT24-052","BT24-052","BT24-052","EX6-039","EX6-039","BT2-059","BT2-059","BT5-063","BT5-063","BT22-057","BT22-057","BT22-057","BT22-057","EX6-041","EX6-041","BT17-055","BT17-055","BT22-059","BT22-059","EX6-043","EX6-043","EX6-043","BT22-064","BT22-064","BT22-064","BT24-065","BT24-065","BT17-060","BT17-060","BT5-090","BT5-090","BT5-090","BT5-090","BT22-091","BT22-091","P-107","P-107","LM-031","LM-031"],
    'Rocks': ["BT21-055","BT21-055","BT21-055","BT21-055","EX10-025","EX10-025","EX8-046","EX8-046","EX8-047","EX8-047","EX8-047","EX8-047","EX10-028","EX10-028","EX10-028","P-167","P-167","P-167","P-167","EX8-048","EX8-048","EX10-032","EX10-032","EX10-032","EX10-032","EX10-033","EX10-033","EX10-063","EX10-063","EX10-063","P-169","P-169","EX10-036","EX10-036","EX10-036","EX10-036","EX10-069","EX10-069","EX10-069","EX10-069","EX8-067","EX8-067","EX8-067","EX8-051","EX8-051","EX8-051","P-107","P-107","EX8-070","EX7-049","EX8-005","EX8-005","EX8-005","EX8-005"],
    'CS Hudiemon': ["BT16-082","BT16-082","BT16-082","BT22-043","BT22-043","BT22-043","BT22-043","BT22-044","BT22-044","BT22-044","BT22-044","BT23-048","BT23-048","BT23-048","BT23-048","BT23-101","BT23-101","BT23-101","BT23-101","BT23-027","BT23-027","BT23-027","BT23-027","BT23-020","BT23-020","BT23-020","BT23-020","BT23-050","BT23-050","BT23-050","BT23-032","BT23-032","BT23-032","BT23-032","BT16-025","BT16-025","BT23-081","BT23-081","BT23-081","BT23-081","BT23-090","BT23-090","BT23-090","BT22-089","BT22-089","BT22-089","BT22-099","BT22-099","BT22-099","BT22-099","BT22-005","BT22-005","BT22-005","BT22-005"],
}

MATCHUPS = [
    ('CS Mastemon', 'TS Neptune'),
    ('Millennium', 'Diaboromon'),
    ('Rocks', 'CS Hudiemon'),
]


def pick_action(acts):
    prios = [
        lambda d: 'attack' in d and 'player' in d,
        lambda d: 'attack' in d,
        lambda d: 'block' in d and 'decline' not in d,
        lambda d: 'counter' in d and 'decline' not in d,
        lambda d: 'digivolve' in d,
        lambda d: 'play' in d and 'pass' not in d,
        lambda d: 'hatch' in d,
        lambda d: 'move' in d,
        lambda d: 'select' in d and 'decline' not in d,
        lambda d: 'decline' not in d and 'pass' not in d,
    ]
    for fn in prios:
        for aid, desc in acts:
            if fn(desc.lower()):
                return aid, desc
    return acts[0]


output = []

for p1n, p2n in MATCHUPS:
    sc, data = api_post('/debug/games', {
        'deck1': DECKS[p1n], 'deck2': DECKS[p2n],
        'player1_type': 'human', 'player2_type': 'human',
        'first_player': 1, 'skip_shuffle': True,
        'auto_mulligan': 'keep', 'initial_memory': 5,
    })
    gid = data['game_id']
    output.append(f'GAME {p1n} vs {p2n} ID={gid}')

    # Boost memory
    api_post(f'/debug/games/{gid}/set-memory', {'memory': 15})

    issues = []
    step = 0
    phase_counts = {}
    last_turn = 0

    for step in range(1, 250):
        state = api_get(f'/games/{gid}/state')
        if state.get('isGameOver'):
            output.append(f'  GAME OVER at step {step} W=P{state.get("winner")}')
            break

        turn = state['turnCount']
        phase = state['currentPhase']
        player = state['currentPlayer']
        mem = state['memoryGauge']
        p1 = state['player1']
        p2 = state['player2']

        if turn != last_turn:
            output.append(f'  T{turn} P{player} Mem={mem} P1:h={p1["handCount"]} s={p1["securityCount"]} BA={len(p1["battleArea"])} P2:h={p2["handCount"]} s={p2["securityCount"]} BA={len(p2["battleArea"])}')
            last_turn = turn
            phase_counts = {}
            if turn % 4 == 0:
                boost = 10 if player == 1 else -10
                api_post(f'/debug/games/{gid}/set-memory', {'memory': boost})

        if p1['memory'] != -p2['memory']:
            issues.append(f'T{turn}: Mem mismatch P1={p1["memory"]} P2={p2["memory"]}')

        pk = (turn, phase, player)
        phase_counts[pk] = phase_counts.get(pk, 0) + 1
        if phase_counts[pk] > 40:
            issues.append(f'T{turn}: Loop Ph{phase} P{player}')
            break

        acts_data = api_get(f'/games/{gid}/actions')
        acts = [(int(k), v) for k, v in acts_data.get('actions', {}).items()]
        if not acts:
            issues.append(f'T{turn}: No actions Ph{phase}')
            break

        aid, desc = pick_action(acts)
        sc, result = api_post(f'/games/{gid}/actions', {'action': aid})

        if sc != 200:
            issues.append(f'T{turn}: HTTP {sc} [{aid}] {desc}')
            for oaid, odesc in acts:
                if oaid != aid:
                    sc2, _ = api_post(f'/games/{gid}/actions', {'action': oaid})
                    if sc2 == 200:
                        break
            continue

        ns = result.get('state', {})
        dl = desc.lower()
        if any(kw in dl for kw in ['attack', 'block', 'counter', 'play ', 'digivolve']):
            output.append(f'    [{aid}] {desc[:60]} -> T{ns.get("turnCount")} Mem={ns.get("memoryGauge")}')

        if turn >= 15:
            break

    final = api_get(f'/games/{gid}/state')
    p1f = final['player1']
    p2f = final['player2']
    output.append(f'  FINAL: T{final["turnCount"]} Over={final["isGameOver"]} W={final.get("winner")}')
    output.append(f'  P1({p1n}): h={p1f["handCount"]} s={p1f["securityCount"]} d={p1f["deckCount"]} BA={len(p1f["battleArea"])} tr={len(p1f["trashIds"])}')
    output.append(f'  P2({p2n}): h={p2f["handCount"]} s={p2f["securityCount"]} d={p2f["deckCount"]} BA={len(p2f["battleArea"])} tr={len(p2f["trashIds"])}')
    output.append(f'  Steps: {step}, Issues: {len(issues)}')
    for i, iss in enumerate(issues):
        output.append(f'    [{i+1}] {iss}')

    # BA details
    for pi, pf, pn in [(1, p1f, p1n), (2, p2f, p2n)]:
        ba = pf.get('battleArea', [])
        if ba:
            dets = []
            for d in ba[:6]:
                dets.append(f'{d["topCardId"]}({d.get("topCardName","?")} DP={d.get("dp")} kw={d.get("keywords",[])})')
            output.append(f'  P{pi} BA: ' + ', '.join(dets))
    output.append('')

with open('qa_g234_results.txt', 'w') as f:
    f.write('\n'.join(output))
