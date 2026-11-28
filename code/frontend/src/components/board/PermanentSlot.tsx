import { useDraggable } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { KeywordBadges } from './KeywordBadges';
import { useGameStore } from '@/stores/gameStore';
import { formatModifier } from '@/utils/modifierDisplay';
import type { PermanentInfo, PermanentModifier } from '@/types/game';

interface PermanentSlotProps {
  perm: PermanentInfo;
  slotIndex: number;
  isOpponent: boolean;
  highlighted?: boolean;
  /** Already picked during an in-progress "choose N" selection. */
  selected?: boolean;
  targeted?: boolean;
  /** True when this own-field Digimon can legally declare an attack (enables drag-to-attack). */
  canAttack?: boolean;
  onClick?: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  /** Right-click (context-menu) to open the stack inspector. */
  onInspect?: () => void;
}

type ModIconKind =
  | 'unsuspendOff'
  | 'attackOff'
  | 'blockOff'
  | 'suspendOff'
  | 'digivolveOff'
  | 'shieldCheck'
  | 'unblockable'
  | 'sword'
  | 'sa';

interface PillBadge {
  key: string;
  label: string;
  tone: 'restrict' | 'buff' | 'info';
  icon: ModIconKind;
  title: string;
}

/**
 * Resolve the live effective DP for a permanent. `dpBreakdown.total` always
 * includes live temporary/aura DP modifiers even when the legacy `dp` field
 * carries base DP.
 */
export function resolveEffectiveDp(perm: PermanentInfo): number {
  return perm.dpBreakdown?.total ?? perm.dp ?? 0;
}

/**
 * True for Tamers and Delay/Field Options (no printed level and no printed DP).
 * Tokens have DP > 0 (even when levelless), so they are NOT classified here.
 */
export function isNonDigimonPermanent(perm: PermanentInfo): boolean {
  const effectiveDp = resolveEffectiveDp(perm);
  const baseDp = perm.dpBreakdown?.base ?? perm.dp ?? 0;
  const hasLevel = perm.level != null && perm.level > 0;
  return !hasLevel && effectiveDp === 0 && baseDp === 0;
}

const PROTECTION_MOD_TYPES = new Set([
  'CannotBeDestroyed',
  'CannotBeDestroyedByBattle',
  'CannotBeDestroyedByEffect',
  'CannotBeDestroyedOutsideBattle',
  'CannotBeDeletedByBattle',
  'CannotBeDeletedByEffect',
  'CannotBeRemoved',
  'CannotBeReturnedToDeck',
  'CannotBeReturnedToHand',
  'CannotBeTrashedByEffect',
  'CannotBeDeDigivolved',
  'CannotBeSelectedByEffect',
  'CannotBeAffected',
  'ImmunityToOpponentEffects',
  'ImmuneFromOpponentEffects',
  'ImmuneFromDPMinus',
]);

function ModStatusIcon({ kind }: { kind: ModIconKind }) {
  const common = {
    width: 10,
    height: 10,
    viewBox: '0 0 16 16',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.85,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true as const,
  };

  switch (kind) {
    case 'shieldCheck':
      return (
        <svg {...common}>
          <path d="M8 1.5L13.5 3.8v4.1c0 3.3-2.3 5.7-5.5 6.6-3.2-.9-5.5-3.3-5.5-6.6V3.8L8 1.5z" />
          <path d="M5.6 8l1.7 1.7 3.2-3.4" />
        </svg>
      );
    case 'unblockable':
      return (
        <svg {...common}>
          <path d="M2 8h10M9 4.5L12.5 8 9 11.5" />
          <path d="M13.5 3v10" strokeDasharray="2 2" />
        </svg>
      );
    case 'unsuspendOff':
      return (
        <svg {...common}>
          <path d="M12.5 6.5A4.8 4.8 0 0 0 4.3 4.8" />
          <path d="M3.5 9.5A4.8 4.8 0 0 0 11.7 11.2" />
          <path d="M2.5 2.5l11 11" />
        </svg>
      );
    case 'attackOff':
      return (
        <svg {...common}>
          <path d="M12.5 3.5l-7 7M2.5 2.5l11 11" />
        </svg>
      );
    case 'blockOff':
      return (
        <svg {...common}>
          <path d="M8 2L13 4.2v3.6c0 2.9-2 5-5 5.8-3-.8-5-2.9-5-5.8V4.2L8 2z" />
          <path d="M3 3l10 10" />
        </svg>
      );
    case 'suspendOff':
    case 'digivolveOff':
      return (
        <svg {...common}>
          <circle cx="8" cy="8" r="5.5" />
          <path d="M4.2 4.2l7.6 7.6" />
        </svg>
      );
    case 'sword':
    case 'sa':
    default:
      return (
        <svg {...common}>
          <path d="M13 3L6 10M10 3h3v3M4.5 8.5l3 3M2.5 13.5l2.5-2.5" />
        </svg>
      );
  }
}

