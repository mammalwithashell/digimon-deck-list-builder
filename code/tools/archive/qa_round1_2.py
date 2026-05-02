"""Comprehensive QA tests for Round 1 (Medusa) and Round 2 (CS Hudiemon) effects."""
import requests
import sys

API = 'http://localhost:8000'

DECK2 = [
    'BT22-005','BT22-005','BT22-005','BT22-005',
    'BT22-043','BT22-043','BT22-043','BT22-043',
    'BT22-044','BT22-044','BT22-044','BT22-044',
    'BT23-048','BT23-048','BT23-048','BT23-048',
    'BT23-027','BT23-027','BT23-027','BT23-027',
    'BT23-050','BT23-050','BT23-050','BT23-050',
    'BT23-101','BT23-101',
    'BT23-020','BT23-020','BT23-020',
    'BT23-032','BT23-032','BT23-032',
    'BT16-025','BT16-025','BT16-025',
    'BT22-089','BT22-089','BT22-089',
    'BT23-081','BT23-081','BT23-081',
    'BT23-090','BT23-090',
    'BT22-099','BT22-099','BT22-099',
    'BT16-082','BT16-082','BT16-082','BT16-082',
]

MEDUSA_DECK = [
    "BT21-001","BT21-001","BT21-001","BT21-001",
    "BT23-005","BT23-005","BT23-005",
    "BT21-008","BT21-008","BT21-008","BT21-008",
    "BT24-008","BT24-008","BT24-008",
    "BT24-011","BT24-011",
    "BT21-017","BT21-017","BT21-017","BT21-017",
    "BT24-012","BT24-012","BT24-012",
    "P-189","P-189",
    "BT24-011",
    "BT21-025",
    "BT24-016","BT24-016","BT24-016","BT24-016",
    "BT24-017","BT24-017","BT24-017","BT24-017",
    "BT24-018","BT24-018","BT24-018",
    "BT18-087","BT18-087",
    "BT21-081","BT21-081",
    "BT24-082","BT24-082","BT24-082",
    "P-035","P-035",
    "P-103","P-103","P-103","P-103",
    "LM-027","LM-027",
    "BT24-089","BT24-089","BT24-089",
]

results = []

def record(tid, card, effect, expected, actual, passed):
    results.append({'id': tid, 'card': card, 'effect': effect, 'pass': passed,
                    'expected': expected, 'actual': actual})
    s = 'PASS' if passed else 'FAIL'
    print(f'  [{s}] {tid}: {card} -- {effect}')
    if not passed:
        print(f'         Expected: {expected} | Actual: {actual}')

def create(deck1, mem=3):
    r = requests.post(f'{API}/debug/games', json={
        'deck1': deck1, 'deck2': DECK2,
        'player1_type': 'human', 'player2_type': 'human',
        'first_player': 1, 'skip_shuffle': True,
        'auto_mulligan': 'keep', 'initial_memory': mem,
        'agent_action_delay_ms': 0,
    }).json()
    if 'game_id' not in r:
        print(f'ERROR creating game: {r}')
        return None
    return r['game_id']

def act(gid, a):
    return requests.post(f'{API}/games/{gid}/actions', json={'action': a}).json()

def state(gid):
    return requests.get(f'{API}/games/{gid}/state').json()

def set_mem(gid, m):
    requests.post(f'{API}/debug/games/{gid}/set-memory', json={'memory': m})

def inject(gid, pid, cid, zone):
    requests.post(f'{API}/debug/games/{gid}/inject-card', json={
        'player_id': pid, 'card_id': cid, 'zone': zone
    })

def pass_to_next_p1_main(gid):
    """Pass through P1 end, P2 full turn, P1 breeding to get back to P1 main."""
    act(gid, 62)  # P1 pass
    act(gid, 62)  # P2 breeding
    act(gid, 62)  # P2 pass
    act(gid, 62)  # P1 breeding

