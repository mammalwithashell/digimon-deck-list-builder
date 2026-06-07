#!/usr/bin/env python3
"""One-shot facade decomposition: move all `&mut self` action methods out of
effect_context/mod.rs into action/<mechanic>.rs submodules.

Behavior-preserving CODE MOVEMENT. Compiler-gated afterwards.
- Spans located by rustfmt invariant: method starts at `^    (pub...)?fn NAME`,
  ends at first subsequent `^    }$`; preceding `///`/`//`/`#[..]` lines included.
- Private `fn`/`async fn` (no pub) are promoted to `pub(crate)` so cross-module
  helper calls keep compiling (visibility widening is always safe).
- All spans computed against the ORIGINAL file, removed in one pass.
"""
import re
import sys

ROOT = sys.argv[1]  # path to effect_context dir
MOD = ROOT + '/mod.rs'
FLOOR = 844

PREAMBLE = """#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
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
"""

DOCS = {
 'play': "Play / put-into-play mutations on `EffectContext` — extracted by mechanic.",
 'digivolve': "Digivolve / DNA-digivolve mutations on `EffectContext` — extracted by mechanic.",
 'trash': "Trash mutations (security, sources, hand) on `EffectContext` — extracted by mechanic.",
 'sources': "Digivolution-source / under-tamer placement on `EffectContext` — extracted by mechanic.",
 'security': "Security-stack mutations on `EffectContext` — extracted by mechanic.",
 'zones': "Draw / reveal / shuffle / hand-and-deck movement on `EffectContext` — extracted by mechanic.",
 'combat': "Attack / battle / counter mutations on `EffectContext` — extracted by mechanic.",
 'modifiers': "Modifier / keyword / immunity grants on `EffectContext` — extracted by mechanic.",
 'digixros': "DigiXros transaction mutations on `EffectContext` — extracted by mechanic.",
 'suspend': "Suspend / unsuspend mutations on `EffectContext` — extracted by mechanic.",
 'replacement': "Replacement-window handling on `EffectContext` — extracted by mechanic.",
 'refire': "Effect re-fire machinery on `EffectContext` — extracted by mechanic.",
 'lifecycle': "Permanent lifecycle / misc mutations on `EffectContext` — extracted by mechanic.",
}

def bucket(n):
    for k,t in [
     ('refire',lambda n:n.startswith('refire')),
     ('play',lambda n:n.startswith('play_') or n=='hatch'),
     ('digivolve',lambda n:'digivolve' in n),
     ('trash',lambda n:n.startswith('trash') or n.startswith('de_digivolve') or 'digisources_from_trash' in n),
     ('digixros',lambda n:'digixros' in n),
     ('sources',lambda n:'under_tamer' in n or 'under_source_tamer' in n or '_source' in n.replace('digisources','') or 'as_bottom' in n or 'under_permanent' in n or 'attach_tamer' in n or 'union_card' in n or 'place_top_source' in n or 'armor_purge' in n or 'training_place' in n or 'place_deck_top' in n or 'remove_source' in n),
     ('security',lambda n:'security' in n),
     ('combat',lambda n:'attack' in n or n.startswith('may_') or 'battle' in n or 'counter' in n or 'force_opponent' in n or n in ('cancel_attack','cancel_pending_attack','cleanup_exposed_battle_area_digi_egg')),
     ('modifiers',lambda n:'modifier' in n or 'keyword' in n or n.startswith('grant') or 'immunity' in n or 'protection' in n or 'add_dp' in n or 'ignore_option_color' in n),
     ('replacement',lambda n:'replacement' in n or n=='cancel_leave' or 'substitute' in n),
     ('zones',lambda n:any(x in n for x in['add_to_hand','return_to_hand','bounce','return_to_deck','return_stack','reveal','draw','shuffle','add_top','add_bottom','add_pending','recover','place_remainder','revealed','return_trash','move_trash_card','move_from_breeding'])),
     ('suspend',lambda n:'suspend' in n),
    ]:
        if t(n): return k
    return 'lifecycle'

HDR = re.compile(r'^    (pub(\([^)]*\))? )?(async )?fn ([a-z0-9_]+)')
PRE = re.compile(r'^    (///|//[^/]|//$|#\[)')
PRIV = re.compile(r'^    (async )?fn ')

def main():
    lines = open(MOD, encoding='utf-8').read().split('\n')
    # gather &mut self action methods >= FLOOR
    chosen = {}  # name -> (start,end)
    order = []
    for i,l in enumerate(lines):
        if i+1 < FLOOR: continue
        m = HDR.match(l)
        if not m: continue
        name = m.group(4)
        sig = ' '.join(lines[i:i+8])
        if '&mut self' not in sig.split(')')[0]: continue
        # span
        start = i; j = i-1
        while j>=0 and PRE.match(lines[j]): start=j; j-=1
        end=None
        for k in range(i,len(lines)):
            if lines[k]=='    }': end=k; break
        if end is None: raise SystemExit(f"no brace for {name}")
        if name in chosen: raise SystemExit(f"dup {name}")
        chosen[name]=(start,end,i); order.append(name)
    # bucket
    from collections import defaultdict
    buckets=defaultdict(list)
    for n in order: buckets[bucket(n)].append(n)
    # overlap check
    spans=sorted((s,e,n) for n,(s,e,_) in chosen.items())
    for a in range(len(spans)-1):
        if spans[a][1]>=spans[a+1][0]:
            raise SystemExit(f"overlap {spans[a][2]} / {spans[a+1][2]}")
    # emit module files
    for bk,names in buckets.items():
        names_sorted=sorted(names, key=lambda n: chosen[n][0])
        bodies=[]
        for n in names_sorted:
            s,e,h=chosen[n]
            body=lines[s:e+1]
            # promote private fn -> pub(crate)
            hdr_idx=h-s
            if PRIV.match(body[hdr_idx]):
                body[hdr_idx]=body[hdr_idx].replace('    fn ','    pub(crate) fn ',1).replace('    async fn ','    pub(crate) async fn ',1)
            bodies.append('\n'.join(body))
        doc=DOCS.get(bk,bk)
        content=f"//! {doc}\n\n"+PREAMBLE+"\nimpl<'a> EffectContext<'a> {\n"+'\n\n'.join(bodies)+'\n}\n'
        open(f"{ROOT}/action/{bk}.rs",'w',encoding='utf-8').write(content)
        print(f"  {bk}: {len(names_sorted)} methods")
    # rewrite mod.rs without moved spans
    remove=set()
    for n,(s,e,_) in chosen.items():
        for k in range(s,e+1): remove.add(k)
        if e+1<len(lines) and lines[e+1]=='': remove.add(e+1)
    kept=[ln for idx,ln in enumerate(lines) if idx not in remove]
    open(MOD,'w',encoding='utf-8').write('\n'.join(kept))
    # rewrite action/mod.rs
    allmods=sorted(buckets.keys())
    hdr="""//! Action mutations on `EffectContext`, split by game mechanic.
//!
//! Each submodule holds `impl EffectContext` blocks for one mechanic.
//! `EffectContext` remains a single type; only its `impl` is split across
//! files for readability — the same technique used for `impl Game` across
//! 14 files and the `selections` sibling module.

"""
    open(f"{ROOT}/action/mod.rs",'w',encoding='utf-8').write(hdr+'\n'.join(f"mod {m};" for m in allmods)+'\n')
    print(f"total moved: {len(chosen)}; modules: {allmods}")

if __name__=='__main__': main()