function summarizeModifiers(
  modifiers: PermanentModifier[],
  isNonDigimon: boolean,
): PillBadge[] {
  const byLabel = new Map<string, { badge: PillBadge; details: Set<string> }>();
  let netSaDelta = 0;
  const saExpiries = new Set<string>();

  const addBadge = (
    label: string,
    tone: PillBadge['tone'],
    icon: ModIconKind,
    key: string,
    detail: string,
  ) => {
    const existing = byLabel.get(label);
    if (existing) {
      existing.details.add(detail);
      existing.badge.title = `${label}: ${[...existing.details].join('; ')}`;
      return;
    }
    const details = new Set([detail]);
    byLabel.set(label, {
      details,
      badge: {
        key,
        label,
        tone,
        icon,
        title: `${label}: ${detail}`,
      },
    });
  };

  for (const mod of modifiers) {
    const formatted = formatModifier(mod);
    const detail = formatted.expiryHint
      ? `${formatted.label} (${formatted.expiryHint})`
      : formatted.label;

    if (PROTECTION_MOD_TYPES.has(mod.type)) {
      addBadge('Protected', 'buff', 'shieldCheck', 'protected', detail);
      continue;
    }

    switch (mod.type) {
      case 'CannotUnsuspend':
      case 'CannotUnsuspendInUnsuspendPhase':
        addBadge("Won't Unsuspend", 'restrict', 'unsuspendOff', mod.type, detail);
        break;
      case 'CannotAttack':
      case 'CannotAttackTarget':
        addBadge("Can't Attack", 'restrict', 'attackOff', mod.type, detail);
        break;
      case 'CannotAttackPlayer':
        addBadge("Can't Attack Player", 'restrict', 'attackOff', mod.type, detail);
        break;
      case 'CannotBlock':
        addBadge("Can't Block", 'restrict', 'blockOff', mod.type, detail);
        break;
      case 'CannotDigivolve':
      case 'CannotDigivolveAsTamer':
        addBadge("Can't Digivolve", 'restrict', 'digivolveOff', mod.type, detail);
        break;
      case 'CannotSuspend':
        addBadge("Can't Suspend", 'restrict', 'suspendOff', mod.type, detail);
        break;
      case 'CannotBeBlocked':
      case 'CannotBeBlockedByNoSources':
        addBadge('Unblockable', 'buff', 'unblockable', mod.type, detail);
        break;
      case 'ForceAttack':
        addBadge('Must Attack', 'info', 'sword', mod.type, detail);
        break;
      case 'MayAttack':
        addBadge('May Attack', 'info', 'sword', mod.type, detail);
        break;
      case 'SecurityAttackPlus':
      case 'SecurityAttackChange':
        if (!isNonDigimon && mod.value !== 0) {
          netSaDelta += mod.value;
          if (formatted.expiryHint) saExpiries.add(formatted.expiryHint);
        }
        break;
      default:
        break;
    }
  }

  const out = [...byLabel.values()].map((entry) => entry.badge);

  if (!isNonDigimon && netSaDelta !== 0) {
    const label = `SA ${netSaDelta > 0 ? `+${netSaDelta}` : netSaDelta}`;
    const expirySuffix = saExpiries.size > 0 ? ` (${[...saExpiries].join(', ')})` : '';
    out.push({
      key: `sa-${netSaDelta}`,
      label,
      tone: netSaDelta > 0 ? 'buff' : 'restrict',
      icon: 'sa',
      title: `Security Attack ${netSaDelta > 0 ? `+${netSaDelta}` : netSaDelta}${expirySuffix}`,
    });
  }

  return out;
}