def get_descs(r):
    return r.get('action_descriptions', {})

# ============================================================
# 1. P-035 Red Memory Boost
# ============================================================
print('\n=== P-035 Red Memory Boost ===')
deck1 = list(MEDUSA_DECK)
# Rearrange: put BT23-005, P-035 first in hand
eggs = [c for c in deck1 if c == 'BT21-001']
non_eggs = [c for c in deck1 if c != 'BT21-001']
# Remove what we want from non_eggs and put at front
wanted_hand = ['BT23-005', 'P-035']
remaining = list(non_eggs)
for w in wanted_hand:
    remaining.remove(w)
deck1 = eggs + wanted_hand + remaining

gid = create(deck1, mem=10)
if gid:
    act(gid, 62)  # breeding
    act(gid, 0)   # play BT23-005 (cost 0)
    r = act(gid, 0)  # play P-035 (cost 3)
    s = r['state']
    record('P035-1', 'P-035', 'Play cost 3 (10->7)', 'mem=7', f'mem={s["memoryGauge"]}', s['memoryGauge'] == 7)
    record('P035-2', 'P-035', 'Main effect reveals from deck', 'phase=12', f'phase={s["currentPhase"]}', s['currentPhase'] == 12)
    record('P035-3', 'P-035', 'No trash pop', 'trash=[]', f'trash={s["player1"]["trashIds"]}', len(s['player1']['trashIds']) == 0)

    # Select from reveal (pick first), then pass turns, activate delay
    if s['currentPhase'] == 12:
        act(gid, 30)  # pick first revealed card (or decline)
    act(gid, 62)  # P1 pass turn
    act(gid, 62)  # P2 breeding
    act(gid, 62)  # P2 pass
    act(gid, 62)  # P1 breeding
    # Check delay action available
    s = state(gid)
    descs_check = requests.post(f'{API}/games/{gid}/actions', json={'action': -1})  # invalid action to just get descs
    mask = requests.get(f'{API}/games/{gid}/action-mask').json()
    delay_available = mask[1011] > 0 if len(mask) > 1011 else False
    record('P035-4', 'P-035', 'Delay available next turn', 'True', str(delay_available), delay_available)

    if delay_available:
        mem_before = s['memoryGauge']
        r = act(gid, 1011)  # activate delay
        mem_after = r['state']['memoryGauge']
        record('P035-5', 'P-035', 'Delay gain 2 memory', f'+2', f'+{mem_after-mem_before}', mem_after - mem_before == 2)
        trash = r['state']['player1']['trashIds']
        record('P035-6', 'P-035', 'Delay trashes P-035', 'P-035 in trash', str(trash), 'P-035' in trash)

# ============================================================
# 2. BT21-008 Elizamon On Play Reveal
# ============================================================
print('\n=== BT21-008 Elizamon On Play ===')
deck1 = list(MEDUSA_DECK)
eggs = [c for c in deck1 if c == 'BT21-001']
non_eggs = [c for c in deck1 if c != 'BT21-001']
wanted = ['BT21-008']
remaining = list(non_eggs)
for w in wanted:
    remaining.remove(w)
deck1 = eggs + wanted + remaining

gid = create(deck1, mem=10)
if gid:
    act(gid, 62)  # breeding
    r = act(gid, 0)  # play BT21-008 (cost 3)
    s = r['state']
    record('BT21008-1', 'BT21-008', 'Play cost 3', 'mem=7', f'mem={s["memoryGauge"]}', s['memoryGauge'] == 7)
    record('BT21008-2', 'BT21-008', 'On Play reveals top 3', 'phase=12, 3 revealed',
           f'phase={s["currentPhase"]}, {len(s["revealedCards"])} revealed',
           s['currentPhase'] == 12 and len(s['revealedCards']) == 3)
    record('BT21008-3', 'BT21-008', 'Reveals from deck not trash', 'trash=[]',
           f'trash={s["player1"]["trashIds"]}', len(s['player1']['trashIds']) == 0)

