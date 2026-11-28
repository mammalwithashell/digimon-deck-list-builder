import type { GameEvent, GameState } from '@/types/game';
import type { BotSpeed, Motion } from '@/stores/uiStore';
import { BASE_PACING, scaleDuration } from './pacingConfig';

export interface NarrationToast {
  id: string;
  actor: 'you' | 'opp' | 'neutral';
  title: string;
  body: string;
  cardId?: string | null;
  createdAt: number;
}

export interface OpponentShowcase {
  cardId: string;
  cardName: string;
  caption: string;
}

export interface CardFlightCue {
  cardId?: string | null;
  cardName?: string | null;
  toZone: 'trash' | 'hand' | 'deck' | 'security';
  actor: 'you' | 'opp';
  label: string;
}

export interface PresentationBeat {
  id: string;
  durationMs: number;
  /** Snapshot to show during this beat (final beat commits full `nextState`). */
  snapshot: GameState;
  /** Optional permanent slot to pulse as the acting effect source. */
  activeSource?: { player: number; slot: number } | null;
  /** Optional brief centered opponent card showcase. */
  showcase?: OpponentShowcase | null;
  /** Optional directional card-flight indicator (draw / mill / trash / bounce / recovery). */
  flight?: CardFlightCue | null;
  /** Optional floating memory delta chip. */
  memoryDelta?: number | null;
  /** Optional toast pushed to the scrollable Narration Stack when this beat starts. */
  toast?: Omit<NarrationToast, 'createdAt'> | null;
}

function actorSide(player: number | undefined): 'you' | 'opp' | 'neutral' {
  if (player === 1) return 'you';
  if (player === 2) return 'opp';
  return 'neutral';
}

function cardLabel(cardName?: string | null, cardId?: string | null): string {
  if (cardName && cardId && cardName !== cardId) {
    return `${cardName} (${cardId})`;
  }
  return cardName || cardId || 'Card';
}

/**
 * Synthesize an intermediate attack-declaration board snapshot where the
 * attacking permanent is visibly suspended sideways and `pendingAttack` draws
 * the targeting arrow BEFORE combat / security deletion resolves.
 */
function buildAttackSnapshot(
  prevState: GameState | null,
  nextState: GameState,
  attackerPlayer: number,
  attackerSlot: number | null | undefined,
  targetSlot: number | null | undefined,
): GameState {
  if (!prevState || attackerSlot == null) return nextState;
  const isOpp = attackerPlayer === 2;
  const sourcePlayer = isOpp ? prevState.player2 : prevState.player1;
  const updatedBattle = sourcePlayer.battleArea.map((perm, idx) =>
    idx === attackerSlot ? { ...perm, isSuspended: true } : perm,
  );
  const updatedPlayer = { ...sourcePlayer, battleArea: updatedBattle };
  return {
    ...prevState,
    currentPlayer: isOpp ? prevState.player2.id : prevState.player1.id,
    player1: isOpp ? prevState.player1 : updatedPlayer,
    player2: isOpp ? updatedPlayer : prevState.player2,
    pendingAttack: {
      attackerSlot,
      targetSlot: targetSlot ?? 14,
    },
  };
}

/**
 * Compiles one authoritative state transition `(prevState, nextState, events)`
 * into an ordered sequence of observable presentation beats.
 *
 * - Coalesces structural `MemoryChange` into the preceding Play/Digivolve/Effect beat
 * - Adds a brief opponent showcase hold when the opponent plays/digivolves/reveals
 * - Applies chain-tail acceleration (`chainTailFrom: 2`) so multi-trigger chains
 *   remain crisp and never drag
 */
