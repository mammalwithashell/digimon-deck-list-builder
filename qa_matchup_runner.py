"""Cross-archetype matchup game runner for QA testing."""
import requests
import json
import sys
import time

BASE = 'http://localhost:8000'

def create_game(deck1, deck2):
    r = requests.post(f'{BASE}/debug/games', json={
        'deck1': deck1,
        'deck2': deck2,
        'player1_type': 'human',
        'player2_type': 'human',
        'first_player': 1,
        'skip_shuffle': True,
        'auto_mulligan': 'keep',
        'initial_memory': 5,
    })
    data = r.json()
    return data['game_id']

def get_state(gid):
    r = requests.get(f'{BASE}/games/{gid}/state')
    return r.json()

def get_actions(gid):
    r = requests.get(f'{BASE}/games/{gid}/actions')
    data = r.json()
    acts = data.get('actions', {})
    result = []
    for aid_str, desc in acts.items():
        result.append((int(aid_str), desc))
    return result

def do_action(gid, action_id):
    r = requests.post(f'{BASE}/games/{gid}/actions', json={'action': action_id})
    if r.status_code != 200:
        return {'error': f'HTTP {r.status_code}: {r.text[:200]}'}
    try:
        return r.json()
    except Exception as e:
        return {'error': f'JSON decode error: {str(e)}, text={r.text[:200]}'}

def set_memory(gid, mem):
    r = requests.post(f'{BASE}/debug/games/{gid}/set-memory', json={'memory': mem})
    return r.json()

def pick_action(actions):
    """Pick an action with priority: hatch > move > digivolve > play > attack > select > pass."""
    priorities = [
        lambda d: 'hatch' in d,
        lambda d: 'move' in d and 'breed' in d,
        lambda d: 'digivolve' in d,
        lambda d: 'play' in d and 'pass' not in d,
        lambda d: 'attack' in d and 'player' in d,  # Attack player (direct)
        lambda d: 'attack' in d,
        lambda d: 'block' in d and 'decline' not in d,
        lambda d: 'counter' in d and 'decline' not in d,
        lambda d: 'select' in d and 'decline' not in d,
    ]
    for prio in priorities:
        for aid, desc in actions:
            if prio(desc.lower()):
                return aid, desc

    # Prefer non-pass, non-decline
    for aid, desc in actions:
        dl = desc.lower()
        if 'decline' not in dl and 'pass' not in dl:
            return aid, desc

    return actions[0]

