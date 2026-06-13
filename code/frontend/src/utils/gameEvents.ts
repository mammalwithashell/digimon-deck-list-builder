import type { GameEvent } from '@/types/game';

const TYPE_ALIASES: Record<string, string> = {
  MemoryChange: 'memory_change',
  TurnStart: 'turn_start',
  PhaseChange: 'phase_change',
  Play: 'play',
  Digivolve: 'digivolve',
  Attack: 'attack',
  Trash: 'trash',
  Mill: 'mill',
  SecurityReveal: 'security_reveal',
  GameOver: 'game_over',
  Concede: 'concede',
  EffectFizzled: 'effect_fizzled',
};

export function normalizeGameEvent(event: GameEvent): GameEvent {
  const type = TYPE_ALIASES[event.type] ?? event.type;
  return type === event.type ? event : { ...event, type };
}

export function normalizeGameEvents(events: GameEvent[] | undefined): GameEvent[] {
  return events?.map(normalizeGameEvent) ?? [];
}
