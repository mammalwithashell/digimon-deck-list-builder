#!/usr/bin/env python3
"""One-shot refactor helper: move a named list of `impl EffectContext` methods
out of effect_context/mod.rs into a new mechanic submodule file.

Behavior-preserving CODE MOVEMENT only. Compiler-gated afterwards.

Heuristics (valid for this rustfmt-formatted file):
- A method starts at a line matching `^    (pub )?(async )?fn NAME(<|\(| )` and
  ends at the first subsequent line that is exactly `^    }$` (4-space indent).
- Preceding contiguous `    ///`, `    //`, `    #[...]` lines belong to the method.
- Search is restricted to lines >= FLOOR (the mutation impl region) so duplicated
  read-accessor names in earlier impl blocks are never matched.
- Each requested name MUST match exactly one method header in the region, else abort.

Usage:
  python extract_facade_module.py <mod.rs> <new_file.rs> <floor_line> <doc_title> name1 name2 ...
The new file is written with a broad glob-import preamble (warnings are allowed
in this crate) and an `impl<'a> EffectContext<'a>` wrapper. mod.rs is rewritten
with the moved spans removed. Prints a summary; exits non-zero on any ambiguity.
"""
import re
import sys

PREAMBLE_IMPORTS = """#![allow(unused_imports)]
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

HEADER_RE = lambda name: re.compile(r'^    (pub(\([^)]*\))? )?(async )?fn ' + re.escape(name) + r'(<|\(|\b)')
PRE_RE = re.compile(r'^    (///|//[^/]|//$|#\[)')


def find_span(lines, name, floor):
    spans = []
    hdr = HEADER_RE(name)
    for i, ln in enumerate(lines):
        if i + 1 < floor:
            continue
        if hdr.match(ln):
            # walk back over preamble (doc/attr) lines
            start = i
            j = i - 1
            while j >= 0 and PRE_RE.match(lines[j]):
                start = j
                j -= 1
            # walk forward to first 4-space closing brace
            end = None
            for k in range(i, len(lines)):
                if lines[k] == '    }':
                    end = k
                    break
            if end is None:
                raise SystemExit(f"!! no closing brace for {name} at line {i+1}")
            spans.append((start, end, i))
    return spans


def main():
    mod_path, new_path, floor_s, doc_title = sys.argv[1:5]
    names = sys.argv[5:]
    floor = int(floor_s)
    text = open(mod_path, encoding='utf-8').read()
    lines = text.split('\n')

    chosen = []  # (start,end,name)
    for name in names:
        spans = find_span(lines, name, floor)
        if len(spans) != 1:
            raise SystemExit(f"!! {name}: expected 1 match in region >= {floor}, found {len(spans)} at lines {[s[2]+1 for s in spans]}")
        s, e, h = spans[0]
        chosen.append((s, e, name))

    # detect overlaps (a method accidentally swallowing another)
    chosen.sort()
    for a in range(len(chosen) - 1):
        if chosen[a][1] >= chosen[a + 1][0]:
            raise SystemExit(f"!! overlap between {chosen[a][2]} and {chosen[a+1][2]}")

    # extract bodies (preserve original order by line)
    bodies = []
    for s, e, name in chosen:
        bodies.append('\n'.join(lines[s:e + 1]))

    # write new module file
    doc = '\n'.join('//! ' + l if l else '//!' for l in doc_title.split('\n'))
    new_content = doc + '\n\n' + PREAMBLE_IMPORTS + '\nimpl<\'a> EffectContext<\'a> {\n' + '\n\n'.join(bodies) + '\n}\n'
    open(new_path, 'w', encoding='utf-8').write(new_content)

    # rewrite mod.rs without the chosen spans (and drop one trailing blank line each)
    remove = set()
    for s, e, _ in chosen:
        for k in range(s, e + 1):
            remove.add(k)
        # absorb a single following blank separator line if present
        if e + 1 < len(lines) and lines[e + 1] == '':
            remove.add(e + 1)
    kept = [ln for idx, ln in enumerate(lines) if idx not in remove]
    open(mod_path, 'w', encoding='utf-8').write('\n'.join(kept))

    print(f"moved {len(chosen)} methods -> {new_path}")
    for s, e, name in chosen:
        print(f"  {name}  (lines {s+1}-{e+1}, {e-s+1} lines)")


if __name__ == '__main__':
    main()
