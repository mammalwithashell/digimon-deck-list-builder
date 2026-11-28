import { useState } from 'react';
import { useStudioStore } from '@/stores/studioStore';

interface PuzzleSaveExportModalProps {
  onClose: () => void;
}

export function PuzzleSaveExportModal({ onClose }: PuzzleSaveExportModalProps) {
  const {
    puzzleId,
    puzzleTitle,
    puzzleDescription,
    puzzleFolder,
    puzzleTags,
    puzzleSeat,
    rlGoal,
    setPuzzleMeta,
    saveCurrentPuzzle,
    exportCurrentScenario,
    exportCurrentSearchExam,
  } = useStudioStore();

  const [tagsText, setTagsText] = useState(puzzleTags.join(', '));
  const [busy, setBusy] = useState(false);

  const syncTags = () => {
    const parsed = tagsText
      .split(',')
      .map((t) => t.trim().toLowerCase())
      .filter(Boolean);
    setPuzzleMeta({ puzzleTags: parsed });
  };

  const run = async (fn: () => Promise<string | null>) => {
    syncTags();
    setBusy(true);
    await fn();
    setBusy(false);
  };

  return (
    <div className="studio-modal-backdrop">
      <div className="studio-modal" data-testid="studio-save-export-modal">
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span className="studio-panel__title">Save & Export Studio Puzzle</span>
          <button type="button" className="studio-btn studio-btn--sm" onClick={onClose}>
            ×
          </button>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
            Puzzle ID (kebab-case)
            <input
              className="studio-input"
              value={puzzleId}
              onChange={(e) => setPuzzleMeta({ puzzleId: e.target.value })}
            />
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
            Folder
            <input
              className="studio-input"
              placeholder="examon/lethal"
              value={puzzleFolder}
              onChange={(e) => setPuzzleMeta({ puzzleFolder: e.target.value })}
            />
          </label>
        </div>

        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
          Title
          <input
            className="studio-input"
            value={puzzleTitle}
            onChange={(e) => setPuzzleMeta({ puzzleTitle: e.target.value })}
          />
        </label>

        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
          Description / Pedagogy Note
          <textarea
            rows={2}
            className="studio-textarea"
            value={puzzleDescription}
            onChange={(e) => setPuzzleMeta({ puzzleDescription: e.target.value })}
          />
        </label>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
            Tags (comma-separated)
            <input
              className="studio-input"
              value={tagsText}
              onChange={(e) => setTagsText(e.target.value)}
            />
          </label>
          <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11, color: 'var(--ink-2)' }}>
            Primary Solver Seat
            <select
              className="studio-select"
              value={puzzleSeat}
              onChange={(e) => setPuzzleMeta({ puzzleSeat: Number(e.target.value) === 2 ? 2 : 1 })}
            >
              <option value={1}>Player 1</option>
              <option value={2}>Player 2</option>
            </select>
          </label>
        </div>

        <div className="studio-panel" style={{ background: 'var(--surface-sunken)' }}>
          <div className="studio-panel__title">RL Curriculum Goal Mode</div>
          <select
            className="studio-select"
            value={rlGoal.mode}
            onChange={(e) =>
              setPuzzleMeta({
                rlGoal: {
                  ...rlGoal,
                  mode: e.target.value === 'bounded_horizon' ? 'bounded_horizon' : 'playout_to_win',
                },
              })
            }
          >
            <option value="playout_to_win">Playout to Win (standard terminal +1/-1)</option>
            <option value="bounded_horizon">Bounded Horizon (win_assertions within K turns/actions)</option>
          </select>
          {rlGoal.mode === 'bounded_horizon' && (
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
              <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11 }}>
                Max Actions
                <input
                  type="number"
                  min={1}
                  className="studio-input"
                  value={rlGoal.max_actions ?? ''}
                  onChange={(e) =>
                    setPuzzleMeta({
                      rlGoal: {
                        ...rlGoal,
                        max_actions: e.target.value ? Number(e.target.value) : null,
                      },
                    })
                  }
                />
              </label>
              <label style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11 }}>
                Max Turns
                <input
                  type="number"
                  min={1}
                  className="studio-input"
                  value={rlGoal.max_turns ?? ''}
                  onChange={(e) =>
                    setPuzzleMeta({
                      rlGoal: {
                        ...rlGoal,
                        max_turns: e.target.value ? Number(e.target.value) : null,
                      },
                    })
                  }
                />
              </label>
            </div>
          )}
        </div>

        <div style={{ display: 'flex', flexWrap: 'wrap', justifyContent: 'flex-end', gap: 6, marginTop: 4 }}>
          <button type="button" className="studio-btn" onClick={onClose}>
            Close
          </button>
          <button
            type="button"
            className="studio-btn studio-btn--primary"
            disabled={busy}
            onClick={() => void run(saveCurrentPuzzle)}
          >
            Save to qa/puzzles/
          </button>
          <button
            type="button"
            className="studio-btn"
            disabled={busy}
            onClick={() => void run(exportCurrentScenario)}
          >
            Export Scenario
          </button>
          <button
            type="button"
            className="studio-btn"
            disabled={busy}
            onClick={() => void run(exportCurrentSearchExam)}
          >
            Export Search Exam
          </button>
        </div>
      </div>
    </div>
  );
}
