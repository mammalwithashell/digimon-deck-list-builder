/** Decimal strings preserve exact counts and physical identities across JSON. */
export interface CountChoice {
  operation: 'commit_count' | 'previous_page' | 'next_page';
  declared_count: string | null;
  payable_count: string | null;
  page_index: string;
  page_count: string;
  host: { player: number; field_index: number; battle_identity: string } | null;
}
