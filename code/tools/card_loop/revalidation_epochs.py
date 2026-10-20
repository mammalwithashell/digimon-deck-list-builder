"""Explicit, append-only execution epochs inside one immutable source generation.

The legacy flat queue remains epoch0 byte-for-byte. Only a valid activation
journal selects a successor; orphan definitions can never dispatch themselves.
"""
from __future__ import annotations

import copy
import json
import os
import re
import uuid
from pathlib import Path

from . import revalidation as r
from .stages.base import StageDeferred


def _sealed(value):
    if not isinstance(value, dict):
        return False
    body = {k:v for k,v in value.items() if k != 'digest'}
    return value.get('digest') == r._digest(body)


def _successor(previous, definition):
    expected_fields = {'version', 'run_id', 'generation', 'reviewer', 'review_evidence',
                       'runtime', 'entries', 'preserved_claims', 'reviewed_deck_candidates',
                       'reviewed_deck_books', 'predecessor', 'digest'}
    if (not isinstance(definition, dict) or set(definition) != expected_fields or
            type(definition.get('version')) is not int or definition['version'] != 1):
        raise StageDeferred('unsupported execution epoch schema')
    if not _sealed(definition) or definition.get('predecessor') != {
        'epoch': previous['definition']['digest'], 'tip': previous['tip'],
        'activation_tip': previous['activation_tip'],
    }:
        raise StageDeferred('reviewed predecessor identity/tip changed')
    old = previous['definition']
    if definition.get('generation') != old['generation'] or definition.get('run_id') != old['run_id']:
        raise StageDeferred('execution transition cannot change source generation or accounting run')
    if not isinstance(definition['entries'], dict) or set(definition['entries']) != set(old['entries']):
        raise StageDeferred('execution transition must preserve every queue item')
    for item, entry in definition['entries'].items():
        if (not isinstance(entry, dict) or
                set(entry) != {'item', 'previous_state', 'previous_attempts', 'previous_data', 'scenarios'} or
                not isinstance(entry['scenarios'], dict) or not entry['scenarios'] or
                any(not isinstance(name, str) or not name or not isinstance(pin, str) or
                    not re.fullmatch('[0-9a-f]{64}', pin) for name, pin in entry['scenarios'].items())):
            raise StageDeferred('invalid execution epoch entry schema')
        for key in ('item','previous_state','previous_attempts','previous_data'):
            if entry.get(key) != old['entries'][item].get(key):
                raise StageDeferred('execution transition cannot rewrite historical questions or accounting')
    if any(not isinstance(definition[key], str) or not definition[key].strip()
           for key in ('reviewer', 'review_evidence')):
        raise StageDeferred('independent transition reviewer and evidence required')
    candidates, pins = definition['reviewed_deck_candidates'], definition['reviewed_deck_books']
    if (not isinstance(candidates, dict) or not isinstance(pins, dict) or
            any(not isinstance(name, str) or not isinstance(books, list) or
                any(not isinstance(book, str) for book in books) for name, books in candidates.items()) or
            any(not isinstance(name, str) or not isinstance(pin, str) or
                not re.fullmatch('[0-9a-f]{64}', pin) for name, pin in pins.items()) or
            not isinstance(definition['runtime'], dict) or not isinstance(definition['preserved_claims'], dict)):
        raise StageDeferred('invalid execution epoch input schema')
    if any(row['status'] == 'STARTED' for row in previous['items'].values()):
        raise StageDeferred('unresolved STARTED attempt requires interruption review')


