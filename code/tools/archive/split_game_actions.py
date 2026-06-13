#!/usr/bin/env python3
"""Split game_actions.rs (single `impl Game` block) into game_actions/<mechanic>.rs.

Mirrors the facade decomposition (extract_facade_all.py) for Tier 2.
Behavior-preserving CODE MOVEMENT. Compiler-gated afterwards.

- Operates on game_actions/mod.rs (after game_actions.rs is git-mv'd there).
- Moves &mut self methods with line >= FLOOR (the impl Game block); &self
  readers stay in mod.rs.
- Private `fn`/`async fn` promoted to `pub(crate)` (cross-module visibility).
- All spans computed against the ORIGINAL file, removed in one pass.
- `--dry-run` prints buckets without writing.
"""
import re, sys

ROOT = sys.argv[1]           # path to game_actions dir
MOD = ROOT + '/mod.rs'
FLOOR = int(sys.argv[2])
DRY = '--dry-run' in sys.argv

PREAMBLE = """#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;
use rand::seq::SliceRandom;
"""

DOCS = {
 'play': "Play / put-into-play operations (Tier 2) — `impl Game`, split by mechanic.",
 'digivolve': "Digivolve / DNA-digivolve operations (Tier 2) — `impl Game`.",
 'breeding': "Breeding-area operations (Tier 2) — `impl Game`.",
 'zones': "Zone movement (hand/deck/reveal/security/shuffle) operations (Tier 2) — `impl Game`.",
 'movement': "Return-to-hand / return-to-deck operations (Tier 2) — `impl Game`.",
 'sources': "Digivolution-source placement operations (Tier 2) — `impl Game`.",
 'security': "Security-stack placement operations (Tier 2) — `impl Game`.",
 'options': "Option-card play operations (Tier 2) — `impl Game`.",
 'cost': "Cost-reduction / before-pay-cost machinery (Tier 2) — `impl Game`.",
 'misc': "Misc Tier-2 operations — `impl Game`.",
}

def bucket(n):
    for k,t in [
     ('cost',  lambda n:'cost_reduc' in n or 'before_pay' in n or 'pay_cost' in n or 'observer_activation' in n or 'cost_reducer' in n or 'inspect_cost' in n),
     ('options',lambda n:'option' in n),
     ('breeding',lambda n:'breeding' in n or 'training' in n or 'bind_training' in n or 'hatch' in n),
     ('digivolve',lambda n:'digivolve' in n or 'dna' in n),
     ('play',  lambda n:n.startswith('play_') or 'fire_on_play' in n or 'fire_play_event' in n or 'activate_' in n and 'main' in n),
     ('movement',lambda n:n.startswith('return_to_hand') or n.startswith('return_to_deck') or 'return_stack' in n or n.startswith('return_')),
     ('zones', lambda n:any(x in n for x in['add_to_hand','reveal','shuffle','trash_from_hand','trash_from_reveal','add_to_hand_from_reveal'])),
     ('sources',lambda n:'as_bottom_source' in n or 'bottom_sources' in n or 'place_as_bottom' in n or '_source' in n),
     ('security',lambda n:'security' in n),
    ]:
        if t(n): return k
    return 'misc'

HDR = re.compile(r'^    (pub(\([^)]*\))? )?(async )?fn ([a-z0-9_]+)')
PRE = re.compile(r'^    (///|//[^/]|//$|#\[)')
PRIV = re.compile(r'^    (async )?fn ')

def main():
    lines = open(MOD, encoding='utf-8').read().split('\n')
    chosen = {}; order = []
    for i,l in enumerate(lines):
        if i+1 < FLOOR: continue
        m = HDR.match(l)
        if not m: continue
        name = m.group(4)
        sig = ' '.join(lines[i:i+8])
        if '&mut self' not in sig.split(')')[0]: continue
        start = i; j = i-1
        while j>=0 and PRE.match(lines[j]): start=j; j-=1
        end=None
        for k in range(i,len(lines)):
            if lines[k]=='    }': end=k; break
        if end is None: raise SystemExit(f"no brace for {name}")
        if name in chosen: raise SystemExit(f"dup {name}")
        chosen[name]=(start,end,i); order.append(name)
    from collections import defaultdict
    buckets=defaultdict(list)
    for n in order: buckets[bucket(n)].append(n)
    if DRY:
        for bk in sorted(buckets):
            print(f"## {bk} ({len(buckets[bk])})")
            print('   '+' '.join(buckets[bk]))
        print(f"TOTAL movable &mut self methods: {len(order)}")
        return
    spans=sorted((s,e,n) for n,(s,e,_) in chosen.items())
    for a in range(len(spans)-1):
        if spans[a][1]>=spans[a+1][0]:
            raise SystemExit(f"overlap {spans[a][2]} / {spans[a+1][2]}")
    for bk,names in buckets.items():
        names_sorted=sorted(names, key=lambda n: chosen[n][0])
        bodies=[]
        for n in names_sorted:
            s,e,h=chosen[n]; body=lines[s:e+1]; hi=h-s
            if PRIV.match(body[hi]):
                body[hi]=body[hi].replace('    fn ','    pub(crate) fn ',1).replace('    async fn ','    pub(crate) async fn ',1)
            bodies.append('\n'.join(body))
        content=f"//! {DOCS.get(bk,bk)}\n\n"+PREAMBLE+"\nimpl Game {\n"+'\n\n'.join(bodies)+'\n}\n'
        open(f"{ROOT}/{bk}.rs",'w',encoding='utf-8').write(content)
        print(f"  {bk}: {len(names_sorted)}")
    remove=set()
    for n,(s,e,_) in chosen.items():
        for k in range(s,e+1): remove.add(k)
        if e+1<len(lines) and lines[e+1]=='': remove.add(e+1)
    kept=[ln for idx,ln in enumerate(lines) if idx not in remove]
    open(MOD,'w',encoding='utf-8').write('\n'.join(kept))
    allmods=sorted(buckets.keys())
    decl='\n'.join(f"mod {m};" for m in allmods)
    print(f"total moved: {len(chosen)}; modules: {allmods}")
    print("ADD THESE to game_actions/mod.rs after the impl block:\n"+decl)

if __name__=='__main__': main()