# ============================================================
# 3. BT24-008 Elizamon On Play
# ============================================================
print('\n=== BT24-008 Elizamon On Play ===')
deck1 = list(MEDUSA_DECK)
eggs = [c for c in deck1 if c == 'BT21-001']
non_eggs = [c for c in deck1 if c != 'BT21-001']
wanted = ['BT24-008', 'BT21-017']  # BT21-017 has Reptile/LIBERATOR for trash target
remaining = list(non_eggs)
for w in wanted:
    remaining.remove(w)
deck1 = eggs + wanted + remaining

gid = create(deck1, mem=10)
if gid:
    act(gid, 62)  # breeding
    r = act(gid, 0)  # play BT24-008 (cost 3)
    s = r['state']
    record('BT24008-1', 'BT24-008', 'Play cost 3', 'mem=7', f'mem={s["memoryGauge"]}', s['memoryGauge'] == 7)
    record('BT24008-2', 'BT24-008', 'On Play triggers hand selection', 'phase=11',
           f'phase={s["currentPhase"]}', s['currentPhase'] == 11)

    if s['currentPhase'] == 11:
        # Hand is: BT21-017 + remaining cards (BT24-008 was played)
        # Trash first hand card (BT21-017) -> then draw 2
        # Original hand was 5, minus BT24-008 played = 4 cards in hand
        hand_before_trash = len(s['player1']['handIds'])
        r = act(gid, 0)  # select first hand card to trash
        s = r['state']
        hand_after = len(s['player1']['handIds'])
        # After fix: trash 1 then draw 2 = hand_before - 1 + 2 = hand_before + 1
        expected = hand_before_trash - 1 + 2
        record('BT24008-3', 'BT24-008', 'Trash 1 then Draw 2',
               f'hand={expected} ({hand_before_trash}-1+2)',
               f'hand={hand_after}', hand_after == expected)

# ============================================================
# 4. BT23-005 Elizamon evo cost reduction
# ============================================================
print('\n=== BT23-005 Evo Cost Reduction ===')
deck1 = list(MEDUSA_DECK)
eggs = [c for c in deck1 if c == 'BT21-001']
non_eggs = [c for c in deck1 if c != 'BT21-001']
wanted = ['BT23-005', 'BT21-017']
remaining = list(non_eggs)
for w in wanted:
    remaining.remove(w)
deck1 = eggs + wanted + remaining

gid = create(deck1, mem=10)
if gid:
    act(gid, 62)  # breeding
    act(gid, 0)   # play BT23-005 (cost 0)
    # Digivolve BT21-017 onto BT23-005 (hand=0, field=0 -> 400)
    # BT21-017 evo cost from Red Lv.3 = 2, minus BT23-005 reduction -1 = 1
    r = act(gid, 400)
    s = r['state']
    mem = s['memoryGauge']
    record('BT23005-1', 'BT23-005', 'Evo cost reduction -1 (2->1)', 'mem=9', f'mem={mem}', mem == 9)
    # Check draw 1 bonus
    hand_count = len(s['player1']['handIds'])
    # Original hand: 5 - BT23-005 (played) - BT21-017 (digivolved) + 1 draw = 4
    record('BT23005-2', 'BT23-005', 'Draw 1 evo bonus', 'hand=4', f'hand={hand_count}', hand_count == 4)

# ============================================================
# 5. BT21-081 Owen Start of Main + End of Turn
# ============================================================
print('\n=== BT21-081 Owen ===')
deck1 = list(MEDUSA_DECK)
eggs = [c for c in deck1 if c == 'BT21-001']
non_eggs = [c for c in deck1 if c != 'BT21-001']
wanted = ['BT21-081', 'BT23-005']
remaining = list(non_eggs)
for w in wanted:
    remaining.remove(w)
deck1 = eggs + wanted + remaining

