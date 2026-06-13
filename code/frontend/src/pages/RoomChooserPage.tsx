import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { getPlayFormat } from '@/features/play/formatCatalog';
import { joinRoom } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import './RoomChooserPage.css';

const CODE_LENGTH = 5;

export function RoomChooserPage() {
  const navigate = useNavigate();
  const { formatId, setQueue } = usePlayFlowStore();
  const [code, setCode] = useState('');
  const [joining, setJoining] = useState(false);
  const [notice, setNotice] = useState('');
  const format = getPlayFormat(formatId);
  const codeComplete = code.length === CODE_LENGTH;

  const handleJoin = async () => {
    if (!codeComplete || joining) return;
    setJoining(true);
    setNotice('');
    try {
      const result = await joinRoom(code);
      setQueue({ gameId: result.game_id, roomCode: code, seat: 2 });
      navigate(`/play/room/${result.game_id}`);
    } catch (err: unknown) {
      const status = (err as { response?: { status?: number } })?.response?.status;
      if (status === 404) setNotice('No room with that code');
      else if (status === 409) setNotice('Room is already full');
      else setNotice('Unable to join room');
      setJoining(false);
    }
  };

  return (
    <InBetweenShell
      title="ROOM MATCH"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'ROOM MATCH' }]}
      rightSlot={<span>{format.name}</span>}
    >
      <main className="room-choose-main">
        <header className="room-choose-header">
          <div className="room-choose-kicker">// PRIVATE ROOMS - SHARE A CODE WITH A FRIEND</div>
          <h1>
            CREATE A ROOM
            <br />
            <em>OR JOIN ONE.</em>
          </h1>
          {notice && <p className="room-notice">{notice}</p>}
        </header>

        <section className="room-choose-grid">
          <article className="room-choose-card host">
            <span className="tag">// HOST</span>
            <h2>CREATE ROOM</h2>
            <p>Get a 5-digit room code, pick your deck and who goes first, then share the code.</p>
            <button type="button" onClick={() => navigate('/play/room/new')}>
              CREATE ROOM
            </button>
          </article>

          <article className="room-choose-card guest">
            <span className="tag">// GUEST</span>
            <h2>JOIN ROOM</h2>
            <p>Enter the room code your opponent shared.</p>
            <input
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={CODE_LENGTH}
              placeholder="-----"
              aria-label="Room code"
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, '').slice(0, CODE_LENGTH))}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void handleJoin();
              }}
            />
            <button type="button" disabled={!codeComplete || joining} onClick={() => void handleJoin()}>
              {joining ? 'JOINING' : 'JOIN'}
            </button>
          </article>
        </section>

        <div className="room-choose-actions">
          <button type="button" onClick={() => navigate('/play')}>
            BACK TO FORMAT
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
