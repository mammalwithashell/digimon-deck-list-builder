import React, { useState, useEffect } from 'react';
import GameBoard from './components/GameBoard';

function App() {
  const [sessionId, setSessionId] = useState(null);
  const [gameState, setGameState] = useState(null);
  const [validActions, setValidActions] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const startGame = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch('/game/start', { method: 'POST' });
      if (!res.ok) throw new Error('Failed to start game');
      const data = await res.json();
      setSessionId(data.session_id);
      await fetchState(data.session_id);
    } catch (e) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  const fetchState = async (sid) => {
    try {
        const res = await fetch(`/game/${sid}`);
        const state = await res.json();
        setGameState(state);

        const res2 = await fetch(`/game/${sid}/actions`);
        const actions = await res2.json();
        setValidActions(actions.valid_actions);
    } catch (e) {
        console.error(e);
    }
  };

  const performAction = async (actionId) => {
      if (!sessionId) return;
      try {
          const res = await fetch(`/game/${sessionId}/action`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ action: actionId })
          });
          const data = await res.json();
          setGameState(data.state);

          // Refresh actions
          const res2 = await fetch(`/game/${sessionId}/actions`);
          const actions = await res2.json();
          setValidActions(actions.valid_actions);
      } catch (e) {
          setError(e.message);
      }
  };

  if (!sessionId) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh', flexDirection: 'column', gap: '20px' }}>
        <h1>Digimon TCG Simulator</h1>
        <button onClick={startGame} style={{ padding: '20px', fontSize: '20px', cursor: 'pointer' }}>Start New Game</button>
        {loading && <div>Starting...</div>}
        {error && <div style={{color: 'red'}}>{error}</div>}
      </div>
    );
  }

  return (
    <div>
      {loading && <div>Loading...</div>}
      {error && <div style={{color: 'red', padding: '10px'}}>Error: {error}</div>}
      <GameBoard state={gameState} validActions={validActions} onAction={performAction} />
    </div>
  );
}

export default App;
