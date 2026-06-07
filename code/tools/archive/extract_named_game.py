#!/usr/bin/env python3
"""Move a NAMED list of `impl Game` methods from game/mod.rs into a submodule.

Behavior-preserving code movement (de_digivolve / facade-split pattern). Each
requested method must match exactly one `impl Game` method header at line>=FLOOR
(else abort). Private `fn`/`async fn` promoted to pub(crate). Spans located by
the rustfmt invariant: method ends at the first `^    }$`. Preceding
doc/attr lines travel with the method.

Usage: extract_named_game.py <mod.rs> <new.rs> <floor> <doc> name1 name2 ...
"""
import re, sys

MOD, NEW, FLOOR, DOC = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
NAMES = sys.argv[5:]

PREAMBLE = """#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;
"""

PRE = re.compile(r'^    (///|//[^/]|//$|#\[)')
PRIV = re.compile(r'^    (async )?fn ')

def hdr(name):
    return re.compile(r'^    (pub(\([^)]*\))? )?(async )?fn ' + re.escape(name) + r'(<|\(|\b)')

lines = open(MOD, encoding='utf-8').read().split('\n')
chosen = []
for name in NAMES:
    h = hdr(name); spans = []
    for i, l in enumerate(lines):
        if i + 1 < FLOOR:
            continue
        if h.match(l):
            start = i; j = i - 1
            while j >= 0 and PRE.match(lines[j]):
                start = j; j -= 1
            end = next((k for k in range(i, len(lines)) if lines[k] == '    }'), None)
            if end is None:
                raise SystemExit(f"!! no closing brace for {name}")
            spans.append((start, end, i))
    if len(spans) != 1:
        raise SystemExit(f"!! {name}: expected 1 match >= {FLOOR}, found {len(spans)} at {[s[2]+1 for s in spans]}")
    chosen.append((spans[0][0], spans[0][1], spans[0][2], name))

chosen.sort()
for a in range(len(chosen) - 1):
    if chosen[a][1] >= chosen[a + 1][0]:
        raise SystemExit(f"!! overlap {chosen[a][3]} / {chosen[a+1][3]}")

bodies = []
for s, e, h, name in chosen:
    body = lines[s:e + 1]; hi = h - s
    if PRIV.match(body[hi]):
        body[hi] = body[hi].replace('    fn ', '    pub(crate) fn ', 1).replace('    async fn ', '    pub(crate) async fn ', 1)
    bodies.append('\n'.join(body))

open(NEW, 'w', encoding='utf-8').write(f"//! {DOC}\n\n" + PREAMBLE + "\nimpl Game {\n" + '\n\n'.join(bodies) + '\n}\n')

remove = set()
for s, e, _, _ in chosen:
    for k in range(s, e + 1):
        remove.add(k)
    if e + 1 < len(lines) and lines[e + 1] == '':
        remove.add(e + 1)
open(MOD, 'w', encoding='utf-8').write('\n'.join(ln for idx, ln in enumerate(lines) if idx not in remove))
print(f"moved {len(chosen)} methods -> {NEW}")
for s, e, _, name in chosen:
    print(f"  {name} ({e-s+1} lines)")
