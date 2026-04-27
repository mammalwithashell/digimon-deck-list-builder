import { InBetweenShell } from '@/features/play/InBetweenShell';

export function MatchingPage() {
  return (
    <InBetweenShell
      title="MATCHMAKING"
      stepLabel="03"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'MATCHMAKING' }]}
    >
      <main style={{ padding: 32 }}>
        <h1>FINDING OPPONENT</h1>
      </main>
    </InBetweenShell>
  );
}
