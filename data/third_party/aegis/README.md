# Aegis compiled-effect IR (vendored, LOW trust)

Pinned snapshot of `packages/shared/src/effects/effects.json` from
[Aegis](https://github.com/vinicius3333/aegis-digimon-tcg) (MIT — see `LICENSE`),
a third-party Digimon TCG simulator. It maps `cardId` → `CompiledCard`:
`effects[]` (each with `trigger`, an ordered `actions[]`, filters, `optional`,
`frequency`, `isInherited`, …) plus self-reported `coverage` / `residual`, and
digivolve / DNA / DigiXros / Assembly / Link / App-Fusion requirement fields.

| | |
|---|---|
| Upstream commit | `ec0cd22f0e51f373a78a5546ca5dfa0e2a772c4b` |
| Records | 4481 card IDs (incl. 9 promos + 1 token we lack) |
| Raw size | 7,630,214 B (git blob, LF) — stored gzipped (~0.5 MB) |
| Integrity | `SNAPSHOT.json` holds the commit, raw sha256 and record count |

`effects.json.gz` is the upstream blob byte-for-byte, gzipped with `mtime=0` so a
refresh at the same commit is byte-identical.

## Trust — read before using

This is a **structural hint only**, ranked **below** every project source:
the card image / official bundle (`data/card_bundles/`), DCGO C#,
`general_rule.pdf`, and `card_overrides.json` / `cards.json` (see CLAUDE.md
"Source priority"). Use it only to cross-check how a card's text decomposes into
clauses (trigger, action order, optionality, once-per-turn). Every record claims
`"coverage": "full"` — that is Aegis's own audit, not ours. **Never copy its
digivolve / Assembly / DNA / DigiXros / Link / App-Fusion requirement data**;
`code/tools/aegis_ir.py` strips those fields from everything it renders.

## Use

```bash
python code/tools/aegis_ir.py BT15-003          # labeled context-pack section
python code/tools/aegis_ir.py --check           # verify the snapshot
```

`from aegis_ir import aegis_section, aegis_record` (with `code/tools` on
`sys.path`) for tooling. The `/digimon-card-lookup` resolver prints the section
for ID / name queries.

## Refresh

```bash
python code/tools/aegis_ir.py refresh --commit <sha>                      # fresh blobless clone to a temp dir
python code/tools/aegis_ir.py refresh --clone D:/tmp_aegis/a --commit <sha>  # reuse a local clone
```

Then update the commit row above and commit `effects.json.gz`, `LICENSE` and
`SNAPSHOT.json` together. On Windows, clone to a short path with
`core.longpaths=true` (the refresh command sets it).
