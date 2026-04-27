import { InBetweenShell } from '@/features/play/InBetweenShell';

export function RoomLobbyPage() {
  return (
    <InBetweenShell
      title="ROOM LOBBY"
      stepLabel="03"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'ROOM MATCH' }, { label: 'LOBBY' }]}
    >
      <main style={{ padding: 32 }}>
        <h1>ROOM LOBBY</h1>
      </main>
    </InBetweenShell>
  );
}
