import sys,os,re,struct,math
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from infer import *

KEYS={}
def keyset(t):
    if t not in KEYS:
        try: KEYS[t]={int.from_bytes(k,'little') for k,_ in index(t)[1]}
        except Exception: KEYS[t]=None
    return KEYS[t]

def ref_table(field):
    base=re.sub(r'^_','',field); base=re.sub(r'(List|Arr)$','',base).lower()
    for t in KEYSIZE:
        if t==base or t==base+'info' or t==base.replace('info','')+'info': return t
    return None

BOOL=re.compile(r'^_(is|use|has|enable|disable|can|apply|block|hide|show|need|check|always|only|allow|ignore|lock|auto|keep)[A-Z]')
FLOAT=re.compile(r'(rate|ratio|distance|speed|scale|rotation|radius|angle|weight|offset|height|width|duration|sec|time)$',re.I)

def check(field, kind, size, values):
    """Hard value constraints for one field over all records (values = bytes)."""
    if kind=='u8' and BOOL.match(field):
        return all(v[0] in (0,1) for v in values)
    if kind.startswith('ref'):
        t=ref_table(field); keys=keyset(t) if t else None
        if keys:
            for v in values:
                n=int.from_bytes(v,'little')
                if n and n not in keys and n!=(1<<(8*size))-1: return False
        return True
    if kind=='f32':
        for v in values:
            x=struct.unpack('<f',v)[0]
            if not math.isfinite(x) or (x!=0 and not (1e-8<abs(x)<1e9)): return False
        return True
    return True

def cands(field):
    out=[]
    rt=ref_table(field)
    if BOOL.match(field) and not re.search(r'(List|Arr)$',field): out.append(('u8',1))
    if rt and not re.search(r'(List|Arr)$',field): out.append((f'ref{KEYSIZE[rt]}',KEYSIZE[rt]))
    if FLOAT.search(field): out.append(('f32',4))
    if re.search(r'(position|min|max|pos|scale3|direction|dir)$',field,re.I): out.append(('vec3',12))
    if re.search(r'(list|arr|array)$',field,re.I):
        for n in (1,2,4,8,12,16,24): out.append((f'list<{n}>',('list',n)))
        for n in (16,24,32,48,64): out.append((f'fixed{n}',n))
        out.append(('list<str>',('liststr',)))
        out.append(('list<loc>',('listloc',)))
    if re.search(r'(Name|Desc|String|Memo|Path|Text|Title|Desc2)$',field):
        out += [('loc',('loc',)),('str',('str',)),('hash4',4)]
    for n in (1,2,4,8): out.append((f'i{n}',n))
    out += [('raw12',12),('raw16',16)]
    seen=set(); res=[]
    for k,s in out:
        if k not in seen: seen.add(k); res.append((k,s))
    return res

def step(spec,d,p):
    if isinstance(spec,int): return p+spec if p+spec<=len(d) else None
    t=spec[0]
    if t=='str': return string(d,p)
    if t=='loc': return locstr(d,p)
    if p+4>len(d): return None
    n=int.from_bytes(d[p:p+4],'little'); p+=4
    if t=='list':
        if n>100000 or p+n*spec[1]>len(d): return None
        return p+n*spec[1]
    for _ in range(n if n<100000 else 0):
        p=string(d,p) if t=='liststr' else locstr(d,p)
        if p is None: return None
    return p if n<100000 else None

def solve(table, sample=400, max_states=20000):
    ks,recs=records(table)
    cls,_=fields(table)
    fl=order(cls) if cls else []
    data=[p for _,_,p in recs]
    idx=sorted(range(len(data)),key=lambda i:len(data[i]))
    pick=sorted(set(idx[:50]+idx[-50:]+idx[::max(1,len(idx)//sample)]))
    sdata=[data[i] for i in pick]
    states=[{tuple([0]*len(sdata)):[]}]   # per level: state -> list of (prev_state, kind)
    for i,f in enumerate(fl):
        nxt={}
        for st in states[-1]:
            for kind,spec in cands(f):
                new=[]; vals=[]
                for p,d in zip(st,sdata):
                    q=step(spec,d,p)
                    if q is None: break
                    new.append(q)
                    if isinstance(spec,int): vals.append(d[p:q])
                else:
                    if isinstance(spec,int) and not check(f,kind,spec,vals): continue
                    nxt.setdefault(tuple(new),[]).append((st,kind))
        if not nxt or len(nxt)>max_states:
            return cls,fl,None,f"stuck at field {i} {f} ({len(nxt)} states)"
        states.append(nxt)
    final=tuple(len(d) for d in sdata)
    if final not in states[-1]: return cls,fl,None,"no layout ends exactly"
    # backward: keep states that reach final
    alive=[set() for _ in states]; alive[-1]={final}
    choices=[set() for _ in fl]
    for i in range(len(fl),0,-1):
        for st in alive[i]:
            for prev,kind in states[i][st]:
                alive[i-1].add(prev); choices[i-1].add((prev[0] if prev else 0,kind))
    layout=[]
    for f,ch in zip(fl,choices):
        layout.append((f, sorted({k for _,k in ch}), 'proven' if len(ch)==1 else 'ambiguous'))
    return cls,fl,layout,f"{len(sdata)} sample records"

def verify(table, layout):
    """Exact decode of every record with a fully determined layout."""
    ks,recs=records(table)
    specs=[dict(cands(f))[k[0]] for f,k,_ in layout]
    for _,_,d in recs:
        p=0
        for s in specs:
            p=step(s,d,p)
            if p is None: return False
        if p!=len(d): return False
    return True

if __name__=="__main__":
    for t in sys.argv[1:]:
        cls,fl,layout,note=solve(t)
        if not layout: print(f"== {t}: no ({note})"); continue
        proven=sum(1 for *_ ,c in layout if c=='proven')
        full=all(len(k)==1 for _,k,_ in layout)
        print(f"== {t} ({cls}): {proven}/{len(layout)} proven; {note}; all-records: {verify(t,layout) if full else 'n/a'}")
        for f,k,c in layout: print(f"     {f:40} {'/'.join(k):30} {c}")