export function PermanentSlot({
  perm,
  slotIndex,
  isOpponent,
  highlighted = false,
  selected = false,
  targeted = false,
  canAttack = false,
  onClick,
  onMouseEnter,
  onMouseLeave,
  onInspect,
}: PermanentSlotProps) {
  const activeEffectSlot = useGameStore((s) => s.activeEffectSlot);
  const activeEffectPlayer = useGameStore((s) => s.activeEffectPlayer);

  const dragEnabled = !isOpponent && canAttack && Boolean(perm.topCardId);
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `attacker-perm-${isOpponent ? 'opp' : 'you'}-${slotIndex}`,
    data: {
      type: 'attacker-perm',
      slotIndex,
      cardId: perm.topCardId ?? undefined,
    },
    disabled: !dragEnabled,
  });

  if (!perm.topCardId) return null;

  const isEffectSource =
    activeEffectSlot === slotIndex &&
    ((isOpponent && activeEffectPlayer === 2) ||
      (!isOpponent && activeEffectPlayer === 1));

  const nonDigimon = isNonDigimonPermanent(perm);
  const effectiveDp = resolveEffectiveDp(perm);
  const baseDp = perm.dpBreakdown?.base ?? perm.dp ?? effectiveDp;
  const dpState: 'normal' | 'buff' | 'debuff' =
    effectiveDp > baseDp ? 'buff' : effectiveDp < baseDp ? 'debuff' : 'normal';
  const modPills = summarizeModifiers(perm.modifiers ?? [], nonDigimon);
  const visibleModPills = modPills.slice(0, 3);
  const hiddenModPills = modPills.slice(3);

  const linkedIds = perm.linkedCardIds ?? [];
  const linkEffects = perm.linkEffects ?? [];
  const hasLinks = linkedIds.length > 0;
  const visibleLinked = linkedIds.slice(0, 3);
  const linkedSummaryTitle = linkedIds
    .map((id, idx) => {
      const fx = linkEffects.find((e) => e.linkIndex === idx);
      const name = fx?.cardName ? `${fx.cardName} (${id})` : id;
      return fx?.text ? `${name}: ${fx.text}` : name;
    })
    .join('\n');

  const slotStateClasses = [
    'ib-permanent-slot',
    isEffectSource ? 'animate-effect-pulse ib-permanent-slot--effect hs-perm--effect-source' : '',
    selected ? 'hs-perm--selected' : '',
    targeted ? 'hs-perm--targeted' : '',
    highlighted ? 'hs-perm--highlighted' : '',
    canAttack ? 'hs-perm--can-attack' : '',
    hasLinks ? 'hs-perm--has-links' : '',
    isDragging ? 'opacity-50' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      ref={setNodeRef}
      {...(dragEnabled ? { ...attributes, ...listeners } : {})}
      className={slotStateClasses}
      data-effect-source={isEffectSource || undefined}
      data-selected={selected || undefined}
      data-targeted={targeted || undefined}
      data-highlighted={highlighted || undefined}
      data-can-attack={canAttack || undefined}
      data-linked-count={hasLinks ? linkedIds.length : undefined}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onContextMenu={
        onInspect
          ? (e) => {
              e.preventDefault();
              e.stopPropagation();
              onInspect();
            }
          : undefined
      }
    >
      <div className="hs-perm-card-frame">
        {/* Tucked miniature Linked Cards peeking beside the host permanent.
            No click stopPropagation so main card attack/target clicks stay 100% open,
            while hover titles and right-click inspect bubble natively. */}
        {hasLinks && (
          <div
            className="hs-perm-links"
            data-testid="perm-linked-cards"
            aria-label={`${linkedIds.length} linked card${linkedIds.length > 1 ? 's' : ''}`}
          >
            {visibleLinked.map((cid, idx) => {
              const fx = linkEffects.find((e) => e.linkIndex === idx);
              const linkLabel = fx?.cardName ? `${fx.cardName} (${cid})` : cid;
              return (
                <div
                  key={`${cid}-${idx}`}
                  data-testid="perm-linked-peek"
                  className="hs-perm-link-peek"
                  style={{
                    top: `${34 + idx * 24}px`,
                    zIndex: visibleLinked.length - idx,
                  }}
                  title={`Linked: ${linkLabel}${fx?.text ? ` — ${fx.text}` : ''} (Right-click to inspect)`}
                >
                  <div className="hs-perm-link-peek__card">
                    <Card
                      cardId={cid}
                      cardName={fx?.cardName ?? undefined}
                      size="sm"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div className="hs-perm-main-card">
          <Card
            cardId={perm.topCardId}
            cardName={perm.topCardName ?? undefined}
            cardColor={perm.colors[0]}
            size="md"
            suspended={perm.isSuspended}
            highlighted={highlighted}
            selected={selected}
            targeted={targeted}
            className="transition-transform duration-300"
            overlay={
              nonDigimon
                ? { keywords: perm.keywords }
                : {
                    dp: effectiveDp,
                    dpState,
                    level: perm.level != null && perm.level > 0 ? perm.level : null,
                    keywords: perm.keywords,
                    saModifier: nonDigimon ? undefined : perm.securityAttackModifier,
                  }
            }
          />
        </div>

        {/* Distinct state indicator pills so effect-source, selected, target, and attackable never blur */}
        {isEffectSource && (
          <div
            data-testid="perm-effect-source-badge"
            className="hs-state-pill hs-state-pill--effect"
            title="Effect activating"
          >
            EFFECT
          </div>
        )}
        {selected && (
          <div
            data-testid="permanent-selected-badge"
            className="absolute top-1 right-1 z-10 w-5 h-5 rounded-full bg-emerald-500 text-white text-xs font-bold flex items-center justify-center shadow ring-1 ring-white/80"
            title="Selected"
          >
            ✓
          </div>
        )}
        {targeted && !selected && (
          <div
            data-testid="perm-targeted-badge"
            className="hs-state-pill hs-state-pill--target"
            title="Targeted"
          >
            TARGET
          </div>
        )}
        {canAttack && !selected && !targeted && !isEffectSource && (
          <div
            data-testid="perm-can-attack-badge"
            className="hs-state-pill hs-state-pill--attack"
            title="Ready to attack (click or drag to target)"
          >
            ATK
          </div>
        )}

        {hasLinks && (
          <div
            data-testid="perm-link-badge"
            className="hs-link-badge"
            title={`Linked Cards (${linkedIds.length}) — Right-click to inspect:\n${linkedSummaryTitle}`}
            aria-label={`Linked ${linkedIds.length}`}
          >
            <span aria-hidden="true">🔗</span>
            <span>{linkedIds.length}</span>
          </div>
        )}

        {perm.sourceCount > 0 && (
          <div
            className="ib-source-count rounded-full"
            title={`${perm.sourceCount} digivolution source card${perm.sourceCount === 1 ? '' : 's'}`}
          >
            x{perm.sourceCount}
          </div>
        )}
      </div>

      <KeywordBadges
        keywords={perm.keywords}
        gainedKeywords={perm.keywordBreakdown?.gained}
        innateKeywords={perm.keywordBreakdown?.innate}
        variant="slot"
      />

      {modPills.length > 0 && (
        <div className="hs-perm-modifiers" data-testid="perm-modifiers">
          {visibleModPills.map((pill) => (
            <span
              key={pill.key}
              data-testid={`perm-mod-pill-${pill.tone}`}
              className={`hs-mod-pill hs-mod-pill--${pill.tone}`}
              title={pill.title}
            >
              <span className="hs-mod-pill__icon">
                <ModStatusIcon kind={pill.icon} />
              </span>
              <span>{pill.label}</span>
            </span>
          ))}
          {hiddenModPills.length > 0 && (
            <span
              data-testid="perm-mod-overflow"
              className="hs-mod-pill hs-mod-pill--info"
              title={hiddenModPills.map((p) => p.title).join('\n')}
            >
              +{hiddenModPills.length}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
