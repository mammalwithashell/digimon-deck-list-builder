import { useState } from 'react';
import { Card } from '@/components/shared/Card';
import type { TriggerChoice, TriggerPlan } from '@/types/game';

export function TriggerOrderPicker({ choices, waitingTriggers = [], actionMask, validIndices, onSubmit, onInspectCard, onDecline }: {
  choices: TriggerChoice[];
  waitingTriggers?: TriggerChoice[];
  actionMask: number[];
  validIndices: number[];
  onSubmit: (plan: TriggerPlan) => void;
  onInspectCard?: (id: string) => void;
  onDecline?: () => void;
}) {
  const [order, setOrder] = useState<string[]>([]);
  const [answers, setAnswers] = useState<Record<string, boolean>>({});
  const [dragged, setDragged] = useState<string | null>(null);
  const legal = choices.filter((choice) => validIndices.includes(choice.actionId) && actionMask[choice.actionId] === 1);
  const displayed = [...order, ...legal.map((choice) => choice.triggerId).filter((id) => !order.includes(id))];
  const move = (from: string, to: string) => {
    const next = [...displayed];
    const start = next.indexOf(from);
    const end = next.indexOf(to);
    if (start < 0 || end < 0 || start === end) return;
    next.splice(start, 1);
    next.splice(end, 0, from);
    setOrder(next);
  };
  const buttonClass = 'px-3 py-2 rounded-lg border border-[var(--ib-line)] text-sm disabled:opacity-40';

  return (
    <div className="space-y-4 text-[var(--ib-bone)]">
      <p className="text-sm text-[var(--ib-bone-d)]">
        Click effects in resolution order, or drag the numbered handles. New triggers reopen this picker.
      </p>
      <div className="flex flex-wrap gap-2">
        <button className={buttonClass} onClick={() => setOrder(displayed)}>Order all top to bottom</button>
        <button className={buttonClass} onClick={() => { setOrder([]); setAnswers({}); }}>Reset</button>
        {legal.some((choice) => choice.isOptional && choice.activationPresetAvailable !== false) && (
          <>
            <button className={buttonClass} onClick={() => setAnswers(Object.fromEntries(legal.filter((choice) => choice.isOptional && choice.activationPresetAvailable !== false).map((choice) => [choice.triggerId, true])))}>Yes to all optional effects</button>
            <button className={buttonClass} onClick={() => setAnswers(Object.fromEntries(legal.filter((choice) => choice.isOptional && choice.activationPresetAvailable !== false).map((choice) => [choice.triggerId, false])))}>No to all optional effects</button>
          </>
        )}
      </div>
      <ol aria-label="Pending trigger resolution order" className="space-y-3">
        {displayed.map((id, index) => {
          const choice = legal.find((entry) => entry.triggerId === id)!;
          const position = order.indexOf(id);
          return (
            <li key={id} className={`flex gap-3 p-3 rounded-lg border bg-[var(--ib-panel-2)] ${position >= 0 ? 'border-[var(--ib-opp)]' : 'border-[var(--ib-line)]'}`}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => { event.preventDefault(); if (dragged) move(dragged, id); setDragged(null); }}>
              <button className={`${buttonClass} self-start cursor-grab`} aria-label={`Reorder item ${index + 1}`}
                draggable onDragStart={(event) => { event.dataTransfer?.setData('text/plain', id); setDragged(id); }} onDragEnd={() => setDragged(null)}
                onKeyDown={(event) => {
                  if (event.key === 'Escape') setDragged(null);
                  if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return;
                  event.preventDefault();
                  const target = displayed[index + (event.key === 'ArrowUp' ? -1 : 1)];
                  if (target) move(id, target);
                }}>{index + 1} ↕</button>
              <div onContextMenu={(event) => { event.preventDefault(); onInspectCard?.(choice.cardId); }}>
                <Card cardId={choice.cardId} size="sm" />
              </div>
              <div className="flex-1 min-w-0 space-y-2">
                <button className="text-left w-full" aria-pressed={position >= 0}
                  aria-label={`Order ${choice.cardName}: ${choice.effectText}`}
                  onClick={() => setOrder((current) => current.includes(id) ? current.filter((entry) => entry !== id) : [...current, id])}>
                  <span className="block font-semibold">{choice.cardName} <span className="text-xs">{choice.cardId}</span></span>
                  <span className="block text-xs text-[var(--ib-bone-d)]">{choice.timing.replace(/([a-z])([A-Z])/g, '$1 $2')} · {choice.sourceLabel} · {choice.isOptional ? 'Optional' : 'Mandatory'}</span>
                  <span className="block mt-2 text-sm whitespace-pre-wrap">{choice.effectText}</span>
                  {position >= 0 && <span className="block mt-1 text-xs font-bold text-[var(--ib-opp)]">Resolves #{position + 1}</span>}
                </button>
                {choice.isOptional && choice.activationPresetAvailable !== false && (
                  <div role="group" aria-label={`Activate ${choice.cardName}`} className="flex gap-1">
                    {(['Ask', 'Yes', 'No'] as const).map((label) => {
                      const value = label === 'Ask' ? undefined : label === 'Yes';
                      return <button key={label} className={`${buttonClass} ${answers[id] === value ? 'bg-[var(--ib-opp-soft)] border-[var(--ib-opp)] text-[var(--ib-opp)]' : ''}`} aria-pressed={answers[id] === value}
                        onClick={() => setAnswers((current) => {
                          const next = { ...current };
                          if (value === undefined) delete next[id]; else next[id] = value;
                          return next;
                        })}>{label}</button>;
                    })}
                  </div>
                )}
              </div>
            </li>
          );
        })}
      </ol>
      {waitingTriggers.length > 0 && (
        <section aria-label="Other pending effects" className="border-t border-[var(--ib-line)] pt-3 text-[var(--ib-bone-d)]">
          <h3 className="text-sm font-semibold">Other pending effects</h3>
          <p className="text-xs mb-2">Outside this choice. The engine will offer these when eligible.</p>
          <ul className="space-y-2">
            {waitingTriggers.map((choice) => <li key={choice.triggerId} className="text-sm">
              <span className="font-medium">{choice.cardName}</span> · {choice.timing.replace(/([a-z])([A-Z])/g, '$1 $2')}
              <span className="block text-xs">{choice.effectText}</span>
            </li>)}
          </ul>
        </section>
      )}
      <p className="text-xs text-[var(--ib-bone-d)]">Ask / Yes / No controls activation of optional effects. Target choices and later optional steps are still prompted.</p>
      <div className="sticky bottom-0 flex flex-wrap items-center justify-between gap-2 border-t border-[var(--ib-line)] bg-[var(--ib-graphite)] py-3" aria-live="polite">
        <span className="text-xs">{order.length} of {legal.length} ordered{order.length > 0 && order.length < legal.length ? ' · choose the rest later' : ''}</span>
        {onDecline && <button className={buttonClass} onClick={onDecline}>Decline remaining optional effects</button>}
        <button className={`${buttonClass} bg-[var(--ib-panel-2)] font-semibold`} disabled={!legal.length}
          onClick={() => onSubmit({ order: order.length ? order : displayed, offered: legal.map((choice) => choice.triggerId), optionalAnswers: answers, ...(legal[0]?.generation ? { generation: legal[0].generation } : {}) })}>
          {order.length === 1 ? 'Resolve next effect' : 'Resolve in this order'}
        </button>
      </div>
    </div>
  );
}