gid = create(deck1, mem=10)
if gid:
    act(gid, 62)  # breeding
    r = act(gid, 0)  # play Owen (cost 3)
    s = r['state']
    record('BT21081-1', 'BT21-081', 'Play cost 3', 'mem=7', f'mem={s["memoryGauge"]}', s['memoryGauge'] == 7)

    # Play Elizamon to have a Digimon
    act(gid, 0)
    # Pass to P2
    act(gid, 62)
    # P2 play Terriermon to have opponent Digimon
    act(gid, 62)  # P2 breeding
    act(gid, 0)   # P2 play Terriermon
    # Set memory to 0 (P1 perspective) so when P2 passes, P1 gets 0 memory
    set_mem(gid, 0)
    act(gid, 62)  # P2 pass

    # P1 turn 3 starts - Owen should fire "Start of Main: if opponent has Digimon, gain 1 memory"
    act(gid, 62)  # P1 breeding
    s = state(gid)
    mem = s['memoryGauge']
    # Owen "Start of Main: if opponent has Digimon, gain 1 memory"
    # Starting memory was 0, Owen should add 1 = 1
    # But "Start of Main" means it fires after breeding, at start of main phase
    record('BT21081-2', 'BT21-081', 'Start of Main: gain 1 memory (opp has Digimon)',
           'mem>=1', f'mem={mem}', mem >= 1)

# ============================================================
# 6. BT23-048 Gotsumon On Play Reveal
# ============================================================
print('\n=== BT23-048 Gotsumon ===')
deck_hudie = [
    'BT22-005','BT22-005','BT22-005','BT22-005',
    'BT23-048','BT22-043','BT22-044','BT22-043','BT22-044',
    'BT22-043','BT22-043',
    'BT22-044','BT22-044',
    'BT23-048','BT23-048','BT23-048',
    'BT23-027','BT23-027','BT23-027','BT23-027',
    'BT23-050','BT23-050','BT23-050','BT23-050',
    'BT23-101','BT23-101',
    'BT23-020','BT23-020','BT23-020',
    'BT23-032','BT23-032','BT23-032',
    'BT16-025','BT16-025','BT16-025',
    'BT22-089','BT22-089','BT22-089',
    'BT23-081','BT23-081','BT23-081',
    'BT23-090','BT23-090',
    'BT22-099','BT22-099','BT22-099',
    'BT16-082','BT16-082','BT16-082','BT16-082',
]
gid = create(deck_hudie, mem=10)
if gid:
    act(gid, 62)  # breeding
    r = act(gid, 0)  # play BT23-048 (cost 3)
    s = r['state']
    record('BT23048-1', 'BT23-048', 'Play cost 3', 'mem=7', f'mem={s["memoryGauge"]}', s['memoryGauge'] == 7)
    record('BT23048-2', 'BT23-048', 'On Play reveals from deck', 'phase=12, trash=[]',
           f'phase={s["currentPhase"]}, trash={s["player1"]["trashIds"]}',
           s['currentPhase'] == 12 and len(s['player1']['trashIds']) == 0)
    revealed = [c['cardId'] for c in s['revealedCards']]
    record('BT23048-3', 'BT23-048', 'Reveals 3 cards', f'3 revealed',
           f'{len(revealed)} revealed: {revealed}', len(revealed) == 3)

# ============================================================
# SUMMARY
# ============================================================
print(f'\n{"="*60}')
print(f'QA TEST RESULTS')
print(f'{"="*60}')
passed = sum(1 for r in results if r['pass'])
failed = sum(1 for r in results if not r['pass'])
total = len(results)
print(f'Total: {total} | Passed: {passed} | Failed: {failed}')
print()
for r in results:
    s = 'PASS' if r['pass'] else 'FAIL'
    print(f'  [{s}] {r["id"]}: {r["card"]} -- {r["effect"]}')
    if not r['pass']:
        print(f'         Expected: {r["expected"]} | Actual: {r["actual"]}')
