import { useEffect, useRef, useState } from 'react';
import { Card } from '@/components/shared/Card';
import type { PermanentInfo, SourceInfo } from '@/types/game';
import type { BoardStyle } from '@/stores/uiStore';
import { useGameStore } from '@/stores/gameStore';
import { COLOR_HEX, COLOR_NAMES, KEYWORD_DISPLAY } from '@/utils/constants';
import { groupModifiers } from '@/utils/modifierDisplay';
import { CardEffectText } from '@/utils/formatCardText';

/** Something the player can do with the inspected permanent right now. */
export interface InspectorAction {
  key: string;
  label: string;
  /** Hover text, e.g. the full authored effect summary. */
  title?: string;
  tone: 'attack' | 'effect' | 'move';
  onClick: () => void;
}

interface CardOverlayProps {
  permanent: PermanentInfo | null;
  onClose: () => void;
  /**
   * Right-click (context-menu) or click a card image to open it in the
   * enlarged CardDetailOverlay — without closing this panel (DCGO's
   * CardInfo → OpenCardDetail flow).
   */
  onInspectCard?: (cardId: string) => void;
  /** Whose permanent this is. The panel covers the OTHER half of the board
   *  so the inspected card stays in sight. */
  side?: 'own' | 'opponent';
  /** Legal actions for this permanent (attack, [Main] effects, move). */
  actions?: InspectorAction[];
  boardStyle?: BoardStyle;
}

type KeywordTone = 'defense' | 'offense' | 'recovery' | 'effect';

const KEYWORD_TONES: Record<string, KeywordTone> = {
  Blocker: 'defense', Barrier: 'defense', Evade: 'defense', ArmorPurge: 'defense',
  Jamming: 'defense', Decoy: 'defense', Fragment: 'defense', Scapegoat: 'defense',
  Iceclad: 'defense', Progress: 'defense',
  Reboot: 'recovery', Recovery: 'recovery', Fortitude: 'recovery', Save: 'recovery',
  MaterialSave: 'recovery', Partition: 'recovery', Decode: 'recovery',
  Rush: 'offense', Blitz: 'offense', Raid: 'offense', Piercing: 'offense',
  SecurityAttack: 'offense', Retaliation: 'offense', Alliance: 'offense',
  Collision: 'offense', Vortex: 'offense', Overclock: 'offense', Execute: 'offense',
  DeDigivolve: 'offense',
};

/**
 * Stack inspector — the permanent as it stands right now, laid over the half
 * of the board opposite the card (Aegis-style) so the card stays visible:
 * card art | reading column (DP with its change from base, status badges,
 * printed effect, DP breakdown, actions) | the digivolution cards and linked
 * cards with their inherited / link effects.
 *
 * With no permanent but a hovered card, shows a plain hover preview.
 */
