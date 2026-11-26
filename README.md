# EX13 Small-Pool Hybrid Search ↔ LSTM PPO (R1 Artifacts)

Temporary standalone orphan branch containing the trained models and held-out evaluation bundle from `feat/small-pool-hybrid-search-lstm-ppo` (`41a10a0f`).

## Contents
- `models/onnx/`: `lstm_ppo_r1.onnx`, `lstm_distilled_r1.onnx`, `gen6_hybrid.onnx`, `gen6_control.onnx`, `gen3.onnx`, `gen0_seed.onnx` (+ `.onnx.meta.json` sidecars)
- `models/sb3/`: full SB3/PyTorch `.zip` checkpoints (+ `.meta.json` sidecars)
- `eval/`: `final_evaluation.json`, stage gates, `exams/` (Examon `<Training>` search exam runs), `ex13_small3_hybrid_results.md`, and `ex13_match_replay_theater.html`
- `SHA256SUMS.txt`: checksums for all model and evaluation files

## Quick Download
```bash
git clone --single-branch --branch artifacts/ex13-small3-hybrid-r1 --depth 1 \
  https://github.com/mammalwithashell/digimon-deck-list-builder.git ex13-hybrid-artifacts
```

## Cleanup After Downloading
```bash
git push origin --delete artifacts/ex13-small3-hybrid-r1
```
