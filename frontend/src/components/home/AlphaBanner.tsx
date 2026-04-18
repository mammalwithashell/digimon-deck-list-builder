import { Link } from 'react-router-dom';

export function AlphaBanner() {
  return (
    <div className="mb-6 rounded border border-amber-600 bg-amber-900/20 p-3 text-sm text-amber-100">
      <span className="mr-2 rounded bg-amber-600 px-1.5 py-0.5 text-xs font-bold uppercase text-amber-950">
        Alpha
      </span>
      Card-effect coverage is incomplete. See{' '}
      <Link to="/patch-notes" className="underline hover:text-amber-200">
        patch notes
      </Link>{' '}
      for what works today.
    </div>
  );
}
