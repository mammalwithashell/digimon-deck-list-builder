import { useState } from 'react';
import { GameLog } from './GameLog';

interface GameLogDrawerProps {
  logs: string[];
}

/**
 * Slide-out game log panel toggled by a button.
 * Replaces the old 240px sidebar.
 */
export function GameLogDrawer({ logs }: GameLogDrawerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [lastSeenCount, setLastSeenCount] = useState(0);

  const unreadCount = logs.length - lastSeenCount;

  const handleOpen = () => {
    setIsOpen(true);
    setLastSeenCount(logs.length);
  };

  const handleClose = () => {
    setIsOpen(false);
    setLastSeenCount(logs.length);
  };

  return (
    <>
      {/* Toggle button (always visible, top-right area) */}
      <button
        onClick={isOpen ? handleClose : handleOpen}
        className="absolute top-2 right-2 z-20 flex items-center gap-1.5 px-2.5 py-1.5 bg-[var(--ib-graphite)] border border-[var(--ib-line)] rounded text-xs text-[var(--ib-bone-d)] hover:text-[var(--ib-bone)] hover:bg-[var(--ib-panel-2)] transition-colors"
      >
        <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
        </svg>
        Log
        {unreadCount > 0 && (
          <span className="ml-1 px-1.5 py-0.5 bg-blue-600 text-white text-[10px] font-bold rounded-full min-w-[18px] text-center">
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {/* Drawer panel */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div
            className="absolute inset-0 z-30 bg-black/30"
            onClick={handleClose}
          />
          {/* Panel */}
          <div className="absolute top-0 right-0 z-40 w-[300px] h-full bg-[var(--ib-graphite)] border-l border-[var(--ib-line)] backdrop-blur-sm shadow-2xl flex flex-col">
            <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--ib-line)]">
              <span className="text-sm font-medium text-[var(--ib-bone)]">Game Log</span>
              <button
                onClick={handleClose}
                className="text-[var(--ib-bone-dd)] hover:text-[var(--ib-bone-d)] text-lg leading-none"
              >
                &times;
              </button>
            </div>
            <div className="flex-1 overflow-hidden">
              <GameLog logs={logs} />
            </div>
          </div>
        </>
      )}
    </>
  );
}
