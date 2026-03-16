# Plan: Restructure CLAUDE.md (~250 lines) + docs index

## Context

CLAUDE.md is 493 lines mixing high-level project orientation with deep implementation details (API routes, UI component lists, animation keyframes, WebSocket protocol, etc.). Goal: slim to ~250 lines of project vision, tech stack, directory tree, commands, and working rules. Move all detailed reference material into dedicated docs files linked from CLAUDE.md and a new `docs/INDEX.md`.

## What stays in CLAUDE.md (~250 lines)

| Section | Lines | Notes |
|---------|-------|-------|
| Scope | ~3 | Keep as-is |
| Project Vision | ~5 | Keep as-is |
| System Overview | ~15 | 3 services + 5 surfaces |
| Service Boundaries | ~18 | Import constraints + requirements |
| Directory Tree | ~55 | NEW — replaces "Key Repository Paths" |
| Commands | ~50 | Keep as-is |
| Working Rules | ~18 | Keep as-is |
| Pinecone MCP | ~20 | Trim: setup + namespace table only |
| Documentation Pointers | ~10 | Links to docs/INDEX.md + key refs |
| **Total** | **~194** | Well under 250 target |

## What moves out → new docs files

| CLAUDE.md Section | New File | ~Lines |
|---|---|---|
| RL and Game Contracts (lines 97-159) | `docs/RL_CONTRACTS.md` | 65 |
| Backend API Surface (lines 161-233) | `docs/API_SURFACE.md` | 75 |
| Frontend Surface (lines 235-320) | `docs/FRONTEND_SURFACE.md` | 90 |
| Admin AI Workflow (lines 322-343) | `docs/ADMIN_AI.md` | 20 |
| Desktop Distribution (lines 345-372) | `docs/DESKTOP.md` | 30 |
| QA Artifacts (lines 446-449) | Covered by tree + INDEX.md | — |
| Key Repository Paths (lines 50-95) | Replaced by directory tree | — |

## Directory tree (replaces Key Repository Paths)

```
.
├── CLAUDE.md, AGENTS.md, README.md
├── requirements*.txt                # 3 dep profiles (full, desktop, training)
├── digimon_gym/
│   ├── api.py                       # Hosted API entry (FastAPI)
│   ├── desktop_main.py              # Desktop sidecar entry (no DB)
│   ├── digimon_gym.py               # DigimonEnv (Gymnasium)
│   ├── engine/
│   │   ├── game/                    # Rules engine, tensor, action mask/decode
│   │   ├── core/                    # CardSource, Permanent, Player
│   │   ├── data/                    # cards.json, enums, registry, scripts/
│   │   │   └── scripts/             # Card effect scripts (frozen + generated)
│   │   ├── interfaces/              # ICardEffect, modifiers
│   │   └── validation/              # Digivolve, DigiXros validators
│   ├── agents/                      # RL training, gauntlet, deck pool, architect
│   │   └── maskable_recurrent/      # Custom PPO + LSTM + action masking
│   ├── routers/                     # Engine-only HTTP + WebSocket routes
│   ├── db/                          # SQLAlchemy models, auth, DB-backed routers
│   └── ai/                          # Admin AI task pipeline
├── frontend/src/
│   ├── pages/                       # GamePage, LobbyPage, DeckBuilder, Admin*
│   ├── components/board/            # GameBoard, HandZone, MemoryGauge, BattleArea
│   ├── components/game/             # ActionBar, PhaseBanner, DigivolveBanner
│   ├── api/                         # REST + WebSocket clients
│   └── utils/                       # constants.ts, actionDecoder.ts
├── src-tauri/                       # Tauri v2 desktop shell (Rust)
├── tools/                           # CLI tools, transpiler, Pinecone ingestion
│   ├── transpiler/                  # C# → Python transpiler package
│   └── archive/                     # One-time migration/fix scripts
├── qa/
│   ├── archetype-qa/                # Per-archetype QA, engine API ref, gaps
│   └── qa-reports/                  # Gameplay QA reports, validated cards
├── docs/                            # Project documentation (see docs/INDEX.md)
├── tests/                           # pytest suite (3400+ tests)
├── DCGO/                            # Git submodule — C# reference implementation
└── data/, models/                   # External datasets, ONNX models
```

## `docs/INDEX.md` structure

```markdown
# Documentation Index

## Contracts & Specs
- TENSOR_SPEC.md — observation tensor layout (1375 floats)
- ACTION_SPEC.md — action space (2168 actions)
- RL_CONTRACTS.md — environment API, reward shaping, wrapper chain, phases
- RULES_CONTEXT.md — official Digimon TCG rules reference

## Architecture
- API_SURFACE.md — backend routes, WebSocket protocol, lobby, ONNX inference
- FRONTEND_SURFACE.md — React pages, game UI components, animations, data flow
- DESKTOP.md — Tauri v2 dual-server architecture, build profiles, working rules
- ADMIN_AI.md — admin AI task pipeline, scope profiles, batch operations

## Operations
- TOOLS.md — CLI tools, transpiler, Pinecone, model export, new-set workflow
- TRAINING_RUNBOOK.md — RL training operations
- ../AGENTS.md — RL pipeline, wrapper chain, gauntlet

## QA
- ../qa/archetype-qa/ — per-archetype QA, engine API ref, engine gaps
- ../qa/qa-reports/ — gameplay test reports, validated cards index
```

## Execution steps

### Step 1: Create 5 extracted doc files
Move content verbatim from CLAUDE.md into:
- `docs/RL_CONTRACTS.md` — "## RL and Game Contracts" through end of "Training Pipeline"
- `docs/API_SURFACE.md` — "## Backend API Surface" through end of "Admin AI Routes"
- `docs/FRONTEND_SURFACE.md` — "## Frontend Surface" through end of "Frontend Action/Phase Constants"
- `docs/DESKTOP.md` — "## Desktop Distribution (Tauri v2)" through end
- `docs/ADMIN_AI.md` — "## Admin AI Workflow (Current)" section + cross-ref to admin_ai_batch_runbook.md

### Step 2: Create `docs/INDEX.md`

### Step 3: Rewrite CLAUDE.md
- Keep: Scope, Project Vision, System Overview, Service Boundaries
- Replace "Key Repository Paths" → directory tree
- Keep: Commands, Working Rules
- Trim Pinecone (drop CLI examples — they're in TOOLS.md)
- Add short "Documentation" section pointing to docs/INDEX.md
- Target: ~250 lines

### Step 4: Verify
1. `wc -l CLAUDE.md` — confirm ~250
2. Grep for broken cross-references
3. All content preserved in new docs files

## Files modified
- `CLAUDE.md` — rewritten (~250 lines)
- `docs/INDEX.md` — NEW
- `docs/RL_CONTRACTS.md` — NEW
- `docs/API_SURFACE.md` — NEW
- `docs/FRONTEND_SURFACE.md` — NEW
- `docs/DESKTOP.md` — NEW
- `docs/ADMIN_AI.md` — NEW
