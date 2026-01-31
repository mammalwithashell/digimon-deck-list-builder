import React from 'react';
import Card from './Card';

const GameBoard = ({ state, validActions, onAction }) => {
  if (!state) return <div>Loading...</div>;

  const { player1, player2, current_phase, memory, turn_player_id } = state;
  const isP1Turn = turn_player_id === 1; // Assuming P1 is the user

  // Helper to check if an action is valid
  const getActionId = (type, index) => {
    if (type === 'PLAY') return 0 + index;
    if (type === 'TRASH') return 10 + index;
    if (type === 'ATTACK') return 23 + index;
    return -1;
  };

  const isActionValid = (actionId) => validActions.includes(actionId);

  const handleCardClick = (type, index) => {
    // Determine action based on context
    // If pending_action is TRASH_CARD, clicking hand -> Trash
    // Else clicking hand -> Play

    let actionId = -1;
    if (state.pending_action === 'TRASH_CARD' && type === 'HAND') {
        actionId = getActionId('TRASH', index);
    } else if (type === 'HAND') {
        actionId = getActionId('PLAY', index);
    } else if (type === 'BATTLE') {
        actionId = getActionId('ATTACK', index);
    }

    if (actionId !== -1 && isActionValid(actionId)) {
        onAction(actionId);
    }
  };

  const renderArea = (cards, type, isOpponent) => {
    return (
      <div style={{ display: 'flex', gap: '5px', minHeight: '90px', padding: '5px', border: '1px solid #eee', overflowX: 'auto' }}>
        {cards.map((c, i) => {
             // For battle area, c is permanent (object with top_card)
             const cardData = c.top_card || c;
             // If opponent hand, face down
             const isFacedown = isOpponent && type === 'HAND';

             let actionId = -1;
             if (!isOpponent) {
                 if (state.pending_action === 'TRASH_CARD' && type === 'HAND') actionId = 10 + i;
                 else if (type === 'HAND') actionId = 0 + i;
                 else if (type === 'BATTLE') actionId = 23 + i;
             }

             const selectable = !isOpponent && isActionValid(actionId);

             return (
                 <div key={i} style={{position: 'relative'}}>
                    <Card
                        card={cardData}
                        isFacedown={isFacedown}
                        isSelectable={selectable}
                        onClick={() => !isOpponent && handleCardClick(type, i)}
                    />
                    {c.is_suspended && <div style={{position: 'absolute', top:0, right:0, color:'red', fontWeight:'bold', fontSize:'10px', backgroundColor:'rgba(255,255,255,0.8)'}}>Zzz</div>}
                 </div>
             )
        })}
      </div>
    );
  };

  return (
    <div style={{ padding: '20px', maxWidth: '800px', margin: '0 auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '10px' }}>
        <div style={{fontWeight: 'bold'}}>Phase: {current_phase}</div>
        <div style={{fontWeight: 'bold', color: memory >= 0 ? 'blue' : 'red'}}>Memory: {memory}</div>
        <div style={{fontWeight: 'bold'}}>Turn: Player {turn_player_id}</div>
      </div>

      <h3>Opponent</h3>
      <div style={{ backgroundColor: '#ffeebb', padding: '10px', borderRadius: '8px' }}>
          <div style={{marginBottom: '5px'}}>Security: {player2.security_count} | Deck: {player2.deck_count} | Eggs: {player2.digitama_count} | Trash: {player2.trash_count}</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
            <div style={{ display: 'flex', alignItems: 'center' }}>
                <div style={{width: '60px'}}>Hand:</div>
                {renderArea(player2.hand, 'HAND', true)}
            </div>
            <div style={{ display: 'flex', alignItems: 'center' }}>
                <div style={{width: '60px'}}>Battle:</div>
                {renderArea(player2.battle_area, 'BATTLE', true)}
            </div>
             <div style={{ display: 'flex', alignItems: 'center' }}>
                 <div style={{width: '60px'}}>Breed:</div>
                 <Card card={player2.breeding_area ? player2.breeding_area.top_card : null} isFacedown={false} />
              </div>
          </div>
      </div>

      <hr style={{margin: '20px 0'}} />

      <h3>You (Player 1)</h3>
      <div style={{ backgroundColor: '#ccffee', padding: '10px', borderRadius: '8px' }}>
         <div style={{ display: 'flex', flexDirection: 'column', gap: '5px' }}>
             <div style={{ display: 'flex', alignItems: 'center' }}>
                 <div style={{width: '60px'}}>Breed:</div>
                 <Card card={player1.breeding_area ? player1.breeding_area.top_card : null} />
             </div>
              <div style={{ display: 'flex', alignItems: 'center' }}>
                <div style={{width: '60px'}}>Battle:</div>
                {renderArea(player1.battle_area, 'BATTLE', false)}
              </div>
              <div style={{ display: 'flex', alignItems: 'center' }}>
                <div style={{width: '60px'}}>Hand:</div>
                {renderArea(player1.hand, 'HAND', false)}
              </div>
         </div>
          <div style={{marginTop: '5px'}}>Security: {player1.security_count} | Deck: {player1.deck_count} | Eggs: {player1.digitama_count} | Trash: {player1.trash_count}</div>
      </div>

      <div style={{ marginTop: '20px', display: 'flex', gap: '10px', alignItems: 'center' }}>
        <button disabled={!isActionValid(22)} onClick={() => onAction(22)} style={{padding: '10px'}}>Pass Turn</button>
        <button disabled={!isActionValid(20)} onClick={() => onAction(20)} style={{padding: '10px'}}>Hatch</button>
        <button disabled={!isActionValid(33)} onClick={() => onAction(33)} style={{padding: '10px'}}>Move</button>

        {state.pending_action !== 'NO_ACTION' && <div style={{color: 'red', fontWeight: 'bold'}}>Action Required: {state.pending_action}</div>}
      </div>
    </div>
  );
};

export default GameBoard;
