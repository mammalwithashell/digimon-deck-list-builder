import { InBetweenShell } from '@/features/play/InBetweenShell';

export function DeckSelectPage() {
  return (
    <InBetweenShell
      title="SELECT DECK"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'DECK' }]}
    >
      <main style={{ padding: 32 }}>
        <h1>SELECT YOUR DECK</h1>
      </main>
    </InBetweenShell>
  );
}
