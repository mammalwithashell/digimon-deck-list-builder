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

function cardNameFromState(event: GameEvent, state: GameState): string | null {
  const slot = event.source_slot;
  if (slot == null) return null;
  const zone = zoneFor(state, event.player);
  return zone?.battleArea[slot]?.topCardName ?? null;
}

function cardRef(cardId: string | null, name: string | null | undefined): string | null {
  if (!cardId) return null;
  return `[${cardId}:${name || cardId}]`;
}

function sourceCardRef(event: GameEvent, ctx: GameLogFormatContext): string | null {
  const name =
    (event.meta.card_name as string | undefined) ??
    (event.meta.source_card_name as string | undefined) ??
    cardNameFromState(event, ctx.state);
  return cardRef(event.source_card_id, name);
}

function memoryWord(value: number): string {
  return value === 1 ? 'memory' : 'memory';
}

export function formatEvent(
  rawEvent: GameEvent,
  ctx: GameLogFormatContext,
): string[] {
  const event = normalizeGameEvent(rawEvent);
  const actor = playerLabel(event.player, ctx.playerLabels);
  const card = sourceCardRef(event, ctx);

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
      const attacker = card ?? `slot ${event.source_slot}`;
      const targetPlayer = event.meta.target_player as number | null | undefined;
      const target =
        targetPlayer != null
          ? playerLabel(targetPlayer, ctx.playerLabels)
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
    case 'memory_change': {
      const delta = event.meta.delta;
      const total = event.meta.total;
      if (typeof delta !== 'number') return [];
      const direction = delta < 0 ? 'lost' : 'gained';
      const amount = Math.abs(delta);
      const suffix = typeof total === 'number' ? ` (now ${total})` : '';
      return [`${actor} ${direction} ${amount} ${memoryWord(amount)}${suffix}.`];
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
  return [...events]
    .sort((a, b) => a.seq - b.seq)
    .flatMap((event) => formatEvent(event, ctx));
}
