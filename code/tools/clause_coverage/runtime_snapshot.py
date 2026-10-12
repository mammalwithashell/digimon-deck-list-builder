"""Read an externally pinned full snapshot once; never confer source admission.

The external artifact hash belongs to reviewed configuration, not to arbitrary
caller input. A handle retains immutable original bytes, not a forever cache of
live paths. Dispatch must still compare its configured inputs to this handle.
"""
from pathlib import Path, PurePosixPath, PureWindowsPath
from types import MappingProxyType
import re

from .runtime_projection_proof import _json, _sha, _require, enc

ROLES=frozenset(('canonical','runtime','registry','aggregate','official_scope',
                 'secondary_scope','corrections','selections','adjudications'))
_TOKEN=object()


def _digest(value):
    return isinstance(value,str) and re.fullmatch('[0-9a-f]{64}',value) is not None


class VerifiedRuntimeSnapshot:
    """Immutable in-memory record extraction; only the loader constructs it."""
    __slots__=('_files','_canonical','_runtime','_artifact','_artifact_sha')

    def __init__(self,token=None,*,files=None,canonical=None,runtime=None,artifact=None):
        if token is not _TOKEN:raise TypeError('use load_runtime_snapshot')
        object.__setattr__(self,'_files',MappingProxyType(dict(files)))
        object.__setattr__(self,'_canonical',MappingProxyType(dict(canonical)))
        object.__setattr__(self,'_runtime',MappingProxyType(dict(runtime)))
        object.__setattr__(self,'_artifact',artifact)
        object.__setattr__(self,'_artifact_sha',_sha(artifact))

    def __setattr__(self,name,value):raise TypeError('snapshot is immutable')

    def card(self,card_id):
        _require(card_id in self._canonical and card_id in self._runtime,'snapshot card absent')
        return {'canonical_card':self._canonical[card_id],'runtime_record':self._runtime[card_id],
                'registry_bytes':self._files['registry']}

    def file(self,role):return self._files[role]

    def metadata(self):
        artifact=_json(self._artifact)
        return {**artifact,'extraction_artifact_sha256':self._artifact_sha,
                'limit':'Exact extraction only; source authority, unclaimed metadata and admission not approved'}


def load_runtime_snapshot(*,candidate_root,extraction_artifact,expected_extraction_sha256):
    """Verify every full file and complete record map once before retaining bytes."""
    _require(_digest(expected_extraction_sha256) and isinstance(extraction_artifact,bytes)
             and _sha(extraction_artifact)==expected_extraction_sha256,'extraction artifact pin mismatch')
    d=_json(extraction_artifact)
    _require(isinstance(d,dict) and set(d)=={'schema_version','kind','source_revision','files',
        'official_card_ids','canonical_record_sha256','runtime_record_sha256'},'extraction schema fields')
    _require(type(d['schema_version'])is int and d['schema_version']==1
        and d['kind']=='exact-runtime-snapshot-extraction-v1' and _digest(d['source_revision']),
        'unsupported extraction schema/revision')
    _require(isinstance(d['files'],dict) and set(d['files'])==ROLES,'exact file roles required')
    root=Path(candidate_root).resolve(strict=True);files={};documents={};seen=set()
    for role,entry in d['files'].items():
        _require(isinstance(entry,dict) and set(entry)=={'path','sha256','bytes'},'file descriptor fields')
        path=entry['path']
        _require(isinstance(path,str) and bool(path) and '\\' not in path and ':' not in path
            and not PurePosixPath(path).is_absolute() and not PureWindowsPath(path).drive
            and all(p not in ('','..','.')for p in path.split('/')),'unsafe candidate-relative path')
        target=(root/path).resolve(strict=True)
        _require(target.is_relative_to(root) and target not in seen and
                 not any(target.samefile(prior)for prior in seen),'escaped or duplicate file path')
        seen.add(target)
        _require(_digest(entry['sha256']) and type(entry['bytes'])is int and entry['bytes']>=0,
                 'invalid file digest/size')
        raw=target.read_bytes()
        _require(len(raw)==entry['bytes'] and _sha(raw)==entry['sha256'],'candidate full file bytes differ')
        files[role]=raw
        documents[role]=_json(raw)
        _require(isinstance(documents[role],dict),'candidate file must be a JSON object')
    canonical_doc=documents['canonical'];runtime_doc=documents['runtime']
    _require(isinstance(canonical_doc,dict) and isinstance(canonical_doc.get('cards'),dict)
             and isinstance(runtime_doc,dict),'unsupported candidate container shapes')
    records={}
    for role,document in (('canonical',canonical_doc['cards']),('runtime',runtime_doc)):
        wanted=d[role+'_record_sha256']
        _require(isinstance(wanted,dict) and set(wanted)==set(document),'complete record membership mismatch')
        records[role]={}
        for cid,record in document.items():
            _require(isinstance(record,dict) and record.get('card_id')==cid and _digest(wanted[cid]),
                     'record identity/digest mismatch')
            raw=enc(record)
            _require(_sha(raw)==wanted[cid],'record extraction differs from full container')
            records[role][cid]=raw
    aggregate=documents['aggregate'];official=d['official_card_ids']
    _require(isinstance(official,list) and all(isinstance(cid,str)for cid in official)
        and official==sorted(set(official)),'complete sorted official universe required')
    _require(aggregate['manifest_revision']==d['source_revision']
        and set(official)==set(aggregate['sources']['official']['gameplay_card_ids']),
        'aggregate source revision/universe mismatch')
    _require(set(official)<=set(records['canonical']) and set(official)<=set(records['runtime']),
             'official records omitted from container')
    return VerifiedRuntimeSnapshot(_TOKEN,files=files,canonical=records['canonical'],
                                   runtime=records['runtime'],artifact=extraction_artifact)
