import { useEffect, useMemo, useState } from 'react';
import { Card } from '@/components/shared/Card';
import { formatEffect } from '@/utils/formatCardText';
import type { EffectChoice, PendingSelection, WaitingTriggerInfo } from '@/types/game';
import {
  triggerChoiceKey,
  useResolutionPlanStore,
  type OptionalPreset,
} from '@/stores/resolutionPlanStore';

export interface TriggerChooserModalProps {
  title: string;
  choices: EffectChoice[];
  actionMask: number[];
  pendingSelection?: PendingSelection;
  waitingTriggers?: WaitingTriggerInfo[];
  canDecline: boolean;
  declineActionId: number;
  onAction: (actionId: number) => void;
  onInspectCard?: (cardId: string) => void;
}

/**
 * Aegis-inspired Simultaneous Trigger Chooser & Client-Side Resolution Plan.
 *
 * Lets the player click/reorder triggered effects into an explicit resolution
 * plan and preset optional `Ask | Yes | No` responses — then dispatches step 1
 * immediately and queues remaining steps safely through `resolutionPlanStore`
 * against our unchanged 1-step-at-a-time Rust engine.
 */
export function TriggerChooserModal({
  title,
  choices,
  actionMask,
  pendingSelection,
  waitingTriggers = [],
  canDecline,
  declineActionId,
  onAction,
  onInspectCard,
}: TriggerChooserModalProps) {
  const startPlan = useResolutionPlanStore((s) => s.startPlan);
  const [peeking, setPeeking] = useState(false);

  const validChoices = useMemo(
    () =>
      choices.filter(
        (c) => c.actionId != null && actionMask[c.actionId] === 1 && !c.countChoice,
      ),
    [choices, actionMask],
  );

  const tierKeys = useMemo(() => validChoices.map(triggerChoiceKey), [validChoices]);
  const tierSignature = tierKeys.join('|');
  const [order, setOrder] = useState<string[]>([]);
  const [presets, setPresets] = useState<Record<string, OptionalPreset>>({});

  // Reset local ordering state whenever a new trigger tier arrives so stale
  // keys from an earlier tier can never stick at index 0 and disable Resolve.
  useEffect(() => {
    setOrder([]);
    setPresets({});
  }, [tierSignature]);

  const liveOrder = useMemo(() => {
    const allowed = new Set(tierKeys);
    return order.filter((k) => allowed.has(k));
  }, [order, tierKeys]);

  const displayedOrder = useMemo(
    () => [...liveOrder, ...tierKeys.filter((k) => !liveOrder.includes(k))],
    [liveOrder, tierKeys],
  );

  const choiceByKey = useMemo(() => {
    const map = new Map<string, EffectChoice>();
    for (const c of validChoices) {
      map.set(triggerChoiceKey(c), c);
    }
    return map;
  }, [validChoices]);

  const optionalKeys = useMemo(
    () => validChoices.filter((c) => c.isOptional).map(triggerChoiceKey),
    [validChoices],
  );

  const toggleOrder = (key: string) => {
    setOrder((curr) => (curr.includes(key) ? curr.filter((k) => k !== key) : [...curr, key]));
  };

  const nudge = (key: string, delta: -1 | 1) => {
    const base = [...displayedOrder];
    const idx = base.indexOf(key);
    const dest = idx + delta;
    if (idx < 0 || dest < 0 || dest >= base.length) return;
    [base[idx], base[dest]] = [base[dest]!, base[idx]!];
    setOrder(base);
  };

  const setPreset = (key: string, value: OptionalPreset | undefined) => {
    setPresets((curr) => {
      const next = { ...curr };
      if (value === undefined) delete next[key];
      else next[key] = value;
      return next;
    });
  };

  const presetAll = (value: OptionalPreset) => {
    setOrder(displayedOrder);
    const next: Record<string, OptionalPreset> = {};
    for (const k of optionalKeys) next[k] = value;
    setPresets(next);
  };

  const submittedOrder = liveOrder.length > 0 ? liveOrder : displayedOrder;
  const firstChoice =
    (submittedOrder[0] ? choiceByKey.get(submittedOrder[0]) : undefined) ?? validChoices[0];

  const handleResolve = () => {
    if (firstChoice?.actionId == null) return;
    if (submittedOrder.length > 1 || Object.keys(presets).length > 0) {
      startPlan({
        orderedKeys: submittedOrder,
        tierKeys,
        presets,
        originSelection: pendingSelection ?? null,
      });
    }
    onAction(firstChoice.actionId);
  };

  return (
    <>
      {peeking && (
        <button
          type="button"
          className="hs-peek-return"
          data-testid="peek-return-button"
          onClick={() => setPeeking(false)}
        >
          Return to Trigger Order
        </button>
      )}
      <div
        data-testid="trigger-chooser-modal"
        className={`hs-modal-backdrop${peeking ? ' is-peeking' : ''}`}
      >
        <div className="hs-modal-sheet" role="dialog" aria-label={title}>
          <div className="hs-modal-sheet__header">
            <div>
              <h2 className="hs-modal-sheet__title">{title}</h2>
              <div className="hs-modal-sheet__subtitle">
                Click effects in the order you want them to resolve, or reorder with arrows.
              </div>
            </div>
            {canDecline && (
              <button
                type="button"
                data-testid="selection-decline"
                onClick={() => onAction(declineActionId)}
                className="ds-btn ds-btn--ghost"
              >
                Decline
              </button>
            )}
          </div>

          <div className="hs-modal-sheet__body">
            {optionalKeys.length > 0 && (
              <div className="hs-trigger-toolbar">
                <span className="text-xs text-[var(--ink-2)]">
                  Preset optional effects before resolving:
                </span>
                <div className="flex gap-2">
                  <button
                    type="button"
                    className="ds-btn"
                    onClick={() => presetAll('yes')}
                  >
                    Yes all
                  </button>
                  <button
                    type="button"
                    className="ds-btn"
                    onClick={() => presetAll('no')}
                  >
                    No all
                  </button>
                  <button
                    type="button"
                    className="ds-btn ds-btn--ghost"
                    disabled={order.length === 0 && Object.keys(presets).length === 0}
                    onClick={() => {
                      setOrder([]);
                      setPresets({});
                    }}
                  >
                    Reset
                  </button>
                </div>
              </div>
            )}

            <div className="hs-trigger-list" role="list">
              {displayedOrder.map((key, displayIdx) => {
                const entry = choiceByKey.get(key);
                if (!entry) return null;
                const explicitPos = order.indexOf(key);
                const isExplicit = explicitPos >= 0;

                return (
                  <div
                    key={key}
                    role="listitem"
                    className={`hs-trigger-row${isExplicit ? ' is-ordered' : ''}`}
                  >
                    <div className="hs-trigger-row__order">
                      <button
                        type="button"
                        className="hs-order-nudge"
                        disabled={displayIdx === 0}
                        onClick={() => nudge(key, -1)}
                        aria-label="Move up"
                      >
                        ▲
                      </button>
                      <span className="hs-order-badge">
                        {isExplicit ? explicitPos + 1 : displayIdx + 1}
                      </span>
                      <button
                        type="button"
                        className="hs-order-nudge"
                        disabled={displayIdx === displayedOrder.length - 1}
                        onClick={() => nudge(key, 1)}
                        aria-label="Move down"
                      >
                        ▼
                      </button>
                    </div>

                    <button
                      type="button"
                      className="hs-trigger-row__main"
                      onClick={() => toggleOrder(key)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        if (entry.cardId) onInspectCard?.(entry.cardId);
                      }}
                    >
                      <div className="hs-trigger-row__head">
                        {entry.cardId && <Card cardId={entry.cardId} size="sm" />}
                        <div className="min-w-0">
                          <div className="hs-trigger-row__name" title={entry.cardId}>
                            {entry.cardName && entry.cardId && entry.cardName !== entry.cardId
                              ? `${entry.cardName} (${entry.cardId})`
                              : entry.cardName || entry.cardId}
                          </div>
                          {entry.timing && (
                            <span className="hs-trigger-row__timing">[{entry.timing}]</span>
                          )}
                        </div>
                      </div>
                      <div className="hs-trigger-row__clause card-effect-text">
                        {formatEffect(entry.label)}
                      </div>
                    </button>

                    {entry.isOptional ? (
                      <div
                        className="hs-preset-group"
                        role="group"
                        aria-label={`Optional preset for ${entry.cardName || entry.cardId}`}
                      >
                        {(
                          [
                            { val: undefined, text: 'Ask' },
                            { val: 'yes', text: 'Yes' },
                            { val: 'no', text: 'No' },
                          ] as const
                        ).map((opt) => (
                          <button
                            key={opt.text}
                            type="button"
                            className="hs-preset-btn"
                            aria-pressed={presets[key] === opt.val}
                            onClick={() => setPreset(key, opt.val)}
                          >
                            {opt.text}
                          </button>
                        ))}
                      </div>
                    ) : entry.isOptional === false ? (
                      <span className="hs-mandatory-tag">Mandatory</span>
                    ) : null}
                  </div>
                );
              })}
            </div>

            {waitingTriggers.length > 0 && (
              <details className="hs-waiting-triggers" open>
                <summary>
                  Waiting Effects ({waitingTriggers.length}) — resolve after current tier
                </summary>
                <div className="hs-waiting-triggers__list">
                  {waitingTriggers.map((wt, i) => (
                    <span
                      key={`${wt.cardId}-${i}`}
                      className="hs-waiting-triggers__chip"
                      title={wt.cardId}
                    >
                      <strong>
                        {wt.cardName && wt.cardId && wt.cardName !== wt.cardId
                          ? `${wt.cardName} (${wt.cardId})`
                          : wt.cardName || wt.cardId}
                      </strong>
                      {wt.timing && <span>[{wt.timing}]</span>}
                    </span>
                  ))}
                </div>
              </details>
            )}
          </div>

          <div className="hs-modal-sheet__footer">
            <button
              type="button"
              className="ds-btn ds-btn--ghost"
              onClick={() => setPeeking(true)}
            >
              View Board
            </button>
            <button
              type="button"
              data-testid="trigger-resolve-confirm"
              disabled={firstChoice?.actionId == null}
              onClick={handleResolve}
              className="ds-btn ds-btn--primary"
            >
              {submittedOrder.length > 1 ? 'Resolve in Order' : 'Resolve Effect'}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
