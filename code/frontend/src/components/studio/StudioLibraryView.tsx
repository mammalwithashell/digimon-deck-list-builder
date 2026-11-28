import { useEffect, useMemo, useState } from 'react';
import { getStudioTransport } from '@/api/studioApi';
import type { LibraryItem } from '@/api/studioTransport';
import { useStudioStore } from '@/stores/studioStore';

type KindFilter = 'all' | 'puzzles' | 'replays';

function tagChipClass(tag: string, active: boolean, isAuto = false): string {
  if (active) return 'studio-chip is-active';
  if (tag === 'diverged') return 'studio-chip studio-chip--danger';
  if (tag === 'clean' || tag === 'no-concede') return 'studio-chip studio-chip--ok';
  if (tag === 'error' || tag === 'concede' || tag === 'unfinished') return 'studio-chip studio-chip--warn';
  if (isAuto) return 'studio-chip studio-chip--auto';
  return 'studio-chip';
}

export function StudioLibraryView() {
  const {
    library,
    deckTemplates,
    refreshLibrary,
    createBlankSession,
    createSessionFromDecks,
    openPuzzle,
    openSearchExam,
    openScenario,
    loadReplay,
  } = useStudioStore();

  const [selectedFolder, setSelectedFolder] = useState<string>('');
  const [kindFilter, setKindFilter] = useState<KindFilter>('all');
  const [noConcedeOnly, setNoConcedeOnly] = useState<boolean>(false);
  const [search, setSearch] = useState('');
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [newFolderName, setNewFolderName] = useState('');
  const [editingItem, setEditingItem] = useState<LibraryItem | null>(null);
  const [editTitle, setEditTitle] = useState('');
  const [editFolder, setEditFolder] = useState('');
  const [editTags, setEditTags] = useState('');
  const [deck1Id, setDeck1Id] = useState('blank-filler');
  const [deck2Id, setDeck2Id] = useState('blank-filler');

  const analysisRunning = Boolean(library?.analysis_status?.running);
  useEffect(() => {
    if (!analysisRunning) return;
    const id = window.setInterval(() => {
      void refreshLibrary();
    }, 3000);
    return () => window.clearInterval(id);
  }, [analysisRunning, refreshLibrary]);

  const allTags = useMemo(() => {
    const s = new Set<string>();
    for (const item of library?.items ?? []) {
      for (const t of item.tags) s.add(t);
    }
    return [...s].sort();
  }, [library]);

  const filteredItems = useMemo(() => {
    const q = search.trim().toLowerCase();
    return (library?.items ?? []).filter((item) => {
      if (kindFilter === 'puzzles' && !['puzzle', 'search_exam', 'scenario'].includes(item.kind)) {
        return false;
      }
      if (kindFilter === 'replays' && !['dcgo_jsonl', 'recording'].includes(item.kind)) {
        return false;
      }
      if (noConcedeOnly && item.finish_type !== 'natural') {
        return false;
      }
      if (selectedFolder) {
        if (item.folder !== selectedFolder && !item.folder.startsWith(`${selectedFolder}/`)) {
          return false;
        }
      }
      if (selectedTags.length > 0 && !selectedTags.every((t) => item.tags.includes(t))) {
        return false;
      }
      if (q) {
        const hay = `${item.id} ${item.title} ${item.folder} ${item.tags.join(' ')} ${item.kind}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  }, [library, kindFilter, noConcedeOnly, selectedFolder, selectedTags, search]);

  const handleCreateFolder = async () => {
    const cleaned = newFolderName.trim().toLowerCase();
    if (!cleaned) return;
    await getStudioTransport().createFolder(cleaned);
    setNewFolderName('');
    await refreshLibrary();
  };

  const handleDeleteFolder = async (folder: string) => {
    await getStudioTransport().deleteFolder(folder);
    if (selectedFolder === folder) setSelectedFolder('');
    await refreshLibrary();
  };

  const handleSaveItemMeta = async () => {
    if (!editingItem) return;
    const tags = editTags
      .split(',')
      .map((t) => t.trim().toLowerCase())
      .filter(Boolean);
    await getStudioTransport().updateLibraryItem({
      item_key: editingItem.item_key,
      title_override: editTitle.trim(),
      folder: editFolder.trim(),
      tags,
    });
    setEditingItem(null);
    await refreshLibrary();
  };

  const handleStartFromTemplates = () => {
    const d1 = deckTemplates.find((d) => d.id === deck1Id)?.cards ?? [];
    const d2 = deckTemplates.find((d) => d.id === deck2Id)?.cards ?? [];
    if (d1.length > 0 && d2.length > 0) {
      void createSessionFromDecks(d1, d2);
    } else {
      void createBlankSession();
    }
  };

  const handleOpenItem = (item: LibraryItem, mode: 'compose' | 'play' = 'compose') => {
    if (item.kind === 'puzzle') {
      void openPuzzle(item.id, mode);
    } else if (item.kind === 'search_exam') {
      void openSearchExam(item.id, mode);
    } else if (item.kind === 'scenario') {
      void openScenario(item.id, mode);
    } else if (item.kind === 'dcgo_jsonl') {
      void loadReplay({ source: 'dcgo_jsonl', ref: item.ref, verify: true });
    } else if (item.kind === 'recording') {
      void loadReplay({ source: 'recording_id', ref: item.ref, verify: false });
    }
  };

  return (
    <div className="studio-library" data-testid="studio-library-view">
      {/* Left sidebar: folders + tag filters */}
      <aside className="studio-panel studio-library__sidebar">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div className="studio-panel__title">Folders</div>
          <button
            type="button"
            className={`studio-btn ${selectedFolder === '' ? 'studio-btn--primary' : ''}`}
            style={{ justifyContent: 'space-between' }}
            onClick={() => setSelectedFolder('')}
          >
            <span>All Items</span>
            <span>{library?.items.length ?? 0}</span>
          </button>
          {(library?.folders ?? []).map((f) => (
            <div key={f} style={{ display: 'flex', gap: 4 }}>
              <button
                type="button"
                className={`studio-btn ${selectedFolder === f ? 'studio-btn--primary' : ''}`}
                style={{ flex: 1, justifyContent: 'flex-start', overflow: 'hidden', textOverflow: 'ellipsis' }}
                onClick={() => setSelectedFolder(f)}
                title={f}
              >
                📁 {f}
              </button>
              <button
                type="button"
                className="studio-btn studio-btn--sm studio-btn--danger"
                title={`Remove folder ${f}`}
                onClick={() => void handleDeleteFolder(f)}
              >
                ×
              </button>
            </div>
          ))}
          <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
            <input
              className="studio-input"
              style={{ flex: 1, minWidth: 0 }}
              placeholder="examon/lethal"
              value={newFolderName}
              onChange={(e) => setNewFolderName(e.target.value)}
            />
            <button
              type="button"
              className="studio-btn studio-btn--sm"
              onClick={() => void handleCreateFolder()}
            >
              + Folder
            </button>
          </div>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div className="studio-panel__title">Tags</div>
          <div className="studio-pill-group">
            {allTags.length === 0 && (
              <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>No tags yet</span>
            )}
            {allTags.map((t) => {
              const active = selectedTags.includes(t);
              return (
                <button
                  key={t}
                  type="button"
                  className={tagChipClass(t, active)}
                  onClick={() =>
                    setSelectedTags((prev) =>
                      prev.includes(t) ? prev.filter((x) => x !== t) : [...prev, t],
                    )
                  }
                >
                  #{t}
                </button>
              );
            })}
          </div>
        </div>
      </aside>

      {/* Main area */}
      <div className="studio-library__main">
        <div className="studio-panel" style={{ flexDirection: 'row', flexWrap: 'wrap', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6 }}>
            <button
              type="button"
              data-testid="studio-new-blank"
              className="studio-btn studio-btn--primary"
              onClick={() => void createBlankSession()}
            >
              + Blank Board
            </button>
            <select
              className="studio-select"
              value={deck1Id}
              onChange={(e) => setDeck1Id(e.target.value)}
              aria-label="Player 1 deck template"
            >
              {deckTemplates.map((d) => (
                <option key={d.id} value={d.id}>
                  P1: {d.name}
                </option>
              ))}
            </select>
            <select
              className="studio-select"
              value={deck2Id}
              onChange={(e) => setDeck2Id(e.target.value)}
              aria-label="Player 2 deck template"
            >
              {deckTemplates.map((d) => (
                <option key={d.id} value={d.id}>
                  P2: {d.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="studio-btn"
              onClick={handleStartFromTemplates}
            >
              Compose From Decks
            </button>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {library?.analysis_status?.running && (
              <span className="studio-chip studio-chip--warn" style={{ cursor: 'default' }}>
                Verifying {library.analysis_status.analyzed}/{library.analysis_status.total}…
              </span>
            )}
            <button
              type="button"
              data-testid="studio-filter-no-concede"
              className={`studio-btn studio-btn--sm ${noConcedeOnly ? 'studio-btn--primary' : ''}`}
              title="Show only games that played out to 0 security or deck-out (no concede on either side)"
              onClick={() => setNoConcedeOnly((v) => !v)}
            >
              {noConcedeOnly ? '✓ No Concede Only' : 'No Concede Only'}
            </button>
            <input
              type="search"
              placeholder="Search puzzles, exams, replays..."
              className="studio-input"
              style={{ width: 230 }}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
            <div className="studio-seg">
              {(['all', 'puzzles', 'replays'] as KindFilter[]).map((k) => (
                <button
                  key={k}
                  type="button"
                  className={`studio-seg__btn ${kindFilter === k ? 'is-active' : ''}`}
                  onClick={() => setKindFilter(k)}
                >
                  {k.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="studio-table-wrap">
          <table className="studio-table">
            <thead>
              <tr>
                <th>Type</th>
                <th>Title / ID</th>
                <th>Folder</th>
                <th>Tags</th>
                <th style={{ textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredItems.length === 0 && (
                <tr>
                  <td colSpan={5} style={{ textAlign: 'center', padding: 28, color: 'var(--ink-3)' }}>
                    No matching puzzles, exams, or replays.
                  </td>
                </tr>
              )}
              {filteredItems.map((item) => (
                <tr key={item.item_key}>
                  <td>
                    <span className="studio-chip">{item.kind}</span>
                  </td>
                  <td>
                    <div style={{ fontWeight: 600, color: 'var(--ink-0)' }}>{item.title}</div>
                    <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--ink-3)' }}>{item.id}</div>
                  </td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--ink-2)' }}>
                    {item.folder ? `📁 ${item.folder}` : '—'}
                  </td>
                  <td>
                    <div className="studio-pill-group">
                      {item.tags.map((t) => {
                        const isAuto = Boolean(item.auto_tags?.includes(t));
                        return (
                          <span
                            key={t}
                            className={tagChipClass(t, false, isAuto)}
                            title={isAuto ? 'Auto-tag' : 'User tag'}
                            style={{ cursor: 'default' }}
                          >
                            #{t}
                          </span>
                        );
                      })}
                    </div>
                  </td>
                  <td style={{ textAlign: 'right' }}>
                    <div style={{ display: 'inline-flex', gap: 4 }}>
                      {item.kind === 'puzzle' && (
                        <>
                          <button
                            type="button"
                            className="studio-btn studio-btn--sm"
                            onClick={() => handleOpenItem(item, 'compose')}
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            className="studio-btn studio-btn--sm studio-btn--primary"
                            onClick={() => handleOpenItem(item, 'play')}
                          >
                            Playout
                          </button>
                        </>
                      )}
                      {item.kind === 'search_exam' && (
                        <>
                          <button
                            type="button"
                            className="studio-btn studio-btn--sm"
                            onClick={() => handleOpenItem(item, 'compose')}
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            className="studio-btn studio-btn--sm studio-btn--primary"
                            onClick={() => handleOpenItem(item, 'play')}
                          >
                            Open / Inspect
                          </button>
                        </>
                      )}
                      {item.kind === 'scenario' && (
                        <button
                          type="button"
                          className="studio-btn studio-btn--sm studio-btn--primary"
                          onClick={() => handleOpenItem(item, 'compose')}
                        >
                          Open in Studio
                        </button>
                      )}
                      {(item.kind === 'dcgo_jsonl' || item.kind === 'recording') && (
                        <button
                          type="button"
                          className="studio-btn studio-btn--sm studio-btn--primary"
                          onClick={() => handleOpenItem(item)}
                        >
                          Step Replay
                        </button>
                      )}
                      <button
                        type="button"
                        className="studio-btn studio-btn--sm"
                        onClick={() => {
                          setEditingItem(item);
                          setEditTitle(item.title);
                          setEditFolder(item.folder);
                          setEditTags((item.user_tags ?? item.tags).join(', '));
                        }}
                      >
                        Rename / Organize
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {editingItem && (
        <div className="studio-modal-backdrop">
          <div className="studio-modal">
            <div className="studio-panel__title">Rename & Organize</div>
            <label style={{ fontSize: 11, color: 'var(--ink-2)' }}>Display Title</label>
            <input
              className="studio-input"
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              placeholder="Give this replay or exam a memorable name"
            />
            <label style={{ fontSize: 11, color: 'var(--ink-2)' }}>Folder</label>
            <input
              className="studio-input"
              value={editFolder}
              onChange={(e) => setEditFolder(e.target.value)}
              placeholder="e.g. examon/lethal"
            />
            <label style={{ fontSize: 11, color: 'var(--ink-2)' }}>Tags (comma-separated)</label>
            <input
              className="studio-input"
              value={editTags}
              onChange={(e) => setEditTags(e.target.value)}
              placeholder="lethal, training-option"
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 6, marginTop: 6 }}>
              <button type="button" className="studio-btn" onClick={() => setEditingItem(null)}>
                Cancel
              </button>
              <button type="button" className="studio-btn studio-btn--primary" onClick={() => void handleSaveItemMeta()}>
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
