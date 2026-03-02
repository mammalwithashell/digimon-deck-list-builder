"""Enhanced combat-focused matchup runner v2."""
import requests
import json
import sys
import time

BASE = 'http://localhost:8000'

DECKS = {
    'Royal Knights': ["ST12-12","ST12-12","BT23-054","BT23-054","BT23-054","BT23-054","BT20-083","BT20-083","BT20-083","BT20-083","BT13-093","BT13-093","BT13-093","BT19-072","BT20-017","BT20-017","BT20-017","BT20-056","BT20-060","BT20-060","BT20-060","BT23-058","BT23-014","BT23-035","BT23-035","BT13-112","BT13-112","BT13-112","BT20-102","BT20-102","BT20-102","BT20-102","BT17-018","BT13-102","BT13-102","BT20-091","BT20-091","BT20-091","BT20-091","BT21-086","BT13-110","BT20-100","BT20-100","BT20-100","BT20-100","BT8-097","BT8-097","BT8-097","P-206","P-206","BT13-007","BT13-007","BT13-007","BT13-007"],
    'Medusa': ["BT21-001","BT21-001","BT21-001","BT21-001","BT21-008","BT21-008","BT21-008","BT21-008","BT23-005","BT23-005","BT23-005","BT24-008","BT24-008","BT24-008","BT24-008","P-189","BT21-017","BT21-017","BT21-017","BT24-011","BT24-012","BT24-012","BT24-012","BT24-012","BT21-025","BT21-025","BT21-025","BT24-016","BT24-016","BT24-016","BT24-016","BT21-029","BT21-029","BT24-017","BT24-017","BT24-017","BT24-018","BT24-018","BT24-018","BT18-087","BT21-081","BT21-081","BT24-082","BT24-082","BT24-082","P-035","P-103","P-103","P-103","LM-027","LM-027","BT24-089","BT24-089","BT24-089"],
    'CS Mastemon': ["BT15-037","BT15-037","EX6-020","EX6-020","ST10-04","ST10-04","ST10-04","ST10-04","BT22-034","BT22-034","BT14-033","BT14-033","BT14-033","BT14-033","ST10-02","ST10-02","BT9-033","BT23-067","BT23-067","BT23-067","BT11-083","BT11-083","BT11-083","BT23-031","BT23-031","BT23-031","BT23-102","BT23-102","BT23-102","BT11-042","BT11-042","BT11-094","BT11-094","BT11-094","EX6-074","EX6-074","EX6-022","BT22-089","BT22-089","BT22-089","BT22-089","P-187","P-187","P-187","EX6-029","EX4-074","BT10-042","BT4-104","LM-035","BT9-082","BT14-003","BT14-003","BT14-003","BT14-003"],
    'TS Neptune': ["BT24-002","BT24-002","BT24-002","BT24-002","P-196","P-196","P-196","P-196","P-197","BT24-020","BT24-020","BT24-020","BT24-020","BT24-031","BT24-031","BT24-023","BT24-023","BT24-023","BT24-027","BT24-027","BT24-034","BT24-034","BT24-034","BT24-034","BT24-028","BT24-028","BT24-028","BT24-029","BT24-059","BT24-059","BT24-059","BT24-030","BT24-030","BT24-030","BT24-040","BT24-040","BT24-040","BT24-051","BT24-085","BT24-085","BT24-085","BT24-085","BT24-102","P-104","P-104","P-104","P-104","BT24-090","BT24-090","BT24-090","BT24-100","BT24-100","BT24-100","BT24-100"],
    'Millennium': ["BT15-006","BT15-006","BT15-006","BT15-006","EX2-046","EX2-046","EX8-056","EX8-056","EX8-056","EX8-056","EX10-040","EX10-040","EX10-040","EX10-040","EX9-059","EX9-060","EX9-060","BT19-069","BT19-069","BT18-015","BT18-015","BT18-015","BT18-015","BT19-070","BT19-070","BT18-073","BT18-073","BT18-073","BT18-073","BT19-065","BT19-065","P-220","P-220","P-220","P-220","BT18-019","BT18-019","BT18-019","BT18-019","BT19-075","BT19-075","BT19-101","EX1-066","EX1-066","EX1-066","EX1-066","P-193","P-193","P-205","P-205","P-205","P-205","ST6-15","BT19-099"],
    'Diaboromon': ["EX6-036","EX6-036","EX6-036","BT17-053","BT17-053","BT17-053","BT22-053","BT22-053","BT22-053","BT22-053","BT24-052","BT24-052","BT24-052","BT24-052","EX6-039","EX6-039","BT2-059","BT2-059","BT5-063","BT5-063","BT22-057","BT22-057","BT22-057","BT22-057","EX6-041","EX6-041","BT17-055","BT17-055","BT22-059","BT22-059","EX6-043","EX6-043","EX6-043","BT22-064","BT22-064","BT22-064","BT24-065","BT24-065","BT17-060","BT17-060","BT5-090","BT5-090","BT5-090","BT5-090","BT22-091","BT22-091","P-107","P-107","LM-031","LM-031"],
    'Rocks': ["BT21-055","BT21-055","BT21-055","BT21-055","EX10-025","EX10-025","EX8-046","EX8-046","EX8-047","EX8-047","EX8-047","EX8-047","EX10-028","EX10-028","EX10-028","P-167","P-167","P-167","P-167","EX8-048","EX8-048","EX10-032","EX10-032","EX10-032","EX10-032","EX10-033","EX10-033","EX10-063","EX10-063","EX10-063","P-169","P-169","EX10-036","EX10-036","EX10-036","EX10-036","EX10-069","EX10-069","EX10-069","EX10-069","EX8-067","EX8-067","EX8-067","EX8-051","EX8-051","EX8-051","P-107","P-107","EX8-070","EX7-049","EX8-005","EX8-005","EX8-005","EX8-005"],
    'CS Hudiemon': ["BT16-082","BT16-082","BT16-082","BT22-043","BT22-043","BT22-043","BT22-043","BT22-044","BT22-044","BT22-044","BT22-044","BT23-048","BT23-048","BT23-048","BT23-048","BT23-101","BT23-101","BT23-101","BT23-101","BT23-027","BT23-027","BT23-027","BT23-027","BT23-020","BT23-020","BT23-020","BT23-020","BT23-050","BT23-050","BT23-050","BT23-032","BT23-032","BT23-032","BT23-032","BT16-025","BT16-025","BT23-081","BT23-081","BT23-081","BT23-081","BT23-090","BT23-090","BT23-090","BT22-089","BT22-089","BT22-089","BT22-099","BT22-099","BT22-099","BT22-099","BT22-005","BT22-005","BT22-005","BT22-005"],
}

