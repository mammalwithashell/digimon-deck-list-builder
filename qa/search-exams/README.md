# Search exams

A search exam is a real recorded position where a model's search should do
something specific: give a line a real look, avoid a bad play, find a win. It
is a microscope for *why* search plays badly, not a strength measurement. One
position says little about overall play, so judge a search change with
anchored games (CLAUDE.md rule 30) and use exams to explain and regress
specific mistakes.

Each `<id>.json` holds everything needed to rebuild the position offline:
both decklists, the game seed, the recorded actions before the decision, the
searching seat, its information regime (and the frozen deck-list prior for
`closed-list`), the search budget and seed, the model's sha256, the
expectations, optional lines to score with the value head, and the search the
panel actually recorded.

## Grade a model

```bash
cargo run --release -p digimon-engine --example search_exam -- \
  qa/search-exams/<id>.json --model <model.onnx> --seeds 8
```

Seed 0 uses the recorded search seed. It reproduces the `recorded` visits
exactly only on the panel's engine build (same commit, `portable-policy-math`
feature); on any other build the tree differs slightly, so treat seed 0 like
any other seed and read the pass counts, not one seed's visits. Each expectation prints as "passed k of N seeds".
`--simulations` / `--worlds` grade other budgets. A different model is
allowed (the runner notes the hash mismatch), which is how to check whether a
newer checkpoint fixed a mistake.

## Save a new exam

Positions come from completed frozen-evaluation panels (the same games the
`/watch` queue replays):

```bash
python code/tools/search_exam/make_exam.py --panel <panel dir> \
  --request match-015-1 --game 1 --decision 26 --id <exam-id> \
  --title "..." --why "..." \
  --expect visits:1002:0.10 --expect avoid:4 \
  --line name=4,100
```

`--decision` is the number of recorded actions before the position (the
watch overlay's move counter when the move is "next"). Expectations:
`visits:<plan>:<share>`, `avoid:<plan>`, `choose:<plan>`; a multi-action plan
joins ids with `+`.

## Exams

| Exam | Model | What goes wrong | Baseline (320 sims, 8 seeds) |
|---|---|---|---|
| `ex13-examon-training-before-wingdramon` | checkpoint8 | Plays Wingdramon from hand (opponent gets 6 memory) instead of Sprint Dash Training → Coredramon → Wingdramon (opponent gets 2). The value head scores the half-resolved Training effect at −0.76, so the line gets ~2% of visits. | Training ≥10% visits 0/8; Wingdramon avoided 6/8 |