def play_game(game_id, p1_name, p2_name, max_turns=12, max_steps=300):
    """Play a game and return (turn_log, issues, final_state)."""
    issues = []
    turn_log = []
    step = 0
    last_turn = 0
    phase_counter = {}  # Detect loops

    while step < max_steps:
        step += 1
        try:
            state = get_state(game_id)
        except Exception as e:
            issues.append(f'Step {step}: Failed to get state: {e}')
            break

        if state.get('isGameOver'):
            winner = state.get('winner')
            w_name = p1_name if winner == 1 else p2_name if winner == 2 else 'Draw'
            turn_log.append(f'Step {step}: GAME OVER - Winner: {w_name} (Player {winner})')
            break

        turn = state.get('turnCount', 0)
        phase = state.get('currentPhase', 0)
        player = state.get('currentPlayer', 0)
        mem = state.get('memoryGauge', 0)
        p1 = state['player1']
        p2 = state['player2']

        # Memory consistency check
        if p1['memory'] != -p2['memory']:
            issues.append(f'Step {step}: T{turn} Memory inconsistency! P1={p1["memory"]} P2={p2["memory"]} (should sum to 0)')

        # Log new turns
        if turn != last_turn:
            ba1 = [d.get('cardId','?') for d in p1.get('battleArea', [])]
            ba2 = [d.get('cardId','?') for d in p2.get('battleArea', [])]
            p_name = p1_name if player == 1 else p2_name
            turn_log.append(f'')
            turn_log.append(f'--- Turn {turn} ({p_name}, P{player}) Mem={mem} ---')
            turn_log.append(f'  P1 ({p1_name}): hand={p1["handCount"]} sec={p1["securityCount"]} deck={p1["deckCount"]} BA={ba1}')
            turn_log.append(f'  P2 ({p2_name}): hand={p2["handCount"]} sec={p2["securityCount"]} deck={p2["deckCount"]} BA={ba2}')
            if p1.get('breedingArea'):
                turn_log.append(f'  P1 breeding: {p1["breedingArea"].get("topCardId", "-")}')
            if p2.get('breedingArea'):
                turn_log.append(f'  P2 breeding: {p2["breedingArea"].get("topCardId", "-")}')
            last_turn = turn
            phase_counter = {}

        # Detect phase loops
        phase_key = (turn, phase, player)
        phase_counter[phase_key] = phase_counter.get(phase_key, 0) + 1
        if phase_counter[phase_key] > 30:
            issues.append(f'Step {step}: Phase loop detected! T{turn} Ph{phase} P{player} repeated {phase_counter[phase_key]} times')
            turn_log.append(f'  Step {step}: PHASE LOOP DETECTED - stopping')
            break

        try:
            actions = get_actions(game_id)
        except Exception as e:
            issues.append(f'Step {step}: Failed to get actions: {e}')
            break

        if not actions:
            issues.append(f'Step {step}: No actions at T{turn} Ph{phase} P{player} - game stuck')
            turn_log.append(f'  Step {step}: NO ACTIONS - game stuck')
            break

        chosen_id, chosen_desc = pick_action(actions)

        result = do_action(game_id, chosen_id)

        if 'error' in result:
            err = result['error']
            issues.append(f'Step {step}: Action error [{chosen_id}] "{chosen_desc}": {err}')
            turn_log.append(f'  Step {step}: ERROR [{chosen_id}] {chosen_desc} -> {err[:100]}')
            # Try to continue by picking a different action
            if len(actions) > 1:
                for aid, desc in actions:
                    if aid != chosen_id:
                        result2 = do_action(game_id, aid)
                        if 'error' not in result2:
                            turn_log.append(f'  Step {step}: Recovered with [{aid}] {desc}')
                            result = result2
                            chosen_desc = desc
                            break
                else:
                    break
            else:
                break

        new_state = result.get('state', {})
        new_turn = new_state.get('turnCount', turn)
        new_phase = new_state.get('currentPhase', phase)
        new_mem = new_state.get('memoryGauge', mem)

        # Log significant actions
        dl = chosen_desc.lower()
        if any(kw in dl for kw in ['attack', 'security', 'play ', 'digivolve', 'hatch', 'move', 'block', 'counter']):
            turn_log.append(f'  Step {step}: [{chosen_id}] {chosen_desc} -> T{new_turn} Ph{new_phase} Mem={new_mem}')

        # Log selection phases
        if phase in [5,6,7,8,9,10,11,12,13]:
            phase_names = {5:'SelectTarget',6:'SelectMaterial',7:'SelectTrash',8:'SelectSource',9:'SelectHand',10:'SelectReveal',11:'SelectEffectChoice',12:'SelectSecurity',13:'BlockTiming'}
            pn = phase_names.get(phase, f'Ph{phase}')
            turn_log.append(f'  Step {step}: [{pn}] {chosen_desc}')

        # Check for cross-player effect contamination
        if 'error' not in result:
            ns = result.get('state', {})
            logs = result.get('logs', [])
            for log in logs:
                if isinstance(log, str) and 'wrong player' in log.lower():
                    issues.append(f'Step {step}: Wrong player effect detected in logs: {log}')

        # Stop condition
        if turn >= max_turns:
            turn_log.append(f'  Step {step}: Reached turn {turn}, stopping.')
            break

    # Final state
    try:
        final = get_state(game_id)
    except:
        final = state

    p1f = final['player1']
    p2f = final['player2']
    turn_log.append(f'')
    turn_log.append(f'=== FINAL STATE ===')
    turn_log.append(f'Turn: {final["turnCount"]}, Phase: {final["currentPhase"]}, GameOver: {final["isGameOver"]}, Winner: {final.get("winner")}')
    turn_log.append(f'P1 ({p1_name}): hand={p1f["handCount"]} sec={p1f["securityCount"]} deck={p1f["deckCount"]} BA={len(p1f["battleArea"])} trash={len(p1f["trashIds"])}')
    turn_log.append(f'P2 ({p2_name}): hand={p2f["handCount"]} sec={p2f["securityCount"]} deck={p2f["deckCount"]} BA={len(p2f["battleArea"])} trash={len(p2f["trashIds"])}')
    turn_log.append(f'Memory gauge: {final["memoryGauge"]}')

    # Battle area details
    for pi, pf, pn in [(1, p1f, p1_name), (2, p2f, p2_name)]:
        if pf['battleArea']:
            ba_details = []
            for d in pf['battleArea']:
                card = d.get('topCardId', '?')
                name = d.get('topCardName', '?')
                dp = d.get('dp', '?')
                susp = 'S' if d.get('isSuspended') else ''
                kws = d.get('keywords', [])
                ba_details.append(f'{card}({name} DP={dp}{" "+susp if susp else ""} kw={kws})')
            turn_log.append(f'P{pi} ({pn}) BA: {", ".join(ba_details)}')

    return turn_log, issues, final, step


# Deck definitions
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
    ('Royal Knights', 'Medusa', 'White aggro vs Red control'),
    ('CS Mastemon', 'TS Neptune', 'DNA mechanics vs Sovereigns'),
    ('Millennium', 'Diaboromon', 'Token/swarm interactions'),
    ('Rocks', 'CS Hudiemon', 'Machine vs CS multi-color'),
]


def run_matchup(idx, p1_arch, p2_arch, desc):
    print(f'\n{"="*70}')
    print(f'GAME {idx}: {p1_arch} vs {p2_arch} -- {desc}')
    print(f'{"="*70}')

    game_id = create_game(DECKS[p1_arch], DECKS[p2_arch])
    print(f'Game ID: {game_id}')

    turn_log, issues, final, steps = play_game(game_id, p1_arch, p2_arch)

    for line in turn_log:
        print(line)

    print(f'\nTotal steps: {steps}')
    print(f'Issues found: {len(issues)}')
    for i, iss in enumerate(issues):
        print(f'  [{i+1}] {iss}')

    return {
        'game_id': game_id,
        'p1': p1_arch,
        'p2': p2_arch,
        'desc': desc,
        'turn_log': turn_log,
        'issues': issues,
        'final': final,
        'steps': steps,
    }


if __name__ == '__main__':
    # Run specific matchup or all
    which = int(sys.argv[1]) if len(sys.argv) > 1 else 0

    results = []
    if which == 0:
        for i, (p1, p2, desc) in enumerate(MATCHUPS, 1):
            results.append(run_matchup(i, p1, p2, desc))
    else:
        p1, p2, desc = MATCHUPS[which-1]
        results.append(run_matchup(which, p1, p2, desc))

    print(f'\n{"="*70}')
    print('SUMMARY')
    print(f'{"="*70}')
    total_issues = 0
    for r in results:
        n = len(r['issues'])
        total_issues += n
        go = r['final'].get('isGameOver', False)
        winner = r['final'].get('winner')
        t = r['final'].get('turnCount', 0)
        print(f"Game {r['p1']} vs {r['p2']}: {r['steps']} steps, turn {t}, gameOver={go}, winner={winner}, issues={n}")
    print(f'Total issues across all games: {total_issues}')
