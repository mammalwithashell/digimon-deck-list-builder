import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { getCardById } from '@/api/digimonCardApi';
import type { DigimonCardData } from '@/types/cards';
import type { DeckFolder, DeckResponse, DeckSummary } from '@/types/deck';
import { COLOR_HEX } from '@/utils/constants';
import {
  buildDeckAnalytics,
  deckInitials,
  deckToExportText,
  filterAndSortDecks,
  formatRelativeTime,
  type DeckSortKey,
} from '@/utils/deckLibrary';
import './DeckLibraryPage.css';

type ViewMode = 'grid' | 'list';

const FALLBACK_COLORS = ['#2563eb', '#dc2626', '#16a34a', '#9333ea'];

function colorForName(name: string): string {
  return COLOR_HEX[name] ?? '#64748b';
}

function DeckSleeve({
  deck,
  cards,
  large = false,
}: {
  deck: DeckSummary | DeckResponse;
  cards?: Map<string, DigimonCardData>;
  large?: boolean;
}) {
  const colors = useMemo(() => {
    if (cards && 'main_deck' in deck) {
      const found = new Set<string>();
      for (const id of [...deck.main_deck, ...deck.egg_deck]) {
        const card = cards.get(id);
        if (card?.color) found.add(card.color);
        if (card?.color2) found.add(card.color2);
      }
      if (found.size > 0) return Array.from(found).map(colorForName);
    }
    if ('colors' in deck && deck.colors?.length) return deck.colors.map(colorForName);
    return FALLBACK_COLORS.slice(0, 1 + (deck.name.length % 3));
  }, [cards, deck]);

  return (
    <div className={`library-sleeve ${large ? 'large' : ''}`}>
      <div className="library-sleeve-split">
        {colors.map((color, index) => (
          <span key={`${color}-${index}`} style={{ background: color }} />
        ))}
      </div>
      <span className="library-sleeve-mark">{deckInitials(deck.name)}</span>
      <span className="library-sleeve-level">
        {'highest_level' in deck && deck.highest_level ? `L${deck.highest_level}` : 'DECK'}
      </span>
    </div>
  );
}

function DeckTile({
  deck,
  selected,
  view,
  onSelect,
  onTogglePin,
}: {
  deck: DeckSummary;
  selected: boolean;
  view: ViewMode;
  onSelect: (id: string) => void;
  onTogglePin: (deck: DeckSummary) => void;
}) {
  const validity = deck.is_valid ? 'Legal' : 'Draft';
  const edited = formatRelativeTime(deck.updated_at);
  return (
    <button
      type="button"
      className={`library-card ${selected ? 'selected' : ''} ${view === 'list' ? 'list' : ''}`}
      onClick={() => onSelect(deck.id)}
    >
      <div className="library-card-thumb">
        <DeckSleeve deck={deck} />
        <button
          type="button"
          className={`library-pin ${deck.is_pinned ? 'on' : ''}`}
          onClick={(event) => {
            event.stopPropagation();
            onTogglePin(deck);
          }}
          title={deck.is_pinned ? 'Unpin deck' : 'Pin deck'}
        >
          {deck.is_pinned ? '◆' : '◇'}
        </button>
      </div>
      <div className="library-card-body">
        <div className="library-card-name">{deck.name}</div>
        <div className="library-card-meta">
          <span className={deck.is_valid ? 'good' : 'warn'}>{validity}</span>
          <span>{deck.main_count}/{deck.egg_count}</span>
        </div>
        <div className="library-card-meta">
          <span>{deck.meta_archetype ?? deck.meta_tier ?? 'Unclassified'}</span>
          <span>{edited}</span>
        </div>
      </div>
    </button>
  );
}