export function buildPresentationBeats(params: {
  prevState: GameState | null;
  nextState: GameState;
  events: GameEvent[];
  speed: BotSpeed;
  motion: Motion;
}): PresentationBeat[] {
  const { prevState, nextState, events, speed, motion } = params;
  if (speed === 'instant' || motion === 'off' || events.length === 0) {
    return [];
  }

  const beats: PresentationBeat[] = [];
  let chainCount = 0;

  for (let i = 0; i < events.length; i++) {
    const ev = events[i]!;
    const raw = ev as unknown as Record<string, unknown>;
    const side = actorSide(ev.player);
    const isOpp = side === 'opp';
    const isLast = i === events.length - 1;
    const snap = isLast || !prevState ? nextState : prevState;
    const beatId = `beat-${ev.seq ?? i}-${ev.type}-${i}`;

    switch (ev.type) {
      case 'turn_start': {
        const turnNum = typeof raw.turn_count === 'number' ? raw.turn_count : nextState.turnCount;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.turnStartMs, speed, motion),
          snapshot: snap,
          toast: {
            id: beatId,
            actor: side,
            title: isOpp ? `Opponent's Turn ${turnNum}` : `Your Turn ${turnNum}`,
            body: isOpp ? 'Opponent begins their turn.' : 'Your turn begins.',
          },
        });
        break;
      }

      case 'play': {
        const cardId = (raw.card_id as string | undefined) ?? ev.source_card_id ?? '';
        const cardName = (raw.card_name as string | undefined) ?? ev.source_card_name ?? cardId;
        const fieldIdx = typeof raw.field_index === 'number' ? raw.field_index : ev.source_slot;
        const costPaid = typeof raw.cost_paid === 'number' ? raw.cost_paid : undefined;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(
            isOpp ? BASE_PACING.oppPlayShowcaseMs : BASE_PACING.ownPlayMs,
            speed,
            motion,
            chainCount,
          ),
          snapshot: nextState,
          activeSource: fieldIdx != null ? { player: ev.player, slot: fieldIdx } : null,
          showcase:
            isOpp && cardId
              ? {
                  cardId,
                  cardName,
                  caption:
                    costPaid != null ? `Opponent played (cost ${costPaid})` : 'Opponent played',
                }
              : null,
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: isOpp ? 'Opponent Played' : 'Played',
            body:
              costPaid != null
                ? `${cardLabel(cardName, cardId)} (paid ${costPaid})`
                : cardLabel(cardName, cardId),
          },
        });
        chainCount += 1;
        break;
      }

      case 'digivolve': {
        const cardId = (raw.top_card_id as string | undefined) ?? ev.source_card_id ?? '';
        const cardName = (raw.card_name as string | undefined) ?? ev.source_card_name ?? cardId;
        const fieldIdx = typeof raw.field_index === 'number' ? raw.field_index : ev.source_slot;
        const wasDna = Boolean(raw.was_dna);
        beats.push({
          id: beatId,
          durationMs: scaleDuration(
            isOpp ? BASE_PACING.oppDigivolveMs : BASE_PACING.ownDigivolveMs,
            speed,
            motion,
            chainCount,
          ),
          snapshot: nextState,
          activeSource: fieldIdx != null ? { player: ev.player, slot: fieldIdx } : null,
          showcase:
            isOpp && cardId
              ? {
                  cardId,
                  cardName,
                  caption: wasDna ? 'Opponent DNA Digivolved' : 'Opponent Digivolved',
                }
              : null,
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: wasDna ? 'DNA Digivolve' : 'Digivolve',
            body: `${isOpp ? 'Opponent' : 'You'} digivolved into ${cardLabel(cardName, cardId)}`,
          },
        });
        chainCount += 1;
        break;
      }

      case 'hatch':
      case 'breeding_move': {
        const cardId = (raw.card_id as string | undefined) ?? ev.source_card_id ?? '';
        const cardName = (raw.card_name as string | undefined) ?? ev.source_card_name ?? cardId;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.hatchOrMoveMs, speed, motion),
          snapshot: nextState,
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: ev.type === 'hatch' ? 'Hatched' : 'Promoted',
            body: `${isOpp ? 'Opponent' : 'You'} ${
              ev.type === 'hatch' ? 'hatched' : 'moved'
            } ${cardLabel(cardName, cardId)}`,
          },
        });
        break;
      }

      case 'attack': {
        const atkId = (raw.attacker_card_id as string | undefined) ?? ev.source_card_id;
        const atkName = (raw.attacker_card_name as string | undefined) ?? ev.source_card_name;
        const tgtId = (raw.target_card_id as string | undefined) ?? ev.target_card_id;
        const tgtName = (raw.target_card_name as string | undefined) ?? ev.target_card_name;
        const atkSlot =
          typeof raw.attacker_field_index === 'number' ? raw.attacker_field_index : ev.source_slot;
        const tgtSlot =
          typeof raw.target_field_index === 'number' ? raw.target_field_index : ev.target_slot;
        const attackSnap = buildAttackSnapshot(prevState, nextState, ev.player, atkSlot, tgtSlot);
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.attackDeclareMs, speed, motion),
          snapshot: attackSnap,
          activeSource: atkSlot != null ? { player: ev.player, slot: atkSlot } : null,
          showcase:
            isOpp && atkId
              ? {
                  cardId: atkId,
                  cardName: cardLabel(atkName, atkId),
                  caption: tgtName
                    ? `Attacking ${cardLabel(tgtName, tgtId)}`
                    : 'Attacking Security!',
                }
              : null,
          toast: {
            id: beatId,
            actor: side,
            cardId: atkId,
            title: isOpp ? 'Opponent Attacks' : 'Attack Declared',
            body: `${cardLabel(atkName, atkId)} → ${tgtName ? cardLabel(tgtName, tgtId) : 'Security'}`,
          },
        });
        break;
      }

      case 'security_reveal': {
        const cardId = (raw.card_id as string | undefined) ?? ev.source_card_id ?? '';
        const cardName = (raw.card_name as string | undefined) ?? ev.source_card_name ?? cardId;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.securityRevealMs, speed, motion),
          snapshot: nextState,
          showcase: cardId
            ? { cardId, cardName, caption: 'Security Check' }
            : null,
          toast: {
            id: beatId,
            actor: 'neutral',
            cardId,
            title: 'Security Revealed',
            body: cardLabel(cardName, cardId),
          },
        });
        break;
      }

      case 'effect_activated':
      case 'effect_target': {
        const cardId = (raw.source_card_id as string | undefined) ?? ev.source_card_id;
        const cardName = (raw.source_card_name as string | undefined) ?? ev.source_card_name;
        const slot = typeof raw.source_slot === 'number' ? raw.source_slot : ev.source_slot;
        const timing = (raw.timing as string | undefined) ?? null;
        const summary = (raw.summary as string | undefined) ?? null;
        const targets = Array.isArray(raw.targets)
          ? (raw.targets as { card_name?: string; card_id?: string }[])
              .map((t) => t.card_name || t.card_id)
              .filter(Boolean)
              .join(', ')
          : '';
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.effectSourceMs, speed, motion, chainCount),
          snapshot: nextState,
          activeSource: slot != null ? { player: ev.player, slot } : null,
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: timing
              ? `${cardLabel(cardName, cardId)} [${timing}]`
              : cardLabel(cardName, cardId),
            body:
              ev.type === 'effect_target' && targets
                ? `Targeted ${targets}`
                : summary || 'Effect activated',
          },
        });
        chainCount += 1;
        break;
      }

      case 'memory_change': {
        const delta = typeof raw.delta === 'number' ? raw.delta : 0;
        const srcCard = (raw.source_card_name as string | undefined) ?? (raw.source_card_id as string | undefined);
        // Coalesce structural cost-payment memory changes onto the previous beat so they don't stall
        const prev = beats[beats.length - 1];
        if (!srcCard && prev && prev.memoryDelta == null) {
          prev.memoryDelta = delta;
        } else if (delta !== 0) {
          beats.push({
            id: beatId,
            durationMs: scaleDuration(BASE_PACING.phaseTickMs, speed, motion, chainCount),
            snapshot: nextState,
            memoryDelta: delta,
            toast: srcCard
              ? {
                  id: beatId,
                  actor: side,
                  title: srcCard,
                  body: `Memory ${delta > 0 ? `+${delta}` : delta}`,
                }
              : null,
          });
        }
        break;
      }

      case 'draw':
      case 'return_to_hand':
      case 'card_to_hand': {
        const cardId = (raw.card_id as string | undefined) ?? ev.source_card_id;
        const cardName = (raw.card_name as string | undefined) ?? ev.source_card_name;
        const count = typeof raw.count === 'number' ? raw.count : 1;
        const label =
          ev.type === 'draw'
            ? `${isOpp ? 'Opponent' : 'You'} drew ${count > 1 ? `${count} cards` : 'a card'}`
            : `Returned ${cardLabel(cardName, cardId)} to hand`;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.cardMoveMs, speed, motion, chainCount),
          snapshot: nextState,
          flight: {
            cardId: isOpp && ev.type === 'draw' ? null : cardId,
            cardName,
            toZone: 'hand',
            actor: isOpp ? 'opp' : 'you',
            label: ev.type === 'draw' ? (count > 1 ? `Draw ×${count}` : 'Draw') : 'To Hand',
          },
          toast:
            ev.type !== 'draw' || count > 1
              ? {
                  id: beatId,
                  actor: side,
                  cardId: isOpp && ev.type === 'draw' ? null : cardId,
                  title: ev.type === 'draw' ? 'Card Draw' : 'Bounce to Hand',
                  body: label,
                }
              : null,
        });
        chainCount += 1;
        break;
      }

      case 'trash':
      case 'card_trashed':
      case 'mill':
      case 'delete':
      case 'deletion': {
        const cardId = (raw.card_id as string | undefined) ?? ev.target_card_id ?? ev.source_card_id;
        const cardName =
          (raw.card_name as string | undefined) ?? ev.target_card_name ?? ev.source_card_name;
        const isDelete = ev.type === 'delete' || ev.type === 'deletion';
        const isMill = ev.type === 'mill';
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.cardMoveMs, speed, motion, chainCount),
          snapshot: nextState,
          flight: {
            cardId,
            cardName,
            toZone: 'trash',
            actor: isOpp ? 'opp' : 'you',
            label: isDelete ? 'Deleted' : isMill ? 'Milled' : 'To Trash',
          },
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: isDelete ? 'Deleted' : isMill ? 'Milled to Trash' : 'Trashed',
            body: cardLabel(cardName, cardId),
          },
        });
        chainCount += 1;
        break;
      }

      case 'return_to_deck':
      case 'card_to_deck': {
        const cardId = (raw.card_id as string | undefined) ?? ev.target_card_id ?? ev.source_card_id;
        const cardName =
          (raw.card_name as string | undefined) ?? ev.target_card_name ?? ev.source_card_name;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.cardMoveMs, speed, motion, chainCount),
          snapshot: nextState,
          flight: {
            cardId,
            cardName,
            toZone: 'deck',
            actor: isOpp ? 'opp' : 'you',
            label: 'To Deck',
          },
          toast: {
            id: beatId,
            actor: side,
            cardId,
            title: 'Returned to Deck',
            body: cardLabel(cardName, cardId),
          },
        });
        chainCount += 1;
        break;
      }

      case 'recovery': {
        const count = typeof raw.count === 'number' ? raw.count : 1;
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.cardMoveMs, speed, motion, chainCount),
          snapshot: nextState,
          flight: {
            toZone: 'security',
            actor: isOpp ? 'opp' : 'you',
            label: `Recovery +${count}`,
          },
          toast: {
            id: beatId,
            actor: side,
            title: 'Recovery',
            body: `${isOpp ? 'Opponent' : 'You'} recovered +${count} security`,
          },
        });
        chainCount += 1;
        break;
      }

      case 'effect_fizzled': {
        const reason = (raw.reason as string | undefined) ?? 'No valid target';
        beats.push({
          id: beatId,
          durationMs: scaleDuration(BASE_PACING.cardMoveMs, speed, motion, chainCount),
          snapshot: nextState,
          toast: {
            id: beatId,
            actor: 'neutral',
            title: 'Effect Fizzled',
            body: reason,
          },
        });
        break;
      }

      default:
        break;
    }
  }

  return beats;
}