export function CardOverlay({
  permanent,
  onClose,
  onInspectCard,
  side = 'own',
  actions = [],
  boardStyle = 'classic',
}: CardOverlayProps) {
  const hoveredCard = useGameStore((s) => s.hoveredCard);
  const panelRef = useRef<HTMLElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const [showBreakdown, setShowBreakdown] = useState(false);

  // Dismiss on Escape, or on a press outside the panel. The enlarged card
  // detail sits above this panel and owns its own presses.
  useEffect(() => {
    if (!permanent) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const handlePointer = (e: PointerEvent) => {
      const target = e.target;
      if (!(target instanceof Node) || panelRef.current?.contains(target)) return;
      if (target instanceof Element && target.closest('[data-testid="card-detail-overlay"]')) return;
      onClose();
    };
    window.addEventListener('keydown', handleKey);
    document.addEventListener('pointerdown', handlePointer, true);
    return () => {
      window.removeEventListener('keydown', handleKey);
      document.removeEventListener('pointerdown', handlePointer, true);
    };
  }, [permanent, onClose]);

  const topCardId = permanent?.topCardId ?? null;
  useEffect(() => {
    if (topCardId) closeRef.current?.focus({ preventScroll: true });
  }, [topCardId]);

  // Hover-only preview (no permanent inspected)
  if (!permanent && hoveredCard) {
    return (
      <div className="absolute bottom-4 left-4 z-30 pointer-events-none">
        <Card cardId={hoveredCard} size="inspector" />
      </div>
    );
  }

  if (!permanent) return null;

  const breakdown = permanent.dpBreakdown;
  const baseDp = breakdown.base;
  const dpDelta = baseDp != null ? permanent.dp - baseDp : 0;
  const nonZeroSourceDp = breakdown.sources.filter((s) => s.value !== 0);
  const hasDpBreakdown = breakdown.total != null &&
    (nonZeroSourceDp.length > 0 || breakdown.temporary !== 0);

  // Sources are stored bottom-up; list them top-down under the top card.
  const stackSources = [...permanent.sources].reverse().filter((src) => !src.isTop);
  // Linked cards, each with its printed link effect — the text this Digimon
  // gains from it (rules 4-2-6).
  const linkedIds = permanent.linkedCardIds;
  const linkEffects = permanent.linkEffects ?? [];
  const modifierGroups = groupModifiers(permanent.modifiers ?? []);
  const hasSupporting = stackSources.length > 0 || linkedIds.length > 0;
  const gained = new Set(permanent.keywordBreakdown.gained);
  // Tamers / Options in play report level 0 and a -1 Security Attack.
  const isDigimon = permanent.level != null && permanent.level > 0;

  const inspect = (cardId: string) => () => onInspectCard?.(cardId);
  const inspectOnRightClick = (cardId: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    onInspectCard?.(cardId);
  };

  return (
    <section
      ref={panelRef}
      className="ib-inspector animate-overlay-open"
      data-half={side === 'own' ? 'upper' : 'lower'}
      data-board-style={boardStyle}
      data-has-sources={hasSupporting || undefined}
      role="dialog"
      aria-modal="false"
      aria-label={`${permanent.topCardName ?? 'Permanent'} details`}
    >
      <div className="ib-inspector__body">
        {/* ── Card art ── */}
        {permanent.topCardId && (
          <div className="ib-inspector__art">
            <Card
              cardId={permanent.topCardId}
              cardName={permanent.topCardName ?? undefined}
              cardColor={permanent.colors[0]}
              size="inspector"
              suspended={false}
              onClick={inspect(permanent.topCardId)}
              onContextMenu={inspectOnRightClick(permanent.topCardId)}
            />
            <span className="ib-inspector__id">{permanent.topCardId}</span>
          </div>
        )}

        {/* ── Reading column ── */}
        <div className="ib-inspector__reading">
          <header className="ib-inspector__header">
            <div className="ib-inspector__title">
              <h2>{permanent.topCardName ?? 'Unknown'}</h2>
              {isDigimon && <span className="ib-inspector__level">Lv.{permanent.level}</span>}
              <span className="ib-inspector__colors" aria-label="Colors">
                {permanent.colors.map((c) => (
                  <i key={c} style={{ background: colorHex(c) }} title={COLOR_NAMES[c]} />
                ))}
              </span>
            </div>
            {permanent.dp > 0 && (
              <strong className="ib-inspector__dp" data-testid="inspector-dp">
                {permanent.dp.toLocaleString()} DP
                {dpDelta !== 0 && (
                  <em data-direction={dpDelta > 0 ? 'up' : 'down'}>
                    {dpDelta > 0 ? '+' : '−'}{Math.abs(dpDelta).toLocaleString()}
                  </em>
                )}
              </strong>
            )}
            <button
              ref={closeRef}
              type="button"
              data-testid="card-overlay-close"
              className="ib-inspector__close"
              onClick={onClose}
              aria-label="Close"
            >
              &times;
            </button>
          </header>

          {/* Status: suspended, keywords (granted marked), security attack */}
          <div className="ib-inspector__state">
            {permanent.isSuspended && <span className="ib-inspector__badge" data-tone="muted">Suspended</span>}
            {isDigimon && permanent.securityAttackModifier !== 0 && (
              <span className="ib-inspector__badge" data-tone="offense">
                <ToneIcon tone="offense" />
                Security Attack {permanent.securityAttackModifier > 0 ? '+' : '−'}
                {Math.abs(permanent.securityAttackModifier)}
              </span>
            )}
            {permanent.keywords.map((kw) => {
              const tone = KEYWORD_TONES[kw] ?? 'effect';
              return (
                <span
                  key={kw}
                  className="ib-inspector__badge"
                  data-tone={tone}
                  data-granted={gained.has(kw) || undefined}
                  title={gained.has(kw) ? 'Granted by an effect' : 'Printed'}
                >
                  <ToneIcon tone={tone} />
                  {KEYWORD_DISPLAY[kw] ?? kw}
                  {gained.has(kw) && <small>granted</small>}
                </span>
              );
            })}
          </div>

          <div className="ib-inspector__scroll">
            {/* Active modifiers (immunities, restrictions, stat changes) */}
            {modifierGroups.length > 0 && (
              <div data-testid="active-modifiers" className="ib-inspector__section">
                <span className="ib-inspector__label">Active modifiers</span>
                <ul className="ib-inspector__modifiers">
                  {modifierGroups.flatMap((grp) =>
                    grp.items.map((item, i) => (
                      <li key={`${grp.group}-${i}`} style={{ '--mod-color': grp.color } as React.CSSProperties}>
                        <span>{item.label}</span>
                        {item.expiryHint && <small>({item.expiryHint})</small>}
                      </li>
                    )),
                  )}
                </ul>
              </div>
            )}

            <div className="ib-inspector__section">
              <span className="ib-inspector__label">Printed effect</span>
              {permanent.mainEffectText ? (
                <CardEffectText text={permanent.mainEffectText} />
              ) : (
                <p className="ib-inspector__none">No printed effect.</p>
              )}
            </div>

            {hasDpBreakdown && (
              <div className="ib-inspector__section">
                <button
                  type="button"
                  className="ib-inspector__label ib-inspector__toggle"
                  aria-expanded={showBreakdown}
                  onClick={() => setShowBreakdown((v) => !v)}
                >
                  DP breakdown {showBreakdown ? '▾' : '▸'}
                </button>
                {showBreakdown && (
                  <div className="ib-inspector__breakdown" data-testid="dp-breakdown">
                    <span>Base <b>{(breakdown.base ?? 0).toLocaleString()}</b></span>
                    {nonZeroSourceDp.map((src, i) => (
                      <span key={`${src.cardId}-${i}`}>
                        {src.cardName ?? src.cardId}{' '}
                        <b data-direction={src.value > 0 ? 'up' : 'down'}>
                          {src.value > 0 ? '+' : ''}{src.value.toLocaleString()}
                        </b>
                      </span>
                    ))}
                    {breakdown.temporary !== 0 && (
                      <span>
                        Effects{' '}
                        <b data-direction={breakdown.temporary > 0 ? 'up' : 'down'}>
                          {breakdown.temporary > 0 ? '+' : ''}{breakdown.temporary.toLocaleString()}
                        </b>
                      </span>
                    )}
                    <span>= <b>{breakdown.total?.toLocaleString()}</b></span>
                  </div>
                )}
              </div>
            )}
          </div>

          {actions.length > 0 && (
            <div className="ib-inspector__actions">
              {actions.map((a) => (
                <button
                  key={a.key}
                  type="button"
                  data-testid={`inspector-action-${a.key}`}
                  className="ib-inspector__action"
                  data-tone={a.tone}
                  title={a.title}
                  onClick={a.onClick}
                >
                  {a.label}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* ── Digivolution cards + linked cards ── */}
        {hasSupporting && (
          <div className="ib-inspector__sources" aria-label="Stack">
            <span className="ib-inspector__label">
              {stackSources.length} digivolution card{stackSources.length === 1 ? '' : 's'}
              {linkedIds.length > 0 && ` · ${linkedIds.length} linked`}
            </span>
            {stackSources.map((src, i) => (
              <StackCardEntry key={`${src.cardId}-${i}`} source={src} onInspectCard={onInspectCard} />
            ))}
            {linkedIds.map((cardId, i) => {
              const linkEffect = linkEffects.find((e) => e.linkIndex === i);
              return (
                <div
                  key={`link-${cardId}-${i}`}
                  data-testid="linked-card"
                  className="ib-inspector__source"
                  style={{ '--source-color': '#a855f7' } as React.CSSProperties}
                >
                  <Card cardId={cardId} size="sm" onClick={inspect(cardId)} onContextMenu={inspectOnRightClick(cardId)} />
                  <div>
                    <span className="ib-inspector__label">
                      {linkEffect?.cardName ?? cardId} · Linked
                    </span>
                    {linkEffect && (
                      <>
                        <span className="ib-inspector__sublabel">Link Effect</span>
                        <CardEffectText text={linkEffect.text} />
                      </>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}

/** One digivolution card: thumbnail, name, and its inherited effect. */
function StackCardEntry({
  source,
  onInspectCard,
}: {
  source: SourceInfo;
  onInspectCard?: (cardId: string) => void;
}) {
  // Hidden / face-down source (opponent's cards)
  if (!source.cardId) {
    return (
      <div className="ib-inspector__source" style={{ '--source-color': '#6b7280' } as React.CSSProperties}>
        <Card cardId="hidden" size="sm" faceDown />
        <div>
          <span className="ib-inspector__label">Hidden card</span>
        </div>
      </div>
    );
  }

  const hasInherited = source.inheritedEffectText.trim().length > 0;
  return (
    <div
      className="ib-inspector__source"
      style={{ '--source-color': colorHex(source.colors?.[0]) } as React.CSSProperties}
    >
      <Card
        cardId={source.cardId}
        size="sm"
        onClick={() => onInspectCard?.(source.cardId)}
        onContextMenu={(e) => {
          e.preventDefault();
          onInspectCard?.(source.cardId);
        }}
      />
      <div>
        <span className="ib-inspector__label">{source.cardName ?? source.cardId} · Inherited</span>
        {hasInherited ? (
          <CardEffectText text={source.inheritedEffectText} />
        ) : (
          <p className="ib-inspector__none">No inherited effect.</p>
        )}
      </div>
    </div>
  );
}

function ToneIcon({ tone }: { tone: KeywordTone }) {
  const paths: Record<KeywordTone, string> = {
    defense: 'M8 1.5 13.5 3.5V8c0 3.2-2.4 5.6-5.5 6.5C4.9 13.6 2.5 11.2 2.5 8V3.5Z',
    offense: 'M2.5 13.5 9 7M9 7l1.5-4.5L14 1.5l-1 3.5L8.5 6.5M2 11l3 3M13.5 13.5 7 7M7 7 5.5 2.5 2 1.5l1 3.5 4.5 1.5M14 11l-3 3',
    recovery: 'M13 8a5 5 0 1 1-1.5-3.6M13 2.5v3h-3',
    effect: 'M8 1.5 9.5 6.5 14.5 8 9.5 9.5 8 14.5 6.5 9.5 1.5 8 6.5 6.5Z',
  };
  return (
    <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d={paths[tone]} />
    </svg>
  );
}

/** CSS color for a card color enum value (gray fallback). */
function colorHex(colorVal: number | undefined): string {
  if (colorVal === undefined) return '#6b7280';
  const name = COLOR_NAMES[colorVal];
  return (name && COLOR_HEX[name]) || '#6b7280';
}