def read_active(ctx):
    """Read/validate the full lineage, with no filesystem writes."""
    root = r.directory(ctx)
    active = r._read_queue_at(ctx, root)
    journal = root/'transitions.jsonl'
    if active is None:
        if journal.exists():
            raise StageDeferred('activation journal has no preserved flat epoch0')
        return None
    active['activation_tip'] = active['definition']['digest']
    lineage = [active]
    transitions = []
    try:
        for line in journal.read_text(encoding='utf-8').splitlines() if journal.exists() else []:
            event = json.loads(line)
            if (not _sealed(event) or event['sequence'] != len(transitions)+1 or
                    event['previous'] != active['activation_tip'] or
                    not re.fullmatch('[0-9a-f]{64}', event['epoch'])):
                raise StageDeferred('invalid execution activation chain')
            successor = r._read_queue_at(ctx, root/'epochs'/event['epoch'])
            if successor is None or successor['definition']['digest'] != event['epoch']:
                raise StageDeferred('activated epoch definition unavailable or changed')
            _successor(active, successor['definition'])
            if event['predecessor'] != successor['definition']['predecessor']:
                raise StageDeferred('activation predecessor binding changed')
            successor['activation_tip'] = event['digest']
            lineage.append(successor)
            transitions.append(event)
            active = successor
    except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
        raise StageDeferred(f'invalid execution activation history: {error}') from error
    cumulative = {}
    for epoch in lineage:
        for item, row in epoch['items'].items():
            counts = cumulative.setdefault(item, {})
            for stage, count in row['attempts'].items():
                counts[stage] = counts.get(stage, 0)+count
    # Do not create recursive lineage objects; immutable summaries retain every
    # predecessor's event tip and attempts without copying historical raw rows.
    active['lineage'] = [{'epoch': q['definition']['digest'], 'tip': q['tip'],
                          'directory': q['directory']} for q in lineage]
    active['transitions'] = transitions
    active['cumulative_attempts'] = cumulative
    unresolved = {}
    for epoch in lineage:
        for event in epoch['events']:
            update = event.get('update', {})
            if update.get('outcome') == 'ESCALATED' or update.get('escalation'):
                identity = (epoch['definition']['digest'], event['digest'])
                unresolved[identity] = {'epoch': identity[0], 'event': identity[1],
                                        'item': event['item'], 'reason': update.get('reason'),
                                        'data': copy.deepcopy(update.get('data', {})),
                                        'escalation': copy.deepcopy(update.get('escalation'))}
            if event['kind'] == 'escalation_resolution':
                target = event.get('question', {})
                identity = (target.get('epoch'), target.get('event'))
                if (identity not in unresolved or event.get('expected_tip') != event['previous'] or
                        target != unresolved[identity] or event['item'] != target.get('item') or
                        any(not isinstance(event.get(k), str) or not event[k].strip()
                            for k in ('reviewer', 'evidence')) or update):
                    raise StageDeferred('invalid exact escalation resolution review')
                del unresolved[identity]
    active['unresolved_escalations'] = list(unresolved.values())
    return active


def resolve_escalation(ctx, question, *, expected_tip, reviewer, evidence):
    """Explicit append-only resolution; measurement and activation never resolve questions."""
    if any(not isinstance(value, str) or not value.strip() for value in (reviewer, evidence)):
        raise StageDeferred('explicit escalation reviewer and decision evidence required')
    with r.locked(ctx):
        r._manifest(ctx)
        queue = read_active(ctx)
        if queue is None or expected_tip != queue['tip'] or question not in queue['unresolved_escalations']:
            raise StageDeferred('reviewed escalation identity or active tip changed')
        r._append(ctx, question['item'], 'escalation_resolution', {}, question=question,
                  expected_tip=expected_tip, reviewer=reviewer, evidence=evidence)


def _deck_inputs(ctx, entries):
    from .stages import harness
    candidates, pins = {}, {}
    for entry in entries.values():
        for scenario in entry['scenarios']:
            books = list(harness.deck_books_for(ctx.repo, scenario))
            candidates[scenario] = books
            for book in books:
                if book:
                    pins[book] = r._file_hash(Path(ctx.repo)/book)
    return candidates, pins


def validate_epoch_inputs(ctx, definition):
    """Old flat definitions are unchanged; new epochs bind reviewed input sets."""
    if 'predecessor' not in definition:
        return
    for item, entry in definition['entries'].items():
        for name, digest in entry['scenarios'].items():
            r._scenario(ctx, item, name, digest)
    candidates, pins = _deck_inputs(ctx, definition['entries'])
    if (candidates != definition.get('reviewed_deck_candidates') or
            pins != definition.get('reviewed_deck_books')):
        raise StageDeferred('reviewed execution epoch deck inputs changed')


