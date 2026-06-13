import { useEffect, useMemo, useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { ImportExport } from '@/components/deckbuilder/ImportExport';
import {
  getBuilderDeck,
  saveBuilderDeck,
} from '@/features/deck-builder/deckBuilderAdapter';
import { getCardImageUrl } from '@/utils/cardImages';
import {
  builderCardColorClass,
  deckEntriesToSlotArrays,
  filterBuilderCards,
  getBuilderCounts,
  groupDeckEntriesForBuilder,
  slotArraysToDeckEntries,
  type BuilderCardFilters,
} from '@/features/deck-builder/deckBuilderView';
import { getCardById, searchCards } from '@/api/digimonCardApi';
import * as deckApi from '@/api/deckApi';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import type { DigimonCardData } from '@/types/cards';
import type { DeckEntry, DeckValidationResult } from '@/types/deck';
import './DeckBuilderPage.css';

const COLORS = ['Red', 'Blue', 'Yellow', 'Green', 'Purple', 'Black', 'White'];
const TYPES = ['all', 'Digimon', 'Digi-Egg', 'Tamer', 'Option'];
const LEVELS = ['all', '2', '3', '4', '5', '6', '7'];
const RARITIES = ['all', 'C', 'U', 'R', 'SR', 'SEC', 'P'];

const DEFAULT_FILTERS: BuilderCardFilters = {
  search: '',
  colors: [],
  type: 'all',
  level: 'all',
  rarity: 'all',
  inheritedOnly: false,
  securityOnly: false,
};

function cardButtonName(card: DigimonCardData): string {
  return `${card.name} ${card.cardnumber}${card.isAltArt ? ' alt art' : ''}`;
}

function cardCount(entries: DeckEntry[], cardId: string): number {
  return entries
    .filter((entry) => entry.cardId === cardId)
    .reduce((sum, entry) => sum + entry.count, 0);
}

function uniqueCards(cards: DigimonCardData[]): DigimonCardData[] {
  const seen = new Set<string>();
  return cards.filter((card) => {
    const key = `${card.cardnumber}|${card.isAltArt ? '1' : '0'}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function BuilderCardImage({
  card,
  className = '',
}: {
  card: DigimonCardData;
  className?: string;
}) {
  // No useCardImage here: its eager `new Image()` preload would fetch art
  // for the entire pool (~600 cards) on mount. `loading="lazy"` lets the
  // browser fetch only what scrolls into view.
  const [hasError, setHasError] = useState(false);
  const src = getCardImageUrl(card.cardnumber, card.isAltArt ?? false);
  return (
    <div className={`bld-card-image ${className}`}>
      {!hasError ? (
        <img
          src={src}
          alt={card.name}
          draggable={false}
          loading="lazy"
          onError={() => setHasError(true)}
        />
      ) : (
        <span>{card.name}</span>
      )}
    </div>
  );
}

async function loadCardMap(cardIds: string[]): Promise<Map<string, DigimonCardData>> {
  const pairs = await Promise.allSettled(
    [...new Set(cardIds)].map(async (cardId) => [cardId, await getCardById(cardId)] as const),
  );
  const cardMap = new Map<string, DigimonCardData>();
  for (const result of pairs) {
    if (result.status === 'fulfilled' && result.value[1]) {
      cardMap.set(result.value[0], result.value[1]);
    }
  }
  return cardMap;
}

function ValidationPanelInline({
  validationResult,
}: {
  validationResult: DeckValidationResult | null;
}) {
  if (!validationResult) return null;
  if (validationResult.errors.length === 0 && validationResult.warnings.length === 0) {
    return <div className="bld-validation good">Deck is valid</div>;
  }
  return (
    <div className="bld-validation bad">
      {validationResult.errors.map((error, index) => (
        <p key={`e-${index}`}>ERROR: {error.message}</p>
      ))}
      {validationResult.warnings.map((warning, index) => (
        <p key={`w-${index}`}>WARNING: {warning.message}</p>
      ))}
    </div>
  );
}

export function DeckBuilderPage() {
  const { id: routeDeckId } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const params = new URLSearchParams(location.search);
  const returnToPlay = params.get('returnTo') === 'play';
  const {
    deckName,
    setDeckName,
    deckId,
    setDeckId,
    loadDeck,
    clearDeck,
    mainDeck,
    eggDeck,
    isDirty,
    setIsDirty,
    validationResult,
    setValidationResult,
    testedCardIds,
    setTestedCardIds,
    addCardToDeck,
    removeCardFromDeck,
  } = useDeckBuilderStore();

  const [showImport, setShowImport] = useState(false);
  const [saving, setSaving] = useState(false);
  const [activeSection, setActiveSection] = useState<'main' | 'egg' | 'side'>('main');
  const [builderFilters, setBuilderFilters] = useState<BuilderCardFilters>(DEFAULT_FILTERS);
  const [cardPool, setCardPool] = useState<DigimonCardData[]>([]);
  const [previewCard, setPreviewCard] = useState<DigimonCardData | null>(null);
  const [notice, setNotice] = useState('');

  useEffect(() => {
    const nextParams = new URLSearchParams(location.search);
    if (location.pathname.endsWith('/new') || nextParams.get('new') === '1') {
      clearDeck();
      setPreviewCard(null);
      setShowImport(nextParams.get('import') === '1');
    } else if (nextParams.get('import') === '1') {
      setShowImport(true);
    }
  }, [location.pathname, location.search, clearDeck]);

  useEffect(() => {
    if (testedCardIds !== null) return;
    deckApi
      .listTestedCards()
      .then(setTestedCardIds)
      .catch(() => {
        // Desktop-mode browser tests do not have a Tauri bridge. Leave the
        // gate open rather than turning the builder into a read-only shell.
        setNotice('Tested-card gate unavailable');
      });
  }, [testedCardIds, setTestedCardIds]);

  useEffect(() => {
    if (!routeDeckId) return;
    let cancelled = false;

    async function loadRouteDeck() {
      try {
        const deck = await getBuilderDeck(routeDeckId!);
        const cardMap = await loadCardMap([...deck.main_deck, ...deck.egg_deck]);
        const mainEntries = slotArraysToDeckEntries(
          deck.main_deck,
          deck.main_deck_alt_arts,
          cardMap,
        );
        const eggEntries = slotArraysToDeckEntries(deck.egg_deck, deck.egg_deck_alt_arts, cardMap);
        if (!cancelled) {
          loadDeck(deck.id, deck.name, mainEntries, eggEntries);
          const loadedCards = Array.from(cardMap.values());
          setCardPool((current) => uniqueCards([...loadedCards, ...current]));
          setPreviewCard(loadedCards[0] ?? null);
        }
      } catch {
        if (!cancelled) {
          clearDeck();
          setNotice('Unable to load deck');
        }
      }
    }

    void loadRouteDeck();
    return () => {
      cancelled = true;
    };
  }, [routeDeckId, loadDeck, clearDeck]);

  // Seed the browse pool with the FULL implemented card pool from local
  // data (embedded cards.json on desktop, hosted API copy in browser).
  // This is what guarantees every implemented card is browsable — the
  // remote-search effect below only enriches entries (alt arts,
  // set names) and must never be the only source.
  useEffect(() => {
    let cancelled = false;
    deckApi
      .listCardDatabase()
      .then((cards) => {
        if (cancelled) return;
        // Append after current so richer remote-search entries win dedupe.
        setCardPool((current) => uniqueCards([...current, ...cards]));
        setPreviewCard((current) => current ?? cards[0] ?? null);
      })
      .catch(() => {
        if (!cancelled) setNotice('Local card database unavailable');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const search = builderFilters.search.trim();
    const request = search
      ? searchCards({ n: search, sort: 'name' })
      : searchCards({ sort: 'name', series: 'Digimon Card Game' });

    request
      .then((results) => {
        if (cancelled) return;
        const allowed = testedCardIds && testedCardIds.size > 0
          ? results.filter((card) => testedCardIds.has(card.cardnumber))
          : results;
        setCardPool((current) => uniqueCards([...allowed, ...current]));
        setPreviewCard((current) => current ?? allowed[0] ?? null);
      })
      .catch(() => {
        // Remote enrichment (alt arts, set names) is best-effort; the
        // local database seed above keeps the builder fully usable.
      });

    return () => {
      cancelled = true;
    };
  }, [builderFilters.search, testedCardIds]);

  const counts = useMemo(() => getBuilderCounts(mainDeck, eggDeck), [mainDeck, eggDeck]);
  const visibleCards = useMemo(
    () => filterBuilderCards(cardPool, builderFilters),
    [builderFilters, cardPool],
  );
  const activeCards = activeSection === 'egg' ? eggDeck : mainDeck;
  const visibleSections = useMemo(() => groupDeckEntriesForBuilder(activeCards), [activeCards]);
  const activeExpected = activeSection === 'egg' ? 5 : 50;

  const handleSave = async () => {
    setSaving(true);
    setNotice('');
    try {
      const mainSlots = deckEntriesToSlotArrays(mainDeck);
      const eggSlots = deckEntriesToSlotArrays(eggDeck);
      const saved = await saveBuilderDeck({
        deckId,
        name: deckName,
        main_deck: mainSlots.ids,
        egg_deck: eggSlots.ids,
        main_deck_alt_arts: mainSlots.altArts,
        egg_deck_alt_arts: eggSlots.altArts,
        game_mode: 'standard',
      });
      setDeckId(saved.id);
      setIsDirty(false);
      setNotice('Deck saved');
      if (returnToPlay) {
        navigate('/play/deck', { replace: true });
      } else if (!deckId) {
        navigate(`/deckbuilder/${saved.id}`, { replace: true });
      }
    } catch {
      setNotice('Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleValidate = async () => {
    const mainSlots = deckEntriesToSlotArrays(mainDeck);
    const eggSlots = deckEntriesToSlotArrays(eggDeck);
    try {
      const result = await deckApi.validateDeckRaw(mainSlots.ids, eggSlots.ids);
      setValidationResult(result);
      setNotice(result.valid ? 'Deck valid' : 'Deck has issues');
    } catch {
      setNotice('Validation unavailable');
    }
  };

  const handleClear = () => {
    clearDeck();
    setPreviewCard(null);
    setNotice('Deck cleared');
  };

  return (
    <div className="deck-builder-page">
      <div className="deck-builder-app">
        <div className="bld">
          <header className="bld-top">
            <div className="left">
              <button type="button" className="back" onClick={() => navigate('/')}>
                HOME
              </button>
              <button type="button" className="back" onClick={() => navigate('/deckbuilder')}>
                LIBRARY
              </button>
              {returnToPlay && (
                <button type="button" className="back" onClick={() => navigate('/play/deck')}>
                  BACK TO PLAY
                </button>
              )}
              <input
                className="deck-name-input"
                aria-label="Deck name"
                value={deckName}
                onChange={(event) => setDeckName(event.target.value)}
              />
              <span className="pill"><span className="v">{counts.main}</span>/50</span>
              <span className="pill">EGG <span className="v">{counts.egg}</span>/5</span>
              <span className="pill disabled">SIDE <span className="v">0</span>/15</span>
              {notice && <span className="bld-notice">{notice}</span>}
            </div>

            <div className="bld-counts">
              <div className={`bld-count ${counts.egg >= 4 ? 'ok' : 'warn'}`}>
                <span className="v">{counts.egg}</span>EGG
              </div>
              <div className="bld-count">
                <span className="v player">{counts.digimon}</span>DIGIMON
              </div>
              <div className="bld-count">
                <span className="v">{counts.tamer}</span>TAMER
              </div>
              <div className="bld-count">
                <span className="v">{counts.option}</span>OPTION
              </div>
              <div className="bld-count split"><span className="v">{counts.lv2}</span>L2</div>
              <div className="bld-count"><span className="v">{counts.lv3}</span>L3</div>
              <div className="bld-count"><span className="v">{counts.lv4}</span>L4</div>
              <div className="bld-count"><span className="v">{counts.lv5}</span>L5</div>
              <div className="bld-count"><span className="v">{counts.lv6}</span>L6</div>
              <div className="bld-count"><span className="v">{counts.lv7}</span>L7+</div>
            </div>

            <div className="right">
              <button type="button" className="btn btn-good" onClick={handleSave} disabled={saving || !isDirty}>
                {saving ? 'SAVING...' : 'SAVE'}
              </button>
              <button type="button" className="btn btn-opp" onClick={handleValidate}>VALIDATE</button>
              <button type="button" className="btn btn-ghost" onClick={() => setShowImport(true)}>IMPORT</button>
              <button type="button" className="btn btn-danger" onClick={handleClear}>CLEAR</button>
              <button type="button" className="btn btn-ghost" onClick={() => navigate('/')}>QUIT</button>
            </div>
          </header>

          <section className="bld-filters" aria-label="Builder filters">
            <div className="bld-filter color-filter">
              <span className="l">COLOR</span>
              <div className="bld-colors">
                <button
                  type="button"
                  className={`chip all ${builderFilters.colors.length === 0 ? 'on' : ''}`}
                  onClick={() => setBuilderFilters((current) => ({ ...current, colors: [] }))}
                >
                  ALL
                </button>
                {COLORS.map((color) => (
                  <button
                    type="button"
                    key={color}
                    className={`chip ${builderFilters.colors.includes(color) ? 'on' : ''} ${color.toLowerCase()}`}
                    onClick={() => setBuilderFilters((current) => ({
                      ...current,
                      colors: current.colors.includes(color)
                        ? current.colors.filter((item) => item !== color)
                        : [...current.colors, color],
                    }))}
                  >
                    {color[0]}
                  </button>
                ))}
              </div>
            </div>
            <label className="bld-filter">
              <span className="l">TYPE</span>
              <select value={builderFilters.type} onChange={(event) => setBuilderFilters((current) => ({ ...current, type: event.target.value }))}>
                {TYPES.map((type) => <option key={type} value={type}>{type.toUpperCase()}</option>)}
              </select>
            </label>
            <label className="bld-filter">
              <span className="l">LEVEL</span>
              <select value={builderFilters.level} onChange={(event) => setBuilderFilters((current) => ({ ...current, level: event.target.value }))}>
                {LEVELS.map((level) => <option key={level} value={level}>{level === 'all' ? 'ALL' : `LV${level}`}</option>)}
              </select>
            </label>
            <label className="bld-filter">
              <span className="l">RARITY</span>
              <select value={builderFilters.rarity} onChange={(event) => setBuilderFilters((current) => ({ ...current, rarity: event.target.value }))}>
                {RARITIES.map((rarity) => <option key={rarity} value={rarity}>{rarity.toUpperCase()}</option>)}
              </select>
            </label>
            <label className="bld-filter search">
              <span className="l">SEARCH</span>
              <input
                placeholder="NAME, ID, KEYWORD..."
                value={builderFilters.search}
                onChange={(event) => setBuilderFilters((current) => ({ ...current, search: event.target.value }))}
              />
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={builderFilters.inheritedOnly}
                onChange={(event) => setBuilderFilters((current) => ({ ...current, inheritedOnly: event.target.checked }))}
              />
              INHERITED ONLY
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={builderFilters.securityOnly}
                onChange={(event) => setBuilderFilters((current) => ({ ...current, securityOnly: event.target.checked }))}
              />
              SECURITY ONLY
            </label>
          </section>

          <main className="bld-main">
            <aside className="bld-preview">
              {previewCard ? (
                <>
                  <div className="bld-preview-card">
                    <div className={`frame ${builderCardColorClass(previewCard)}`}>
                      <BuilderCardImage card={previewCard} />
                      {previewCard.play_cost && <span className="cost">{previewCard.play_cost}</span>}
                      {previewCard.level && <span className="lvl">L{previewCard.level}</span>}
                      <span className="nm">{previewCard.name}</span>
                      <span className="id">{previewCard.cardnumber}</span>
                    </div>
                  </div>
                  <div className="bld-preview-meta">
                    <div className="row"><span className="k">SET</span><span className="v">{previewCard.set_name || '-'}</span></div>
                    <div className="row"><span className="k">RARITY</span><span className="v">{previewCard.cardrarity || '-'}</span></div>
                    <div className="row"><span className="k">TYPE</span><span className="v">{previewCard.type}</span></div>
                    <div className="row"><span className="k">IN DECK</span><span className="v">x{cardCount([...mainDeck, ...eggDeck], previewCard.cardnumber)}</span></div>
                  </div>
                  <div className="bld-preview-effect"><h6>MAIN EFFECT</h6><p>{previewCard.maineffect || 'No main effect text loaded.'}</p></div>
                  {previewCard.soureeffect && <div className="bld-preview-effect"><h6 className="opp">INHERITED EFFECT</h6><p>{previewCard.soureeffect}</p></div>}
                </>
              ) : (
                <div className="bld-empty">SEARCH OR SELECT A CARD</div>
              )}
            </aside>

            <section className="bld-pool">
              <div className="bld-pool-head">
                <span>CARD POOL · <span className="v">{visibleCards.length}</span> RESULTS</span>
                <div className="legend"><span><i className="in"></i>IN DECK</span><span><i className="hover"></i>HOVER</span></div>
              </div>
              <div className="bld-pool-grid">
                {visibleCards.map((card) => {
                  const count = cardCount([...mainDeck, ...eggDeck], card.cardnumber);
                  const atCap = count >= 4;
                  return (
                    <button
                      type="button"
                      key={`${card.cardnumber}-${card.isAltArt ? 'alt' : 'base'}`}
                      aria-label={cardButtonName(card)}
                      aria-disabled={atCap}
                      className={`bld-card ${builderCardColorClass(card)} ${count > 0 ? 'in-deck' : ''} ${atCap ? 'cap-reached' : ''} ${previewCard?.cardnumber === card.cardnumber ? 'preview' : ''}`}
                      onMouseEnter={() => setPreviewCard(card)}
                      onFocus={() => setPreviewCard(card)}
                      onClick={() => {
                        if (!atCap) addCardToDeck(card.cardnumber, card, card.isAltArt ?? false);
                      }}
                      onContextMenu={(event) => {
                        event.preventDefault();
                        removeCardFromDeck(card.cardnumber, card.isAltArt ?? false);
                      }}
                    >
                      <BuilderCardImage card={card} />
                      {card.play_cost && <span className="cost">{card.play_cost}</span>}
                      {card.level && <span className="lvl">L{card.level}</span>}
                      <span className="nm">{card.name}</span>
                      <span className="id">{card.cardnumber}</span>
                      {count > 0 && <span className="ct">{atCap ? 'MAX' : `x${count}`}</span>}
                    </button>
                  );
                })}
                {visibleCards.length === 0 && <div className="bld-empty pool-empty">NO CARDS MATCH FILTERS</div>}
              </div>
              <div className="bld-pool-foot"><span>CLICK = ADD · RIGHT-CLICK = REMOVE</span></div>
            </section>

            <aside className="bld-deck">
              <div className="bld-deck-head"><span>DECK CONTENTS</span><span className="v">{counts.total}/55</span></div>
              <div className="bld-deck-tabs">
                <button type="button" className={activeSection === 'main' ? 'on' : ''} onClick={() => setActiveSection('main')}>MAIN <span className="ct">{counts.main}</span></button>
                <button type="button" className={activeSection === 'egg' ? 'on' : ''} onClick={() => setActiveSection('egg')}>EGG <span className="ct">{counts.egg}</span></button>
                <button type="button" className="disabled" disabled title="Sideboard is not supported in standard decks">SIDE <span className="ct">0</span></button>
              </div>
              <div className="bld-deck-list">
                {activeSection === 'side' ? (
                  <div className="bld-empty">SIDEBOARD NOT SUPPORTED IN STANDARD</div>
                ) : (
                  <section className="bld-section">
                    <div className="bld-section-head"><span>{activeSection === 'egg' ? 'EGG DECK' : 'MAIN DECK'}</span><span className="ct">{activeCards.reduce((sum, entry) => sum + entry.count, 0)} / {activeExpected}</span></div>
                    {visibleSections.map((section) => (
                      <div key={section.label} className="bld-subsection">
                        <div className="bld-subsection-head">{section.label} <span>{section.total}</span></div>
                        {section.entries.map((entry) => {
                          const totalCount = cardCount([...mainDeck, ...eggDeck], entry.cardId);
                          return (
                            <div key={`${entry.cardId}-${entry.isAltArt ? 'alt' : 'base'}`} className="bld-row" onMouseEnter={() => entry.cardData && setPreviewCard(entry.cardData)}>
                              <span className="ct">x{entry.count}</span>
                              <span className={`swatch ${builderCardColorClass(entry.cardData)}`} />
                              <div className="nm">{entry.cardData?.name ?? entry.cardId}<small>{entry.cardId} · {entry.cardData?.type?.toUpperCase() ?? 'CARD'}</small></div>
                              <span className="cost">{entry.cardData?.play_cost ? `C${entry.cardData.play_cost}` : '-'}</span>
                              <span className="lvl">{entry.cardData?.level ? `L${entry.cardData.level}` : entry.cardData?.type === 'Option' ? 'OPT' : 'TMR'}</span>
                              <div className="actions">
                                <button type="button" onClick={() => removeCardFromDeck(entry.cardId, entry.isAltArt)}>-</button>
                                <button type="button" disabled={totalCount >= 4} onClick={() => entry.cardData && addCardToDeck(entry.cardId, entry.cardData, entry.isAltArt)}>+</button>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    ))}
                    {activeCards.length === 0 && <div className="bld-empty">EMPTY</div>}
                  </section>
                )}
                <ValidationPanelInline validationResult={validationResult} />
              </div>
            </aside>
          </main>
        </div>
        <ImportExport isOpen={showImport} onClose={() => setShowImport(false)} />
      </div>
    </div>
  );
}
