# BT27 Royal Knights / Glowing Dawn restart

Base: main `349816e5c45a120629ba668ffdd6810a67554642` (2026-11-22).
Branch: `codex/bt27-royal-knights-glowing-dawn`.
This is a fresh implementation effort; historical patches were not recovered.

## Grounded first slices

Official sources inspected on 2026-11-22:
- https://world.digimoncard.com/cards/?category=522040&search=true
- https://world.digimoncard.com/products/pack/ver27/

| Priority | Card | Required behavior / first test boundaries | Current status |
|---|---|---|---|
| 1 | BT27-099 Surtr Brander | DP reduction; security exchange; lowest-DP opponent security placement with tied choices; effect-trash security activation | Not authored, not admitted |
| 2 | BT27-044 Gekkomon | Green BEATBREAK digivolve discount only on own turn outside breeding; own-turn inherited DP | Not authored, not admitted |
| 3 | BT27-056 Pristimon | Black BEATBREAK discount with same boundaries; inherited Reboot | Not authored, not admitted |

Bandai Q7710 and Q7759 explicitly exclude breeding-area activation for the two discounts. Image and detailed Q&A review precede authoring. The product guide identifies Royal Knights and Glowing Dawn support; this three-card checkpoint does not claim a complete set inventory.

Overlap reservations: lane B owns BT27-062; BT27-059/063/070/100 remain reserved for prior Craniamon work.

## Canonical admission boundary

Current main has 4,483 cards and no BT27 card YAML directory. All three IDs are absent from canonical metadata and stable registration. Keep all existing assignments unchanged. No temporary fixture registration proves production admission.

Baseline SHA-256:
- data/card_registry.json: `9a604ff3e4feb20a16ff8ce38febf866191d6d2b9697162e5b7e002332799599`
- data/cards.json: `bc3fd974b51e36a56e360068604a7abec2e321751d52229ff967a6331ccd5ee1`

Historical full-pool acquisition stopped at 84 older condition conflicts. That cache is unavailable here; current bindings and full-pool drift have not been reverified. Do not alter those conflicts or the canonical ledger for this slice. Production admission remains blocked pending approved canonical ingestion and stable allocation.

Initial verification: `git ls-remote origin refs/heads/main refs/heads/codex/bt27-royal-knights-glowing-dawn` found the base above and no existing feature branch; JSON inspection counted canonical cards and confirmed target IDs absent. No behavioral tests run at this inventory checkpoint.

## First native batch

BT27-099: authored YAML; 5/5 focused native tests pass (10.29 s). Test-first baseline: 0 passed / 5 failed, missing embedded BT27-099. Covers selected DP loss and expiration, exchange with empty/nonempty security, direct effect-trash with empty board and tied lowest-DP choices, and ordinary security check. Not production admitted. Q7956 timing and Q7957 revealed-security negative paths still need dedicated regression coverage.

Command: `RUST_MIN_STACK=67108864 sh ../bt27-lane-a-runtime/cargo-run test -p digimon-engine --test bt27_slice -- --test-threads=1`. The external wrapper sets lane-local CARGO_HOME/RUSTUP_HOME/CARGO_TARGET_DIR, dev opt/debug=0, ORT_LIB_LOCATION and LD_LIBRARY_PATH to the isolated onnxruntime 1.24.2 wheel's capi directory, and ORT_PREFER_DYNAMIC_LINK=1. Native toolchain: Rust 1.100.0. The original ORT download CDN refused connections; the matching wheel supplies the native library. Default test stack overflowed; 64 MiB test-thread stack passes. No engine changes.

Production artifact probe: `PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=code python -c 'from digimon_gym.agents.card_admissibility import CardAdmissibility; a=CardAdmissibility.from_artifact(); print({c:a.reasons(c) for c in ["BT27-099","BT27-044","BT27-056"]})'` reports `[unregistered, untested_lowering]` for all three.

Remote push of first inventory checkpoint failed with `fatal: could not read Username for 'https://github.com': No such device or address`. No feature ref was published; do not infer remote persistence from local commits.

## Second native batch

BT27-044: 8/8 focused tests pass (10.42 s), after 8 missing-YAML baseline failures. Covered: matching target, wrong color, absent trait, other carrier, breeding (Q7710), opponent turn, inherited DP on own/opponent turn and no inherited bonus on the top card. Alternative Glowing Dawn egg route is authored but still needs an explicit integrated route regression. No canonical registration or admission claim.

## Third native batch

BT27-056: 8/8 focused tests pass (11.26 s), after 8 missing-YAML baseline failures. Matching/wrong-color/missing-trait/other-carrier/opponent-turn/breeding discount cases pass. Inherited Reboot unsuspends the evolved host during the opponent's unsuspend phase; a top-card Pristimon does not inherit Reboot (CR 4.3 16-11). Canonical admission remains blocked.