def prepare_transition(ctx, selected, *, reviewer, evidence, preserved_claims=None):
    """Read-only proposal for exact independent review; does not activate."""
    if not reviewer or not evidence:
        raise StageDeferred('independent transition reviewer and evidence required')
    r._manifest(ctx)
    old = read_active(ctx)
    if old is None or set(selected) != set(old['definition']['entries']):
        raise StageDeferred('transition selection must preserve the entire predecessor queue')
    entries = copy.deepcopy(old['definition']['entries'])
    for item, scenarios in selected.items():
        if not scenarios:
            raise StageDeferred('each epoch item requires a reviewed scenario')
        entries[item]['scenarios'] = {name:r._scenario(ctx,item,name) for name in scenarios}
    runtime = r._runtime(ctx)
    if runtime['harness_root'] != old['definition']['runtime']['harness_root']:
        raise StageDeferred('execution transition cannot relocate shared oracle root')
    claims = old['definition']['preserved_claims'] if preserved_claims is None else preserved_claims
    candidates, pins = _deck_inputs(ctx, entries)
    definition = dict(version=1,run_id=ctx.run_id,generation=ctx.config.source_manifest_sha256,
                      reviewer=reviewer,review_evidence=evidence,runtime=runtime,entries=entries,
                      preserved_claims=r._claim_inventory(ctx,claims),
                      reviewed_deck_candidates=candidates,reviewed_deck_books=pins,
                      predecessor={'epoch':old['definition']['digest'],'tip':old['tip'],
                                   'activation_tip':old['activation_tip']})
    definition['digest'] = r._digest(definition)
    return definition


def _write_definition(path, definition):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        if json.loads(path.read_text(encoding='utf-8')) != definition:
            raise StageDeferred('immutable execution epoch already exists with different bytes')
        return
    temporary = path.with_name('queue.pending-'+uuid.uuid4().hex)
    with temporary.open('x',encoding='utf-8') as out:
        out.write(json.dumps(definition,ensure_ascii=False,indent=2)+'\n')
        out.flush()
        os.fsync(out.fileno())
    # Common source-generation OS lock excludes all cooperative writers.
    temporary.rename(path)


def _publish_activation(ctx, old, definition):
    event = dict(sequence=len(old['transitions'])+1,previous=old['activation_tip'],
                 epoch=definition['digest'],predecessor=definition['predecessor'],ts=ctx.now())
    event['digest'] = r._digest(event)
    with (r.directory(ctx)/'transitions.jsonl').open('a',encoding='utf-8') as out:
        out.write(json.dumps(event,ensure_ascii=False)+'\n')
        out.flush()
        os.fsync(out.fileno())


def activate_transition(ctx, definition):
    """Activate the exact reviewed proposal; never removes STOP or starts work."""
    with r.locked(ctx):
        r._manifest(ctx)
        old = read_active(ctx)
        if not isinstance(definition,dict) or not _sealed(definition):
            raise StageDeferred('execution epoch proposal digest changed')
        if old is None:
            raise StageDeferred('execution transition requires preserved epoch0')
        if old['definition'] == definition:
            return old  # receipt retry, no new write or dispatch
        _successor(old, definition)
        if not (Path(ctx.run_dir)/'STOP').exists():
            raise StageDeferred('STOP is required for reviewed execution transition')
        if definition['runtime'] != r._runtime(ctx):
            raise StageDeferred('reviewed execution runtime changed')
        if definition['runtime']['harness_root'] != old['definition']['runtime']['harness_root']:
            raise StageDeferred('execution transition cannot relocate shared oracle root')
        validate_epoch_inputs(ctx,definition)
        with r._file_lock(Path(ctx.config.harness_root)/'revalidation-dispatch.lock'):
            r.require_oracle_idle(r._idle_context(ctx,{'definition':definition}))
            path = r.directory(ctx)/'epochs'/definition['digest']/'queue.json'
            _write_definition(path,definition)
            _publish_activation(ctx,old,definition)
        return read_active(ctx)
