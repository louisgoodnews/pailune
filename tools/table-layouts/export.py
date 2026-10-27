import sys,os,json,re
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from mdl import *
def shape(kind):
    m=re.fullmatch(r'(?:i|ref|raw|fixed)(\d+)',kind)
    if m: return f"fixed:{m.group(1)}"
    return {'u8':'fixed:1','f32':'fixed:4','hash4':'fixed:4','vec3':'fixed:12','str':'str','loc':'loc','list<str>':'list:str','list<loc>':'list:loc'}.get(kind) or ("list:"+kind[5:-1] if kind.startswith('list<') else kind)
def meaning(field,kind):
    if kind=='u8': return 'bool'
    if kind=='f32': return 'float'
    if kind.startswith('ref'): return 'ref'
    if kind=='hash4': return 'hash'
    if kind=='vec3': return 'vec3'
    if re.fullmatch(r'i\d+',kind): return 'int'
    return 'raw'
out={}
for t in sorted(TABLES):
    try: ks,recs=records(t)
    except Exception: continue
    cls,fl,layout,note=solve2(t,sample=200,max_states=20000)
    if not layout or not verify2(t,layout): continue
    # Too little evidence, or opaque blocks standing in for unknown types.
    if len(recs)<10 or any(re.fullmatch(r'raw\d+|fixed\d+',k) and not re.search(r'(list|arr|array)$',f,re.I) for f,k,_,_ in layout): continue
    shapes=[shape(k) for _,k,_,_ in layout]
    fields=[]
    for i,(f,k,m,n) in enumerate(layout):
        same=sum(1 for s in shapes if s==shapes[i])
        offset_proven = m==INF
        name_proven = offset_proven and (same==1 or (i==0 and f=='_isBlocked'))
        fields.append({"name":f,"shape":shapes[i],"meaning":meaning(f,k),"offset":"proven" if offset_proven else "inferred","name_certain":name_proven})
    out[t]={"class":cls,"key_size":ks,"records":len(recs),"fields":fields}
# Partial layouts: only the leading fields, for tables whose full layout is not solved yet. Each
# must decode on every record; the evidence is written next to it.
PARTIAL={'iteminfo':("ItemInfo",[("_isBlocked","fixed:1","bool"),("_maxStackCount","fixed:8","int")],
  "leading fields solved on 6753 of 6816 records (phase 2 engine); _maxStackCount confirmed on 2391 "
  "records by the published Fat Stacks mod (new = multiple of the u64 at +1)")}
for t,(cls,lead,evidence) in PARTIAL.items():
    ks,recs=records(t)
    need=sum(int(sh.split(':')[1]) for _,sh,_ in lead)
    if t in out or any(len(payload)<need for _,_,payload in recs): continue
    out[t]={"class":cls,"key_size":ks,"records":len(recs),"partial":True,"evidence":evidence,
      "fields":[{"name":f,"shape":sh,"meaning":m,"offset":"proven","name_certain":True} for f,sh,m in lead]}
json.dump({"format":1,"game_version":"2.3.2","derived":"Pailune table-layout research (docs/research/table-layouts.md)","tables":out},open(sys.argv[1],'w'),indent=1)
print(len(out),"tables;",sum(1 for t in out.values() for f in t['fields'] if f['name_certain']),"editable fields;",sum(len(t['fields']) for t in out.values()),"fields")
