import React from 'react';

const Card = ({ card, onClick, isSelectable, isFacedown, style }) => {
  const handleClick = () => {
    if (isSelectable && onClick) {
      onClick();
    }
  };

  const baseStyle = {
    width: '60px',
    height: '84px',
    border: '1px solid #333',
    borderRadius: '4px',
    backgroundColor: 'white',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: '8px',
    padding: '2px',
    cursor: isSelectable ? 'pointer' : 'default',
    boxShadow: isSelectable ? '0 0 5px blue' : 'none',
    overflow: 'hidden',
    position: 'relative',
    ...style
  };

  if (isFacedown) {
    return (
      <div style={{ ...baseStyle, backgroundColor: '#444', color: 'white' }}>
        Back
      </div>
    );
  }

  if (!card) return <div style={baseStyle}>Empty</div>;

  const colorMap = {
    'Red': '#ffcccc',
    'Blue': '#ccccff',
    'Yellow': '#ffffcc',
    'Green': '#ccffcc',
    'Black': '#cccccc',
    'Purple': '#eeccee',
    'White': '#ffffff'
  };

  const bgColor = card.colors && card.colors.length > 0 ? colorMap[card.colors[0]] || 'white' : 'white';

  return (
    <div style={{ ...baseStyle, backgroundColor: bgColor }} onClick={handleClick}>
      <div style={{fontWeight: 'bold', textAlign: 'center', fontSize: '9px', marginBottom: '2px'}}>{card.name}</div>
      <div style={{fontSize: '7px'}}>DP: {card.dp}</div>
      <div style={{fontSize: '7px'}}>Cost: {card.cost}</div>
      <div style={{fontSize: '7px'}}>Lvl: {card.level}</div>
      {/* Show traits/effects text briefly? */}
    </div>
  );
};

export default Card;
