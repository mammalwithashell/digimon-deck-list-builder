import { InBetweenShell } from '@/features/play/InBetweenShell';

export function ModeSelectPage() {
  return (
    <InBetweenShell
      title="CHOOSE FORMAT"
      stepLabel="01"
      crumbs={[{ label: 'PLAY' }, { label: 'FORMAT' }]}
      rightSlot={<span>STANDARD</span>}
    >
      <main style={{ padding: 32 }}>
        <h1>CHOOSE YOUR FORMAT</h1>
        <button type="button">QUICK MATCH</button>
      </main>
    </InBetweenShell>
  );
}
