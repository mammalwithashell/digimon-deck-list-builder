import client from './client';

export type QueueType = 'jank' | 'casual' | 'sweat' | 'ranked';
export type TicketStatus = 'waiting' | 'matched' | 'cancelled';

export interface QueueRequest {
  queue_type: QueueType;
  // One of these two shapes:
  deck_id?: string;
  main_deck?: string[];
  egg_deck?: string[];
  game_mode?: string;
}

export interface MatchmakingConfig {
  ranked_enabled: boolean;
  queues: QueueType[];
}

export async function getConfig(): Promise<MatchmakingConfig> {
  const { data } = await client.get<MatchmakingConfig>('/matchmaking/config');
  return data;
}

export interface WaitingResponse {
  status: 'waiting';
  ticket_id: string;
}

export interface MatchedResponse {
  status: 'matched';
  ticket_id: string;
  opponent_ticket_id: string;
  game_id: string;
  your_seat: 1 | 2;
}

export type QueueResponse = WaitingResponse | MatchedResponse;

export interface TicketInfo {
  ticket_id: string;
  status: TicketStatus;
  queue_type: QueueType;
  waited_seconds: number;
  rating_window: number | null;
  game_id: string | null;
  your_seat: 1 | 2 | null;
}

export async function queue(req: QueueRequest): Promise<QueueResponse> {
  const { data } = await client.post<QueueResponse>('/matchmaking/queue', req);
  return data;
}

export async function getTicket(ticketId: string): Promise<TicketInfo> {
  const { data } = await client.get<TicketInfo>(`/matchmaking/queue/${ticketId}`);
  return data;
}

export async function cancelTicket(ticketId: string): Promise<void> {
  await client.delete(`/matchmaking/queue/${ticketId}`);
}
