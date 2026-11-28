## ADDED Requirements

### Requirement: Unified Studio Library With Folders and Tags
The Studio SHALL provide a **Library UI** (`/studio` landing view and slide-over library drawer) displaying and organizing both:
1. **Training Puzzles & Exams**: canonical Studio puzzles (`qa/puzzles/**/*.json`), search exams (`qa/search-exams/*.json`), and scenario fixtures (`qa/scenarios/*.json`).
2. **Replays**: saved DB recordings (`/recordings`), frozen-evaluation `/watch` queue games, and local DCGO JSONL recordings (`qa/dcgo-mod-recordings/*.jsonl`).

The Library SHALL support:
- **Hierarchical folders**: browsing a folder tree, creating/renaming/deleting folders, and moving any puzzle, exam, or replay into a folder.
- **Tags**: adding/removing arbitrary kebab-case tags on any item, filtering by one or more tags (AND/OR), and full-text search across title, ID, card IDs, tags, and item kind.
- **Quick actions** per item: open puzzle/exam in Compose or Play & Record / Model Playout mode, open replay in the Replay Stepper, duplicate puzzle, or delete Studio-owned puzzle.

#### Scenario: Organize puzzles and replays into folders and filter by tags
- **WHEN** the user creates folder `"examon/lethal"`, moves a `qa/puzzles/` puzzle and a `qa/dcgo-mod-recordings/` replay into `"examon/lethal"`, tags both with `"training-option"`, and filters the Library by folder `"examon/lethal"` + tag `"training-option"`
- **THEN** both the puzzle and the replay appear together under `"examon/lethal"` with their tags preserved across page reloads

#### Scenario: Authored puzzles persist folder structure on disk without relocating external replays
- **WHEN** the user moves authored puzzle `"ex13-training-lethal"` into folder `"examon/lethal"` and assigns folder `"examon/lethal"` to external replay `qa/dcgo-mod-recordings/mod_game2.jsonl`
- **THEN** the puzzle file is stored at `qa/puzzles/examon/lethal/ex13-training-lethal.json`, `mod_game2.jsonl` remains at its original path so existing CLIs do not break, and its folder/tag assignment is persisted in `qa/puzzles/_library.json`

### Requirement: Resilient Library Discovery and Orphan Safety
The backend `GET /studio/library` endpoint SHALL discover live items directly from the filesystem and DB/watch sources and merge folder/tag metadata from `qa/puzzles/_library.json`. Missing or externally deleted files referenced in `_library.json` SHALL NOT cause errors when listing the Library, and all folder/file mutations MUST reject path traversal outside allowlisted `qa/` directories.

#### Scenario: Externally deleted replay does not break the Library
- **WHEN** a replay entry in `qa/puzzles/_library.json` points to a file that was deleted outside the Studio and the user opens the Library
- **THEN** `GET /studio/library` succeeds with HTTP 200, omits the missing file from active listings, and preserves all valid items and folders
