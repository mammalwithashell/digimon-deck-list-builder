import client from './client';

export type QueueType = 'casual' | 'ranked';
export type TierFilter = 'same' | 'any' | 'meta_only' | 'jank_only';
export type TicketStatus = 'waiting' | 'matched' | 'cancelled';

export interface QueueRequest {
  queue_type: QueueType;
  // One of these two shapes:
  deck_id?: string;
  main_deck?: string[];
  egg_deck?: string[];
  game_mode?: string;
  opponent_tier_filter?: TierFilter;
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
  join_code: string;
}

export type QueueResponse = WaitingResponse | MatchedResponse;

export interface TicketInfo {
  ticket_id: string;
  status: TicketStatus;
  queue_type: QueueType;
  waited_seconds: number;
  rating_window: number | null;
  join_code: string | null;
  game_id: string | null;
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
