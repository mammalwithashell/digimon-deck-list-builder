import type { GameEvent, GameState, PlayerState } from '@/types/game';
import { normalizeGameEvent } from './gameEvents';

export interface GameLogFormatContext {
  state: GameState;
  playerLabels?: Record<number, string>;
}

function playerLabel(player: number, labels?: Record<number, string>): string {
  return labels?.[player] ?? `Player ${player}`;
}

function possessive(name: string): string {
  return name.endsWith('s') ? `${name}'` : `${name}'s`;
}

function zoneFor(state: GameState, player: number): PlayerState | null {
  return player === 1 ? state.player1 : player === 2 ? state.player2 : null;
}

function boardNameForSlot(
  state: GameState,
  player: number,
  slot: number | null | undefined,
): string | null {
  if (slot == null) return null;
  const zone = zoneFor(state, player);
  return zone?.battleArea[slot]?.topCardName ?? null;
}

/**
 * Render a card as `[CARD-ID: Name]` from event-carried identity, with the
 * fallback chain: event name → board-state name → bare id → null. Never
 * throws and never returns an empty string for a present id.
 */
function displayCard(
  cardId: string | null | undefined,
  cardName: string | null | undefined,
  fallbackName?: string | null,
): string | null {
  const name = cardName || fallbackName || null;
  if (cardId) {
    return `[${cardId}: ${name ?? cardId}]`;
  }
  // No id available — fall back to whatever name we could resolve.
  return name;
}

function memoryWord(value: number): string {
  return value === 1 ? 'memory' : 'memory';
}

/** Annotate a rendered card label with its effective DP, when known. */
function withDp(label: string, dp: number | null | undefined): string {
  return typeof dp === 'number' ? `${label} (${dp} DP)` : label;
}

interface EffectTargetEntry {
  card_id?: string | null;
  card_name?: string | null;
}

export function formatEvent(
  rawEvent: GameEvent,
  ctx: GameLogFormatContext,
): string[] {
  const event = normalizeGameEvent(rawEvent);
  const actor = playerLabel(event.player, ctx.playerLabels);
  const metaCardName =
    (event.meta.card_name as string | undefined) ??
    (event.meta.source_card_name as string | undefined);
  const card = displayCard(
    event.source_card_id,
    event.source_card_name ?? metaCardName,
    boardNameForSlot(ctx.state, event.player, event.source_slot),
  );

  switch (event.type) {
    case 'play': {
      if (!card) return [];
      const cost = event.meta.cost_paid;
      return typeof cost === 'number'
        ? [`${actor} played ${card} for ${cost} ${memoryWord(cost)}.`]
        : [`${actor} played ${card}.`];
    }
    case 'digivolve': {
      if (!card) return [];
      const cost = event.meta.memory_paid;
      return typeof cost === 'number'
        ? [`${actor} digivolved into ${card} for ${cost} ${memoryWord(cost)}.`]
        : [`${actor} digivolved into ${card}.`];
    }
    case 'attack': {
      const attackerDp = event.meta.attacker_dp as number | null | undefined;
      const targetDp = event.meta.target_dp as number | null | undefined;
      const attacker = withDp(card ?? `slot ${event.source_slot}`, attackerDp);
      const targetPlayer = event.meta.target_player as number | null | undefined;
      // A Digimon target carries identity; otherwise the attack hit security.
      const targetCard = displayCard(
        event.target_card_id,
        event.target_card_name,
        targetPlayer != null
          ? boardNameForSlot(ctx.state, targetPlayer, event.target_slot)
          : null,
      );
      const target = targetCard
        ? withDp(targetCard, targetDp)
        : event.target_slot != null
          ? `slot ${event.target_slot}`
          : 'security';
      return [`${actor} attacked ${target} with ${attacker}.`];
    }
    case 'security_reveal': {
      if (!card) return [];
      return [`${possessive(actor)} security revealed ${card}.`];
    }
    case 'trash': {
      if (!card) return [];
      return [`${actor} trashed ${card}.`];
    }
    case 'mill': {
      if (!card) return [];
      return [`${actor} milled ${card}.`];
    }
    case 'reveal': {
      if (!card) return [];
      const zone = event.meta.source_zone as string | undefined;
      switch (zone) {
        case 'TrashFromDeckTop':
          return [`${actor} trashed ${card} from the top of their deck.`];
        case 'DeckTop':
        default:
          return [`${actor} revealed ${card} from the top of their deck.`];
      }
    }
    case 'effect_target': {
      if (!card) return [];
      const rawTargets = event.meta.targets;
      const targets = Array.isArray(rawTargets)
        ? (rawTargets as EffectTargetEntry[])
            .map((t) => displayCard(t.card_id, t.card_name))
            .filter((t): t is string => Boolean(t))
        : [];
      if (targets.length === 0) return [];
      return [`${possessive(card)} effect targeted ${targets.join(', ')}.`];
    }
    case 'memory_change': {
      const delta = event.meta.delta;
      const total = event.meta.total;
      if (typeof delta !== 'number') return [];
      const direction = delta < 0 ? 'lost' : 'gained';
      const amount = Math.abs(delta);
      const suffix = typeof total === 'number' ? ` (now ${total})` : '';
      const fromSource = card ? ` from ${card}` : '';
      return [
        `${actor} ${direction} ${amount} ${memoryWord(amount)}${fromSource}${suffix}.`,
      ];
    }
    case 'turn_start': {
      const turn = event.meta.turn_count;
      return typeof turn === 'number'
        ? [`Turn ${turn}: ${actor}'s turn started.`]
        : [`${actor}'s turn started.`];
    }
    case 'phase_change': {
      const phase = event.meta.phase;
      return typeof phase === 'string' ? [`${actor} entered ${phase}.`] : [];
    }
    case 'game_over': {
      const winner = event.meta.winner as number | null | undefined;
      if (winner == null) return ['The game ended in a draw.'];
      return [`${playerLabel(winner, ctx.playerLabels)} won the game.`];
    }
    case 'concede':
      return [`${actor} conceded.`];
    case 'effect_fizzled': {
      const reason = event.meta.reason;
      return typeof reason === 'string' ? [`An effect fizzled: ${reason}.`] : [];
    }
    default:
      return [];
  }
}

export function formatEvents(
  events: GameEvent[],
  ctx: GameLogFormatContext,
): string[] {
  const sorted = [...events].sort((a, b) => a.seq - b.seq);

  // A deck mill emits a `Reveal{TrashFromDeckTop}` (the line we render) plus a
  // paired canonical `Trash` (for event-stream consumers). Suppress that paired
  // Trash line so milling shows once. Multiset-matched by (player, card_id)
  // within this batch so unrelated trashes (hand discard, deletion) still show.
  const milled = new Map<string, number>();
  for (const raw of sorted) {
    const ev = normalizeGameEvent(raw);
    if (
      ev.type === 'reveal' &&
      ev.meta.source_zone === 'TrashFromDeckTop' &&
      ev.source_card_id
    ) {
      const key = `${ev.player}|${ev.source_card_id}`;
      milled.set(key, (milled.get(key) ?? 0) + 1);
    }
  }

  return sorted.flatMap((raw) => {
    const ev = normalizeGameEvent(raw);
    if (ev.type === 'trash' && ev.source_card_id) {
      const key = `${ev.player}|${ev.source_card_id}`;
      const remaining = milled.get(key) ?? 0;
      if (remaining > 0) {
        milled.set(key, remaining - 1);
        return [];
      }
    }
    return formatEvent(raw, ctx);
  });
}
