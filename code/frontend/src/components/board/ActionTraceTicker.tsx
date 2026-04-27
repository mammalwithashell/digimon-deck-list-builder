import type { ActionTrace } from '@/types/game';

interface ActionTraceTickerProps {
  traces: ActionTrace[];
}

function actorLabel(actor: ActionTrace['actor']) {
  if (actor === 'human') return 'HUMAN';
  if (actor === 'agent_greedy') return 'AGENT GREEDY';
  if (actor === 'agent_trained') return 'AGENT TRAINED';
  return actor.replace(/_/g, ' ').toUpperCase();
}

export function ActionTraceTicker({ traces }: ActionTraceTickerProps) {
  const latest = (traces as unknown as { at(index: number): ActionTrace | undefined }).at(-1);

  if (!latest) {
    return (
      <div className="ib-action-trace ib-action-trace--human" aria-label="No action trace yet">
        <span className="ib-action-trace__actor">TRACE</span>
        <span className="ib-action-trace__label">Awaiting action</span>
      </div>
    );
  }

  const isHuman = latest.actor === 'human';

  return (
    <div className={`ib-action-trace ib-action-trace--${isHuman ? 'human' : 'agent'}`}>
      <span className="ib-action-trace__actor">{actorLabel(latest.actor)}</span>
      <span className="ib-action-trace__label">{latest.decoded.label}</span>
    </div>
  );
}
