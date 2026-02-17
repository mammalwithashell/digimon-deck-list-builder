import { Link } from 'react-router-dom';

const CARDS = [
  { to: '/game', title: 'Play', desc: 'Start a game against an AI agent', enabled: true },
  { to: '/deckbuilder', title: 'Deck Builder', desc: 'Build and manage your decks', enabled: true },
  { to: '/replays', title: 'Replays', desc: 'Watch game recordings', enabled: false },
];

export function HomePage() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[calc(100vh-56px)] p-8">
      <h1 className="text-4xl font-bold text-gray-100 mb-2">Digimon TCG Simulator</h1>
      <p className="text-gray-400 mb-10">Play, build decks, and train AI agents</p>
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-3xl w-full">
        {CARDS.map((card) => (
          <Link
            key={card.to}
            to={card.enabled ? card.to : '#'}
            className={`block p-6 rounded-lg border transition-all ${
              card.enabled
                ? 'bg-gray-800 border-gray-700 hover:border-blue-500 hover:bg-gray-750 cursor-pointer'
                : 'bg-gray-800/50 border-gray-700/50 cursor-not-allowed opacity-50'
            }`}
            onClick={(e) => { if (!card.enabled) e.preventDefault(); }}
          >
            <h2 className="text-xl font-semibold text-gray-100 mb-2">{card.title}</h2>
            <p className="text-sm text-gray-400">{card.desc}</p>
            {!card.enabled && (
              <span className="text-xs text-gray-500 mt-2 block">Coming soon</span>
            )}
          </Link>
        ))}
      </div>
    </div>
  );
}
