"""Pure Dual face/section inventory; no source authentication or game semantics.

The supplied observation remains intact, including top-level type disagreements.
This helper neither repairs those disagreements nor binds canonical/runtime data.
An occurrence inventory can be inspected even when the diagnostic is blocked.
"""
from copy import deepcopy
import hashlib


class DualUnitError(ValueError):
    """Malformed input is distinct from unsupported Dual structure."""


ARTS_REMINDER = ('＜Arts Digivolve＞ (Instead of trashing after use, your cards may '
                 'digivolve into this card without paying the cost.)')
_ROLES = {
    'primary': {'Special Digivolution Condition': 'condition', 'Effect': 'effect'},
    'dual': {'DUAL Effect': 'effect', 'DUAL Rule': 'arts-reminder'},
}


def _sections(value):
    return (isinstance(value, list) and all(isinstance(s, dict)
            and isinstance(s.get('label'), str) and isinstance(s.get('text'), str)
            for s in value))


def derive_dual_occurrences(observation):
    """Account exact face-local and global occurrences without deleting siblings.

    Only a primary Digimon and a second Option face are understood. Unsupported
    labels, Rule families, layouts and kind disagreements remain explicit. No
    returned status implies that the observation was authenticated or approved.
    """
    if not isinstance(observation, dict) or not isinstance(observation.get('extracted'), dict):
        raise DualUnitError('expected observation with extracted fields')
    e = observation['extracted']
    sections, faces = e.get('text_sections'), e.get('faces')
    if (not isinstance(e.get('card_id'), str) or not _sections(sections)
            or not isinstance(faces, list) or any(not isinstance(f, dict) for f in faces)):
        raise DualUnitError('explicit card, face and section inventories required')
    blockers, occurrences, face_inventory = [], [], []
    if e.get('layout') != 'dual':
        blockers.append({'code': 'UNSUPPORTED_LAYOUT'})
    if len(faces) != 2 or [(f.get('face_id'), f.get('kind')) for f in faces] != [('primary', 'digimon'), ('dual', 'option')]:
        blockers.append({'code': 'UNSUPPORTED_FACE_IDENTITIES'})
    if e.get('card_type') != 'Digimon/Option':
        blockers.append({'code': 'TOP_LEVEL_KIND_DISAGREES_WITH_FACES',
                         'actual': deepcopy(e.get('card_type')), 'expected_face_kinds': ['digimon', 'option']})
    if e.get('rule_grants') != {'traits': [], 'forms': [], 'attributes': []}:
        blockers.append({'code': 'UNSUPPORTED_DUAL_RULE_GRANTS'})
    if any(not isinstance(f.get('face_id'), str) or not isinstance(f.get('kind'), str)
           or not _sections(f.get('text_sections')) for f in faces):
        raise DualUnitError('explicit well-formed sections required for every face')
    flattened = [s for f in faces for s in f['text_sections']]
    aligned = flattened == sections
    if not aligned:
        blockers.append({'code': 'GLOBAL_FACE_SECTION_MISMATCH'})
    section_index = 0
    for face_index, face in enumerate(faces):
        fid = face.get('face_id')
        labels = [s['label'] for s in face['text_sections']]
        if (fid == 'primary' and 'Effect' not in labels
                or fid == 'dual' and ('DUAL Effect' not in labels or labels.count('DUAL Rule') != 1)):
            blockers.append({'code': 'MISSING_OR_DUPLICATE_REQUIRED_FACE_CONTENT', 'face_index': face_index})
        zones = {z: {'presence': 'absent-box', 'occurrence_indices': []}
                 for z in ('effect', 'condition', 'arts-reminder', 'inherited', 'security')}
        for local_index, section in enumerate(face['text_sections']):
            label, text = section['label'], section['text']
            role = _ROLES.get(fid, {}).get(label)
            if role is None:
                blockers.append({'code': 'UNSUPPORTED_FACE_SECTION', 'face_index': face_index,
                                 'face_section_index': local_index, 'label': label, 'text': text})
            elif role == 'arts-reminder' and text != ARTS_REMINDER:
                blockers.append({'code': 'UNSUPPORTED_ARTS_REMINDER', 'face_index': face_index,
                                 'face_section_index': local_index, 'text': text})
            # Empty known effect boxes remain explicit. Conditions and rules may
            # not silently vanish under an empty-string rendering.
            if role in ('condition', 'arts-reminder') and not text.strip():
                blockers.append({'code': 'EMPTY_STRUCTURAL_SECTION', 'section_index': section_index})
            index = len(occurrences)
            occurrences.append({'occurrence_index': index, 'section_index': section_index if aligned else None,
                'face_index': face_index, 'face_id': fid, 'face_kind': face.get('kind'),
                'face_section_index': local_index, 'label': label, 'text': text, 'role': role,
                'whole_container': text, 'container_sha256': hashlib.sha256(text.encode('utf8')).hexdigest(),
                'char_start': 0, 'char_end': len(text), 'utf8_byte_start': 0,
                'utf8_byte_end': len(text.encode('utf8'))})
            if role is not None:
                zones[role]['occurrence_indices'].append(index)
                if text or zones[role]['presence'] != 'nonempty':
                    zones[role]['presence'] = 'nonempty' if text else 'printed-empty-box'
            section_index += 1
        face_inventory.append({'face_index': face_index, 'source_face': deepcopy(face), 'zones': zones})
    return {'version': 1, 'kind': 'pure-dual-face-occurrence-diagnostic',
        'status': 'BLOCKED_DUAL_DIAGNOSTIC' if blockers else 'DERIVED_DUAL_DIAGNOSTIC',
        'card_id': e['card_id'], 'source_observation': deepcopy(observation),
        'global_section_inventory': deepcopy(sections), 'face_inventory': face_inventory,
        'occurrences': occurrences, 'blockers': blockers,
        'integration_gate': 'source authenticity, kind disagreement, canonical/runtime/correction binding and behavior not approved'}
