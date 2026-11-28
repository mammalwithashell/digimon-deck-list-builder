import { useRef, useEffect, useMemo, useState } from 'react';
import { useDroppable } from '@dnd-kit/core';
import { PermanentSlot, isNonDigimonPermanent } from './PermanentSlot';
import { Card } from '@/components/shared/Card';
import { useUiStore } from '@/stores/uiStore';
import type { PermanentInfo } from '@/types/game';
import { MAX_BATTLE_AREA_SLOTS } from '@/utils/constants';
import { usePositionTransitions } from '@/hooks/usePositionTransitions';

interface DroppableSlotProps {
  slotIndex: number;
  isEmpty: boolean;
  isOpponent: boolean;
  isValidDrop: boolean;
  isDigivolveTarget: boolean;
  isAttackDropTarget: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

function DroppableSlot({
  slotIndex,
  isEmpty,
  isOpponent,
  isValidDrop,
  isDigivolveTarget,
  isAttackDropTarget,
  children,
  onClick,
}: DroppableSlotProps) {
  const dropType = isOpponent
    ? 'enemy-field-slot'
    : isEmpty
      ? 'empty-field-slot'
      : 'occupied-field-slot';
  const owner = isOpponent ? 'opponent' : 'player';
  const { isOver, setNodeRef } = useDroppable({
    id: `field-slot-${owner}-${slotIndex}`,
    data: { type: dropType, slotIndex },
    disabled: isOpponent ? !isAttackDropTarget : false,
  });

  const showDropHighlight = isOver && isValidDrop;

  return (
    <div
      ref={setNodeRef}
      data-testid={`field-slot-${isOpponent ? 'p2' : 'p1'}-${slotIndex}`}
      data-slot-id={`${owner}-${slotIndex}`}
      className={`ib-drop-slot ${
        showDropHighlight
          ? isOpponent
            ? 'ib-drop-slot--attack'
            : isEmpty
              ? 'ib-drop-slot--play'
              : 'ib-drop-slot--digivolve'
          : isDigivolveTarget
            ? 'ib-drop-slot--digivolve-soft'
            : ''
      }`}
      onClick={onClick}
    >
      {children}
    </div>
  );
}

interface BattleAreaProps {
  permanents: PermanentInfo[];
  isOpponent: boolean;
  highlightedSlots?: Set<number>;
  selectedSlots?: Set<number>;
  targetedSlots?: Set<number>;
  attackableSlots?: Set<number>;
  onSlotClick?: (slotIndex: number) => void;
  onSlotHover?: (slotIndex: number | null) => void;
  onSlotInspect?: (slotIndex: number) => void;
  dragValidDropSlots?: Set<number>;
  isDraggingHandCard?: boolean;
  canPlayDragged?: boolean;
  isDraggingAttacker?: boolean;
}

interface RenderCluster {
  primarySlot: number;
  behindSlots: number[];
}

interface ClusterEntry {
  cluster: RenderCluster;
  flipKey: string;
  isBack: boolean;
}

/**
 * Open, centered Hearthstone/Aegis-style battle arena:
 * - Renders only occupied permanents with smooth FLIP reordering
 * - When `battleLanes === 'two'`, separates Digimon/Tokens (front row, closest
 *   to the midline Memory Band) from Tamers & Delay Options (back row),
 *   mirroring row order for opponent vs player and collapsing any empty row
 * - Stacks duplicate Tamers & Delay Options (non-Digimon without sources)
 *   to save board space, showing suspended copies tucked sideways behind
 *   unsuspended copies (auto-unstacking if 2+ copies are independently targetable)
 * - Supports drag-to-attack drops onto opponent permanents
 */
export function BattleArea({
  permanents,
  isOpponent,
  highlightedSlots,
  selectedSlots,
  targetedSlots,
  attackableSlots,
  onSlotClick,
  onSlotHover,
  onSlotInspect,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
  isDraggingAttacker = false,
}: BattleAreaProps) {
  const battleLanes = useUiStore((s) => s.battleLanes);
  const isTwoLanes = battleLanes === 'two';

  const prevCardIds = useRef<(string | null)[]>(
    Array(MAX_BATTLE_AREA_SLOTS).fill(null),
  );
  const [animatingSlots, setAnimatingSlots] = useState<Set<number>>(new Set());

  const occupiedIndices = useMemo(() => {
    const out: number[] = [];
    for (let i = 0; i < MAX_BATTLE_AREA_SLOTS; i++) {
      if (permanents[i]) out.push(i);
    }
    return out;
  }, [permanents]);

  // Group duplicate Tamers & Delay Options by topCardId (Aegis-style layering)
  const clusters = useMemo<RenderCluster[]>(() => {
    const byTamerCardId = new Map<string, number[]>();
    for (const idx of occupiedIndices) {
      const p = permanents[idx]!;
      if (p.topCardId && p.sourceCount === 0 && isNonDigimonPermanent(p)) {
        const list = byTamerCardId.get(p.topCardId) ?? [];
        list.push(idx);
        byTamerCardId.set(p.topCardId, list);
      }
    }

    const emitted = new Set<number>();
    const result: RenderCluster[] = [];
    for (const idx of occupiedIndices) {
      if (emitted.has(idx)) continue;
      const p = permanents[idx]!;
      const group = p.topCardId ? byTamerCardId.get(p.topCardId) : undefined;
      if (!group || group.length < 2) {
        emitted.add(idx);
        result.push({ primarySlot: idx, behindSlots: [] });
        continue;
      }

      // If 2+ copies in this group are independently legal targets or already
      // selected, keep them separated so every choice stays directly clickable.
      const interactiveCount = group.filter(
        (s) => highlightedSlots?.has(s) || selectedSlots?.has(s) || targetedSlots?.has(s),
      ).length;
      if (interactiveCount >= 2) {
        emitted.add(idx);
        result.push({ primarySlot: idx, behindSlots: [] });
        continue;
      }

      // Choose the front copy: interactive first, then unsuspended, then earliest slot.
      const sorted = [...group].sort((a, b) => {
        const aInt = highlightedSlots?.has(a) || selectedSlots?.has(a) || targetedSlots?.has(a) ? 0 : 1;
        const bInt = highlightedSlots?.has(b) || selectedSlots?.has(b) || targetedSlots?.has(b) ? 0 : 1;
        if (aInt !== bInt) return aInt - bInt;
        const aSusp = permanents[a]!.isSuspended ? 1 : 0;
        const bSusp = permanents[b]!.isSuspended ? 1 : 0;
        if (aSusp !== bSusp) return aSusp - bSusp;
        return a - b;
      });
      for (const s of sorted) emitted.add(s);
      result.push({ primarySlot: sorted[0]!, behindSlots: sorted.slice(1) });
    }
    return result;
  }, [occupiedIndices, permanents, highlightedSlots, selectedSlots, targetedSlots]);

  const firstEmptySlot = useMemo(() => {
    for (let i = 0; i < MAX_BATTLE_AREA_SLOTS; i++) {
      if (!permanents[i]) return i;
    }
    return null;
  }, [permanents]);

  const clusterEntries = useMemo<ClusterEntry[]>(() => {
    const seen = new Map<string, number>();
    return clusters.map((cluster) => {
      const p = permanents[cluster.primarySlot]!;
      const baseKey = `${isOpponent ? 'opp' : 'me'}:${p.topCardId ?? '?'}:${p.turnPlayed}`;
      const occurrence = seen.get(baseKey) ?? 0;
      seen.set(baseKey, occurrence + 1);
      return {
        cluster,
        flipKey: `${baseKey}:${occurrence}`,
        isBack: isNonDigimonPermanent(p),
      };
    });
  }, [clusters, permanents, isOpponent]);

  const frontEntries = useMemo(
    () => clusterEntries.filter((entry) => !entry.isBack),
    [clusterEntries],
  );
  const backEntries = useMemo(
    () => clusterEntries.filter((entry) => entry.isBack),
    [clusterEntries],
  );

  const flipKeys = useMemo(
    () => clusterEntries.map((entry) => entry.flipKey),
    [clusterEntries],
  );

  const { registerNode } = usePositionTransitions(flipKeys, { durationMs: 240 });

  useEffect(() => {
    const newAnimating = new Set<number>();
    for (let i = 0; i < MAX_BATTLE_AREA_SLOTS; i++) {
      const currentId = permanents[i]?.topCardId ?? null;
      const prevId = prevCardIds.current[i];
      if (currentId && currentId !== prevId) {
        newAnimating.add(i);
      }
      prevCardIds.current[i] = currentId;
    }
    if (newAnimating.size > 0) {
      setAnimatingSlots(newAnimating);
      const timer = setTimeout(() => setAnimatingSlots(new Set()), 380);
      return () => clearTimeout(timer);
    }
  }, [permanents]);

  const showPlayDropMat =
    !isOpponent && isDraggingHandCard && canPlayDragged && firstEmptySlot !== null;

  const renderCluster = ({ cluster, flipKey }: ClusterEntry) => {
    const { primarySlot: i, behindSlots } = cluster;
    const perm = permanents[i]!;
    const isAttackDrop = isOpponent && isDraggingAttacker && Boolean(targetedSlots?.has(i));
    const totalInStack = 1 + behindSlots.length;
    const restedInStack =
      (perm.isSuspended ? 1 : 0) +
      behindSlots.filter((s) => permanents[s]?.isSuspended).length;

    return (
      <DroppableSlot
        key={i}
        slotIndex={i}
        isEmpty={false}
        isOpponent={isOpponent}
        isValidDrop={
          isOpponent
            ? isAttackDrop
            : isDraggingHandCard
              ? (dragValidDropSlots?.has(i) ?? false)
              : true
        }
        isDigivolveTarget={
          !isOpponent && isDraggingHandCard && (dragValidDropSlots?.has(i) ?? false)
        }
        isAttackDropTarget={isAttackDrop}
        onClick={() => onSlotClick?.(i)}
      >
        <div
          ref={registerNode(flipKey)}
          className={`hs-perm-cluster ${animatingSlots.has(i) ? 'animate-card-play-in' : ''}`}
        >
          {/* Layered duplicate Tamers / Delay Options tucked neatly behind the front card */}
          {behindSlots.slice(0, 2).map((behindIdx, layerIdx) => {
            const behindPerm = permanents[behindIdx]!;
            if (!behindPerm.topCardId) return null;
            return (
              <div
                key={behindIdx}
                className={`hs-dup-stack__behind ${
                  behindPerm.isSuspended ? 'is-suspended' : ''
                }`}
                style={{
                  transform: behindPerm.isSuspended
                    ? `translate(${12 + layerIdx * 6}px, ${4 + layerIdx * 4}px) rotate(90deg)`
                    : `translate(${8 + layerIdx * 6}px, ${-6 - layerIdx * 4}px)`,
                  zIndex: 0,
                }}
                aria-hidden="true"
              >
                <Card
                  cardId={behindPerm.topCardId}
                  cardName={behindPerm.topCardName ?? undefined}
                  cardColor={behindPerm.colors[0]}
                  size="md"
                />
              </div>
            );
          })}

          <div className="relative z-[2]">
            <PermanentSlot
              perm={perm}
              slotIndex={i}
              isOpponent={isOpponent}
              highlighted={highlightedSlots?.has(i)}
              selected={selectedSlots?.has(i)}
              targeted={targetedSlots?.has(i)}
              canAttack={!isOpponent && Boolean(attackableSlots?.has(i))}
              onMouseEnter={() => onSlotHover?.(i)}
              onMouseLeave={() => onSlotHover?.(null)}
              onInspect={onSlotInspect ? () => onSlotInspect(i) : undefined}
            />
          </div>

          {totalInStack > 1 && (
            <span
              data-testid={`tamer-stack-badge-${i}`}
              className="hs-dup-stack__badge"
              title={
                restedInStack > 0
                  ? `${totalInStack} copies (${restedInStack} suspended)`
                  : `${totalInStack} copies`
              }
            >
              ×{totalInStack}
            </span>
          )}
        </div>
      </DroppableSlot>
    );
  };

  const playDropMatNode = showPlayDropMat ? (
    <DroppableSlot
      key="play-drop-mat"
      slotIndex={firstEmptySlot}
      isEmpty
      isOpponent={false}
      isValidDrop
      isDigivolveTarget={false}
      isAttackDropTarget={false}
      onClick={() => onSlotClick?.(firstEmptySlot)}
    >
      <div className="hs-play-drop-mat">Play Here</div>
    </DroppableSlot>
  ) : null;

  const isEmptyBoard = occupiedIndices.length === 0 && !showPlayDropMat;

  const frontLane =
    frontEntries.length > 0 || showPlayDropMat ? (
      <div
        key="front"
        className="hs-battle-lane hs-battle-lane--front"
        data-testid={isOpponent ? 'battle-lane-opp-front' : 'battle-lane-you-front'}
        role="group"
        aria-label={isOpponent ? 'Opponent Front Row (Digimon)' : 'Your Front Row (Digimon)'}
      >
        {frontEntries.map(renderCluster)}
        {playDropMatNode}
      </div>
    ) : null;

  const backLane =
    backEntries.length > 0 ? (
      <div
        key="back"
        className="hs-battle-lane hs-battle-lane--back"
        data-testid={isOpponent ? 'battle-lane-opp-back' : 'battle-lane-you-back'}
        role="group"
        aria-label={
          isOpponent ? 'Opponent Back Row (Tamers & Options)' : 'Your Back Row (Tamers & Options)'
        }
      >
        {backEntries.map(renderCluster)}
      </div>
    ) : null;

  return (
    <div
      className={`hs-battle-area${isTwoLanes ? ' hs-battle-area--laned' : ''}`}
      data-testid={isOpponent ? 'battle-area-opp' : 'battle-area-you'}
      data-battle-lanes={battleLanes}
    >
      {isEmptyBoard ? (
        <div className="hs-battle-area--empty-hint">
          {isOpponent ? 'Opponent Battle Area' : 'Battle Area'}
        </div>
      ) : isTwoLanes ? (
        isOpponent ? (
          <>
            {backLane}
            {frontLane}
          </>
        ) : (
          <>
            {frontLane}
            {backLane}
          </>
        )
      ) : (
        <>
          {clusterEntries.map(renderCluster)}
          {playDropMatNode}
        </>
      )}
    </div>
  );
}
