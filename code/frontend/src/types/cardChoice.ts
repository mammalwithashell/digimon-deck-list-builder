/** Engine-owned page rows. Decimal strings must never round through Number. */
export interface CardChoiceRow {
  action_id: number;
  operation: 'commit_card' | 'inspect_card' | 'previous_page' | 'next_page' | 'decline' | 'refresh_page' | 'finish_impossible';
  zone: 'battle_top' | 'battle_source' | 'breeding_source' | 'security' | 'hand' | 'trash' | null;
  /** Zero-based engine seat, including after recipient normalization. */
  owner: number | null;
  carrier_slot: number | null;
  carrier_identity: string | null;
  position: string | null;
  card_id: string | null;
  card_registry_index: number | null;
  identity_visible: boolean;
}

export interface CardChoiceToken {
  prompt_id: string;
  page_generation: string;
}

export interface CardChoice extends CardChoiceToken {
  inspection_only?: boolean;
  page_index: string;
  page_count: string;
  rows: CardChoiceRow[];
}

export function cardChoiceToken(choice?: CardChoice | null): CardChoiceToken | undefined {
  return choice ? { prompt_id: choice.prompt_id, page_generation: choice.page_generation } : undefined;
}