MATCHUPS = [
    ('Royal Knights', 'Medusa'),
    ('CS Mastemon', 'TS Neptune'),
    ('Millennium', 'Diaboromon'),
    ('Rocks', 'CS Hudiemon'),
]

# Known crasher cards to avoid playing directly
CRASH_CARDS = {'BT23-054'}  # Magnamon On Play crashes

def pick_action(acts):
    """Pick an action, skipping known crashers, prefer attacks."""
    prios = [
        lambda d: 'attack' in d and 'player' in d,
        lambda d: 'attack' in d,
        lambda d: 'block' in d and 'decline' not in d,
        lambda d: 'counter' in d and 'decline' not in d,
        lambda d: 'digivolve' in d,
        lambda d: 'play' in d and 'pass' not in d and 'magnamon' not in d,
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


def run_game(idx, p1_name, p2_name, max_turns=15, max_steps=350):
    r = requests.post(f'{BASE}/debug/games', json={
        'deck1': DECKS[p1_name], 'deck2': DECKS[p2_name],
        'player1_type': 'human', 'player2_type': 'human',
        'first_player': 1, 'skip_shuffle': True,
        'auto_mulligan': 'keep', 'initial_memory': 5,
    }, timeout=10)
    gid = r.json()['game_id']
    print(f'\n=== GAME {idx}: {p1_name} vs {p2_name} ===')
    print(f'ID: {gid}', flush=True)

    issues = []
    turn_log = []
    last_turn = 0
    step = 0
    phase_counter = {}
    consecutive_errors = 0

    for _ in range(max_steps):
        step += 1
        try:
            state = requests.get(f'{BASE}/games/{gid}/state', timeout=5).json()
        except Exception as e:
            issues.append(f'Step {step}: state fetch failed: {e}')
            break

        if state.get('isGameOver'):
            w = state.get('winner')
            wn = p1_name if w == 1 else p2_name if w == 2 else 'Draw'
            turn_log.append(f'Step {step}: GAME OVER -> {wn} (P{w})')
            break

        turn = state['turnCount']
        phase = state['currentPhase']
        player = state['currentPlayer']
        mem = state['memoryGauge']
        p1 = state['player1']
        p2 = state['player2']

        # Memory check
        if p1['memory'] != -p2['memory']:
            issues.append(f'Step {step}: T{turn} Memory mismatch P1={p1["memory"]} P2={p2["memory"]}')

        if turn != last_turn:
            turn_log.append(f'T{turn} P{player} Mem={mem} | P1: h={p1["handCount"]} s={p1["securityCount"]} BA={len(p1["battleArea"])} | P2: h={p2["handCount"]} s={p2["securityCount"]} BA={len(p2["battleArea"])}')
            last_turn = turn
            phase_counter = {}
            # Boost memory periodically for combat
            if turn % 4 == 0:
                boost = 10 if player == 1 else -10
                try:
                    requests.post(f'{BASE}/debug/games/{gid}/set-memory', json={'memory': boost}, timeout=5)
                except:
                    pass

        # Loop detection
        pk = (turn, phase, player)
        phase_counter[pk] = phase_counter.get(pk, 0) + 1
        if phase_counter[pk] > 30:
            issues.append(f'Step {step}: Loop at T{turn} Ph{phase} P{player}')
            break

        try:
            acts_data = requests.get(f'{BASE}/games/{gid}/actions', timeout=5).json()
            acts = [(int(k), v) for k, v in acts_data.get('actions', {}).items()]
        except Exception as e:
            issues.append(f'Step {step}: actions fetch failed: {e}')
            break

        if not acts:
            issues.append(f'Step {step}: T{turn} Ph{phase} P{player} NO ACTIONS')
            break

        aid, desc = pick_action(acts)

        try:
            r = requests.post(f'{BASE}/games/{gid}/actions', json={'action': aid}, timeout=10)
        except Exception as e:
            issues.append(f'Step {step}: action post failed: {e}')
            break

        if r.status_code != 200:
            consecutive_errors += 1
            issues.append(f'Step {step}: T{turn} HTTP {r.status_code} on [{aid}] {desc}')
            turn_log.append(f'  ERR [{aid}] {desc} HTTP {r.status_code}')
            if consecutive_errors > 3:
                issues.append(f'Step {step}: Too many consecutive errors, stopping')
                break
            # Try another action
            for oaid, odesc in acts:
                if oaid != aid:
                    try:
                        r2 = requests.post(f'{BASE}/games/{gid}/actions', json={'action': oaid}, timeout=10)
                        if r2.status_code == 200:
                            turn_log.append(f'  Recovered [{oaid}] {odesc}')
                            consecutive_errors = 0
                            break
                    except:
                        pass
            continue

        consecutive_errors = 0
        result = r.json()
        ns = result.get('state', {})
        dl = desc.lower()

        if any(kw in dl for kw in ['attack', 'block', 'counter', 'security', 'play ', 'digivolve', 'hatch', 'move']):
            turn_log.append(f'  [{aid}] {desc[:65]} -> T{ns.get("turnCount")} Mem={ns.get("memoryGauge")}')

        if turn >= max_turns:
            turn_log.append(f'  Reached turn {turn}, done.')
            break

    # Final
    try:
        final = requests.get(f'{BASE}/games/{gid}/state', timeout=5).json()
    except:
        final = state

    p1f = final['player1']
    p2f = final['player2']
    turn_log.append(f'FINAL: T{final["turnCount"]} Over={final["isGameOver"]} W={final.get("winner")}')
    turn_log.append(f'  P1({p1_name}): h={p1f["handCount"]} s={p1f["securityCount"]} d={p1f["deckCount"]} BA={len(p1f["battleArea"])} tr={len(p1f["trashIds"])}')
    turn_log.append(f'  P2({p2_name}): h={p2f["handCount"]} s={p2f["securityCount"]} d={p2f["deckCount"]} BA={len(p2f["battleArea"])} tr={len(p2f["trashIds"])}')

    for line in turn_log:
        print(line, flush=True)
    print(f'Steps: {step}, Issues: {len(issues)}', flush=True)
    for i, iss in enumerate(issues):
        print(f'  [{i+1}] {iss}', flush=True)

    return {
        'game_id': gid, 'p1': p1_name, 'p2': p2_name,
        'turn_log': turn_log, 'issues': issues,
        'final': final, 'steps': step,
    }


if __name__ == '__main__':
    which = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    results = []

    if which == 0:
        for i, (p1, p2) in enumerate(MATCHUPS, 1):
            results.append(run_game(i, p1, p2))
    else:
        p1, p2 = MATCHUPS[which-1]
        results.append(run_game(which, p1, p2))

    print(f'\n{"="*60}')
    print('SUMMARY')
    print(f'{"="*60}')
    total = 0
    for r in results:
        n = len(r['issues'])
        total += n
        f = r['final']
        print(f"  {r['p1']} vs {r['p2']}: steps={r['steps']} T{f['turnCount']} over={f['isGameOver']} w={f.get('winner')} issues={n}")
    print(f'Total issues: {total}')