export function DeckLibraryPage() {
  const navigate = useNavigate();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [folders, setFolders] = useState<DeckFolder[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selectedDeck, setSelectedDeck] = useState<DeckResponse | null>(null);
  const [selectedCards, setSelectedCards] = useState(new Map<string, DigimonCardData>());
  const [activeFolder, setActiveFolder] = useState('all');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState<DeckSortKey>('recent');
  const [view, setView] = useState<ViewMode>('grid');
  const [loading, setLoading] = useState(true);
  const [notice, setNotice] = useState('');

  const refresh = async () => {
    setLoading(true);
    try {
      const [nextDecks, nextFolders] = await Promise.all([
        library.listDecks(),
        library.listDeckFolders(),
      ]);
      setDecks(nextDecks);
      setFolders(nextFolders);
      setSelectedId((current) => current ?? nextDecks[0]?.id ?? null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setSelectedDeck(null);
      setSelectedCards(new Map());
      return;
    }
    let cancelled = false;
    async function loadSelectedDeck() {
      try {
        const deck = await library.getDeck(selectedId!);
        const ids = [...new Set([...deck.main_deck, ...deck.egg_deck])];
        const cardPairs = await Promise.allSettled(
          ids.map(async (id) => [id, await getCardById(id)] as const),
        );
        const cards = new Map<string, DigimonCardData>();
        for (const result of cardPairs) {
          if (result.status === 'fulfilled' && result.value[1]) {
            cards.set(result.value[0], result.value[1]);
          }
        }
        if (!cancelled) {
          setSelectedDeck(deck);
          setSelectedCards(cards);
        }
      } catch {
        if (!cancelled) {
          setSelectedDeck(null);
          setSelectedCards(new Map());
        }
      }
    }
    void loadSelectedDeck();
    return () => {
      cancelled = true;
    };
  }, [selectedId]);

  const filtered = useMemo(
    () => filterAndSortDecks(decks, { activeFolder, search, sort }),
    [activeFolder, decks, search, sort],
  );
  const pinned = filtered.filter((deck) => deck.is_pinned);
  const others = activeFolder === 'pinned' ? [] : filtered.filter((deck) => !deck.is_pinned);
  const selectedSummary = decks.find((deck) => deck.id === selectedId) ?? filtered[0] ?? null;
  const analytics = useMemo(
    () => buildDeckAnalytics(selectedDeck, selectedCards),
    [selectedCards, selectedDeck],
  );
  const maxCurve = Math.max(...analytics.levelCurve, 1);

  const folderCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const deck of decks) {
      if (deck.folder_id) counts.set(deck.folder_id, (counts.get(deck.folder_id) ?? 0) + 1);
    }
    return counts;
  }, [decks]);

  const togglePin = async (deck: DeckSummary) => {
    await library.updateDeckLibraryFields(deck.id, { is_pinned: !deck.is_pinned });
    setDecks((current) =>
      current.map((item) =>
        item.id === deck.id ? { ...item, is_pinned: !deck.is_pinned } : item,
      ),
    );
  };

  const createFolder = async () => {
    const name = window.prompt('Folder name');
    if (!name?.trim()) return;
    const folder = await library.createDeckFolder(name.trim());
    setFolders((current) => [...current, folder].sort((a, b) => a.sort_order - b.sort_order));
    setActiveFolder(folder.id);
  };

  const renameSelected = async () => {
    if (!selectedSummary) return;
    const name = window.prompt('Deck name', selectedSummary.name);
    if (!name?.trim()) return;
    const updated = await library.updateDeck(selectedSummary.id, { name: name.trim() });
    setDecks((current) =>
      current.map((deck) => (deck.id === updated.id ? { ...deck, name: updated.name } : deck)),
    );
    setSelectedDeck(updated);
  };

  const moveSelected = async (folderId: string) => {
    if (!selectedSummary) return;
    const nextFolderId = folderId === 'none' ? null : folderId;
    await library.updateDeckLibraryFields(selectedSummary.id, { folder_id: nextFolderId });
    setDecks((current) =>
      current.map((deck) =>
        deck.id === selectedSummary.id ? { ...deck, folder_id: nextFolderId } : deck,
      ),
    );
  };

  const duplicateSelected = async () => {
    if (!selectedSummary) return;
    const copy = await library.duplicateDeck(selectedSummary.id);
    await refresh();
    setSelectedId(copy.id);
    setNotice('Deck duplicated');
  };

  const deleteSelected = async () => {
    if (!selectedSummary) return;
    if (!window.confirm(`Delete ${selectedSummary.name}?`)) return;
    await library.deleteDeck(selectedSummary.id);
    setDecks((current) => current.filter((deck) => deck.id !== selectedSummary.id));
    setSelectedId((current) => {
      if (current !== selectedSummary.id) return current;
      return decks.find((deck) => deck.id !== selectedSummary.id)?.id ?? null;
    });
  };

  const exportSelected = async () => {
    if (!selectedDeck) return;
    await navigator.clipboard.writeText(deckToExportText(selectedDeck, selectedCards));
    setNotice('Export copied');
  };

  const copySelectedUrl = async () => {
    if (!selectedSummary) return;
    await navigator.clipboard.writeText(`${window.location.origin}/deckbuilder/${selectedSummary.id}`);
    setNotice('URL copied');
  };

  return (
    <div className="deck-library-page">
      <div className="library-topbar">
        <div className="library-brand">
          <span className="library-brand-mark">D</span>
          <div>
            <strong>Deck Library</strong>
            <small>{loading ? 'Syncing' : `${decks.length} saved decks`}</small>
          </div>
        </div>
        <div className="library-crumb">
          <span>Home</span>
          <span>/</span>
          <strong>{activeFolder === 'all' ? 'All Decks' : activeFolder === 'pinned' ? 'Pinned' : folders.find((f) => f.id === activeFolder)?.name}</strong>
          <span>{filtered.length} items</span>
        </div>
        <div className="library-actions">
          {notice && <span className="library-live">{notice}</span>}
          <button type="button" onClick={() => navigate('/deckbuilder/new')}>New Deck</button>
        </div>
      </div>

      <div className="library-shell">
        <aside className="library-sidebar">
          <div className="library-group-title">
            <span>Folders</span>
            <button type="button" onClick={createFolder} title="New folder">+</button>
          </div>
          <button
            type="button"
            className={`library-folder ${activeFolder === 'all' ? 'active' : ''}`}
            onClick={() => setActiveFolder('all')}
          >
            <span>All Decks</span>
            <b>{decks.length}</b>
          </button>
          <button
            type="button"
            className={`library-folder ${activeFolder === 'pinned' ? 'active' : ''}`}
            onClick={() => setActiveFolder('pinned')}
          >
            <span>Pinned</span>
            <b>{decks.filter((deck) => deck.is_pinned).length}</b>
          </button>
          {folders.map((folder) => (
            <button
              type="button"
              key={folder.id}
              className={`library-folder ${activeFolder === folder.id ? 'active' : ''}`}
              onClick={() => setActiveFolder(folder.id)}
            >
              <span>{folder.name}</span>
              <b>{folderCounts.get(folder.id) ?? 0}</b>
            </button>
          ))}

          <div className="library-stats-box">
            <div><span>Total</span><b>{decks.length}</b></div>
            <div><span>Pinned</span><b>{decks.filter((deck) => deck.is_pinned).length}</b></div>
            <div><span>Legal</span><b>{decks.filter((deck) => deck.is_valid).length}</b></div>
            <div><span>Drafts</span><b>{decks.filter((deck) => !deck.is_valid).length}</b></div>
          </div>
        </aside>

        <main className="library-main">
          {selectedSummary ? (
            <section className="library-banner">
              <DeckSleeve deck={selectedDeck ?? selectedSummary} cards={selectedCards} large />
              <div className="library-banner-copy">
                <h1>{selectedSummary.name}</h1>
                <div className="library-pills">
                  <span>{selectedSummary.is_valid ? 'Legal' : 'Draft'}</span>
                  <span>{selectedSummary.main_count}/50 main</span>
                  <span>{selectedSummary.egg_count}/5 eggs</span>
                  <span>{selectedSummary.meta_archetype ?? selectedSummary.meta_tier ?? 'Unclassified'}</span>
                  <span>Edited {formatRelativeTime(selectedSummary.updated_at)} ago</span>
                </div>
                <p>{selectedSummary.description || 'No notes saved for this deck.'}</p>
              </div>
              <div className="library-banner-actions">
                <button type="button" className="primary" onClick={() => navigate(`/deckbuilder/${selectedSummary.id}`)}>Edit</button>
                <button type="button" onClick={exportSelected}>Export</button>
                <button type="button" onClick={duplicateSelected}>Duplicate</button>
                <button type="button" onClick={() => navigate(`/game?deckId=${selectedSummary.id}`)}>Test Draw</button>
                <button type="button" className="danger" onClick={deleteSelected}>Delete</button>
              </div>
            </section>
          ) : (
            <section className="library-empty-banner">
              <h1>No Decks Yet</h1>
              <button type="button" onClick={() => navigate('/deckbuilder/new')}>Create your first deck</button>
            </section>
          )}

          <section className="library-toolbar">
            <label className="library-search">
              <span>Search</span>
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Decks, archetypes, tags"
              />
            </label>
            <div className="library-segment">
              <button type="button" className={view === 'grid' ? 'on' : ''} onClick={() => setView('grid')}>Grid</button>
              <button type="button" className={view === 'list' ? 'on' : ''} onClick={() => setView('list')}>List</button>
            </div>
            <select value={sort} onChange={(event) => setSort(event.target.value as DeckSortKey)}>
              <option value="recent">Recent</option>
              <option value="name">A to Z</option>
              <option value="validity">Legal first</option>
              <option value="count">Card count</option>
            </select>
          </section>

          <section className="library-scroll">
            {pinned.length > 0 && (
              <>
                <div className="library-section-title"><span>Pinned</span><b>{pinned.length}</b></div>
                <div className={`library-card-grid ${view}`}>
                  {pinned.map((deck) => (
                    <DeckTile
                      key={deck.id}
                      deck={deck}
                      selected={deck.id === selectedSummary?.id}
                      view={view}
                      onSelect={setSelectedId}
                      onTogglePin={togglePin}
                    />
                  ))}
                </div>
              </>
            )}
            <div className="library-section-title"><span>{activeFolder === 'pinned' ? 'Other Decks' : 'All'}</span><b>{others.length}</b></div>
            <div className={`library-card-grid ${view}`}>
              <button type="button" className="library-new-card" onClick={() => navigate('/deckbuilder/new')}>
                <span>+</span>
                <b>New Deck</b>
              </button>
              {others.map((deck) => (
                <DeckTile
                  key={deck.id}
                  deck={deck}
                  selected={deck.id === selectedSummary?.id}
                  view={view}
                  onSelect={setSelectedId}
                  onTogglePin={togglePin}
                />
              ))}
            </div>
          </section>
        </main>

        <aside className="library-detail">
          <div className="library-detail-head">
            <span>Deck Analytics</span>
            <b>Live</b>
          </div>
          <section>
            <h2>Color Identity</h2>
            {analytics.colors.length === 0 ? (
              <p className="library-muted">Select a saved deck to load card metadata.</p>
            ) : (
              analytics.colors.map((color) => (
                <div className="library-meter" key={color.name}>
                  <span>{color.name}</span>
                  <i><b style={{ width: `${color.pct}%`, background: colorForName(color.name) }} /></i>
                  <em>{color.pct}%</em>
                </div>
              ))
            )}
          </section>
          <section>
            <h2>Level Curve</h2>
            <div className="library-curve">
              {analytics.levelCurve.map((count, level) => (
                <div key={level}>
                  <span style={{ height: `${Math.max((count / maxCurve) * 100, count ? 12 : 4)}%` }}>{count || ''}</span>
                  <b>{level === 7 ? '7+' : level}</b>
                </div>
              ))}
            </div>
          </section>
          <section>
            <h2>Build Info</h2>
            <div className="library-kvs">
              <div><span>Mainboard</span><b>{selectedSummary?.main_count ?? 0}/50</b></div>
              <div><span>Egg Deck</span><b>{selectedSummary?.egg_count ?? 0}/5</b></div>
              <div><span>Avg Cost</span><b>{analytics.averagePlayCost}</b></div>
              <div><span>Highest</span><b>{analytics.highestLevel ? `L${analytics.highestLevel}` : '-'}</b></div>
            </div>
          </section>
          <section>
            <h2>Quick Actions</h2>
            <div className="library-quick">
              <button type="button" onClick={renameSelected} disabled={!selectedSummary}>Rename</button>
              <select
                disabled={!selectedSummary}
                value={selectedSummary?.folder_id ?? 'none'}
                onChange={(event) => void moveSelected(event.target.value)}
              >
                <option value="none">No folder</option>
                {folders.map((folder) => (
                  <option key={folder.id} value={folder.id}>{folder.name}</option>
                ))}
              </select>
              <button type="button" onClick={copySelectedUrl} disabled={!selectedSummary}>Copy URL</button>
              <button type="button" onClick={() => selectedSummary && void togglePin(selectedSummary)} disabled={!selectedSummary}>
                {selectedSummary?.is_pinned ? 'Unpin' : 'Pin'}
              </button>
            </div>
          </section>
        </aside>
      </div>
    </div>
  );
}
