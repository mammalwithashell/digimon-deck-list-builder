"""Pure conservative Link segmentation, with no source or runtime authority.

The caller must separately authenticate complete observations and bind canonical,
runtime and DSL fields. This diagnostic never removes ordinary source text.
"""
from copy import deepcopy
import re

_REMINDER = '(Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)'
_CONDITION = re.compile(r'\uff1cLink\uff1e \[Appmon\] trait: Cost (0|[1-9][0-9]*) ' + re.escape(_REMINDER))
_LINK = re.compile(r'link', re.I)
_EFFECT = re.compile(r'\[When Linking\] [^\r\n]+')


def derive_link_occurrences(observation):
    """Retain every original section; derive only the two observed Link layouts."""
    raw=deepcopy(observation)
    extracted=raw.get('extracted') if isinstance(raw,dict) else None
    sections=extracted.get('text_sections') if isinstance(extracted,dict) else None
    result={'version':1,'kind':'link-source-occurrence-diagnostic',
            'status':'BLOCKED','source_authority':False,'training_source_complete':False,
            'observation':raw,'section_inventory':deepcopy(sections),
            'occurrences':[],'blockers':[]}
    def block(reason):result['blockers'].append({'code':'UNSUPPORTED_LINK_SOURCE','reason':reason})
    if not isinstance(raw,dict) or raw.get('source')!='official' or not isinstance(sections,list):
        block('official section array required');return result
    if any(not isinstance(s,dict) or set(s)!={'label','text'} or
           not isinstance(s['label'],str) or not isinstance(s['text'],str) for s in sections):
        block('exact string section envelope required');return result
    relevant=[i for i,s in enumerate(sections) if _LINK.search(s['label']) or _LINK.search(s['text'])]
    if not relevant:
        result['status']='NOT_APPLICABLE';return result
    explicit=any(s['label'] in ('Link Condition','Link Effect','Link DP') for s in sections)
    combined=any(s['label']=='Inherited Effect' and _LINK.search(s['text']) for s in sections)
    if explicit and combined:block('mixed Link layouts')
    dp=[];conditions=[];effects=[]
    def occurrence(i,role,start,end,value):
        text=sections[i]['text']
        result['occurrences'].append({'section_index':i,'role':role,'purpose':'link-source-derivation-only',
            'executable':False,'char_start':start,'char_end':end,
            'utf8_start':len(text[:start].encode('utf-8')),'utf8_end':len(text[:end].encode('utf-8')),
            'text':text[start:end],'value':value})
    def condition(i,start,end):
        text=sections[i]['text'][start:end];m=_CONDITION.fullmatch(text)
        if not m:block('unsupported whole Link condition');return
        conditions.append(i);occurrence(i,'condition',start,end,{'trait':'Appmon','cost':int(m[1])})
    def effect(i,start,end):
        text=sections[i]['text'][start:end]
        # Opaque effect body is preserved. A second timing or Link route is not
        # silently accepted as part of this one observed When Linking span.
        body=text[len('[When Linking] '):]
        # The only angle keyword in this initial observed family is Draw.
        # Unknown angle text must not smuggle another timing into the span.
        residual=re.sub(r'\uff1cDraw (0|[1-9][0-9]*)\uff1e','',body)
        if (not _EFFECT.fullmatch(text) or not body.strip() or '[' in body or _LINK.search(body)
                or any(c in residual for c in '<>\uff1c\uff1e')):
            block('unsupported whole Link effect');return
        effects.append(i);occurrence(i,'effect',start,end,text)
    for i,s in enumerate(sections):
        label,text=s['label'],s['text']
        if label==('Link DP' if explicit else 'Special Rule'):
            m=re.fullmatch(r'DP\+([1-9][0-9]*)' if explicit else r'\+([1-9][0-9]*) DP',text)
            if not m:block('unsupported Link DP container')
            else:dp.append(i);occurrence(i,'dp',0,len(text),int(m[1]))
        elif explicit and label=='Link Condition':condition(i,0,len(text))
        elif explicit and label=='Link Effect':effect(i,0,len(text))
        elif not explicit and label=='Inherited Effect' and _LINK.search(text):
            parts=re.split(r'(\r\n|\n)',text)
            if len(parts)!=3:block('combined Link container requires exactly two lines');continue
            condition(i,0,len(parts[0]));effect(i,len(parts[0])+len(parts[1]),len(text))
        elif _LINK.search(label) or _LINK.search(text):block('Link marker outside supported source container')
    if len(dp)!=1:block('one unambiguous Link DP container required')
    if not conditions or not effects:block('complete nonempty Link condition and effect required')
    if explicit and (len(conditions)!=1 or len(effects)!=1):block('ambiguous explicit Link pairing')
    if not explicit and conditions!=effects:block('combined condition/effect occurrence mismatch')
    if len({o['text'] for o in result['occurrences'] if o['role']=='condition'})>1:
        block('ambiguous alternative Link conditions')
    result['occurrences'].sort(key=lambda o:(o['section_index'],o['char_start'],o['role']))
    if not result['blockers']:result['status']='DERIVED_LINK_DIAGNOSTIC'
    return result
