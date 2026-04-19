import { Link } from 'react-router-dom';
import { AlphaBanner } from '@/components/home/AlphaBanner';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

interface HomeCard {
  to: string;
  title: string;
  desc: string;
  primary?: boolean;
}

const CARDS: HomeCard[] = [
  { to: '/lobby', title: 'Find Match', desc: 'Queue up for casual or ranked play', primary: true },
  { to: '/models', title: 'Play vs AI', desc: 'Try or download AI opponents', primary: true },
  { to: '/deckbuilder', title: 'Deck Builder', desc: 'Build and manage your decks' },
  { to: '/patch-notes', title: 'Patch Notes', desc: 'What is new, what is known-broken' },
];

export function HomePage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-10">
      <AlphaBanner />
      <h1 className="mb-2 text-4xl font-bold text-gray-100">Digimon TCG Simulator</h1>
      <p className="mb-8 text-gray-400">
        {IS_DESKTOP
          ? 'Play, build decks, and face AI opponents. Multiplayer is online.'
          : 'Play, build decks, and train AI agents.'}
      </p>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        {CARDS.map((c) => (
          <Link
            key={c.to}
            to={c.to}
            className={`block rounded-lg border p-6 transition-all ${
              c.primary
                ? 'border-blue-600 bg-blue-900/20 hover:border-blue-500 hover:bg-blue-900/30'
                : 'border-gray-700 bg-gray-800 hover:border-gray-600 hover:bg-gray-750'
            }`}
          >
            <h2 className="mb-2 text-xl font-semibold text-gray-100">{c.title}</h2>
            <p className="text-sm text-gray-400">{c.desc}</p>
          </Link>
        ))}
      </div>
    </div>
  );
}
