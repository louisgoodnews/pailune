"""Recursive layout inference: scalars, strings, lists, nested structures."""
import sys,os,re,struct,math
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from mdl import KEYSIZE, keyset, ref_table, BOOL, FLOAT, int_cost, f32_cost, INF, string, locstr
from tbl import names, records, fields

def class_fields(cls):
    fl=[f for _,f,_ in sorted(names[cls])]
    fl=[f for f in dict.fromkeys(fl) if f.startswith('_') and f not in ('_key','_stringKey')]
    if '_isBlocked' in fl:
        i=fl.index('_isBlocked'); fl=fl[i:]+fl[:i]
    return fl

def struct_class(field, owner):
    base=re.sub(r'^_','',field)
    base=re.sub(r'(List|Arr|Array)$','',base)
    cap=base[:1].upper()+base[1:]
    for c in (cap, cap+'Data', f'{owner}_{cap}', f'{owner}_{cap}Data', cap.replace('Data','')+'Data'):
        if c in names and c!=owner and class_fields(c): return c
    return None

# --- parsers: spec is a tuple tree; step(spec,d,p)->p or None
def step(spec,d,p):
    t=spec[0]
    if t=='fix':
        q=p+spec[1]; return q if q<=len(d) else None
    if t=='str': return string(d,p)
    if t=='loc': return locstr(d,p)
    if t in ('list','opt'):
        if p+4>len(d): return None
        n=int.from_bytes(d[p:p+4],'little')
        if n>20000: return None
        p+=4
        el=spec[1]
        if el[0]=='fix':
            q=p+n*el[1]; return q if q<=len(d) else None
        for _ in range(n):
            p=step(el,d,p)
            if p is None: return None
        return p
    if t=='seq':
        for s in spec[1]:
            p=step(s,d,p)
            if p is None: return None
        return p
    raise ValueError(t)

def vcost(kind,vals):
    if kind=='bool': return sum(1 if v[0] in (0,1) else INF for v in vals)
    if kind.startswith('ref'): return 3*len(vals)
    if kind=='f32': return sum(f32_cost(v) for v in vals)
    if kind=='vec3': return sum(f32_cost(v[i:i+4]) for v in vals for i in (0,4,8))
    if kind=='hash4': return 30*len(vals)
    if kind.startswith('i'): return sum(int_cost(v) for v in vals)
    return 0

def scalar_cands(field):
    out=[]
    rt=ref_table(field)
    if rt: out.append((f'ref{KEYSIZE[rt]}',('fix',KEYSIZE[rt])))
    if FLOAT.search(field): out.append(('f32',('fix',4)))
    if re.search(r'(position|min|max|pos|direction|dir)$',field,re.I): out.append(('vec3',('fix',12)))
    if re.search(r'(name|desc|string|memo|path|text|title|desc2|hash)$',field,re.I):
        out += [('loc',('loc',)),('str',('str',)),('hash4',('fix',4))]
    for n in (1,2,4,8): out.append((f'i{n}',('fix',n)))
    return out

def check_ref(field,kind,vals):
    t=ref_table(field); keys=keyset(t) if t else None
    if not keys: return True
    size=len(vals[0]) if vals else 0
    return all((n:=int.from_bytes(v,'little'))==0 or n in keys or n==(1<<(8*size))-1 for v in vals)

class Solver:
    EK=40  # element layouts kept per nested class (the parent's exact fit decides)
    def __init__(self, owner, depth=0, K=6, max_states=20000):
        self.owner=owner; self.depth=depth; self.K=K; self.max_states=max_states

    def cands(self, field, sd, positions):
        """(label, spec) candidates for a field, given current per-record positions."""
        if BOOL.match(field) and not re.search(r'(list|arr|array)$',field,re.I):
            return [('bool',('fix',1))]
        out=[]
        islist=re.search(r'(list|arr|array)$',field,re.I)
        cls=struct_class(field,self.owner) if self.depth<3 else None
        if cls:
            if islist:
                firsts=[]
                for d,p in zip(sd,positions):
                    if p+4<=len(d) and 0<int.from_bytes(d[p:p+4],'little')<20000: firsts.append((d,p+4))
                for lab,el in self.elements(cls,firsts):
                    out.append((f'list<{lab}>',('list',el)))
            else:
                for lab,el in self.elements(cls,list(zip(sd,positions))):
                    out.append((lab,el))
        if islist:
            for n in (1,2,4,8,12,16): out.append((f'list<{n}>',('list',('fix',n))))
            out += [('list<str>',('list',('str',))),('list<loc>',('list',('loc',)))]
            for n in (16,24,32,48,64): out.append((f'fixed{n}',('fix',n)))
            return out
        return out+scalar_cands(field)

    def elements(self, cls, starts):
        """Top element layouts of a nested class from first-element positions."""
        if not starts: return []
        key=(cls,tuple((id(d),p) for d,p in starts))
        cache=self.__dict__.setdefault('cache',{})
        if key not in cache:
            sub=Solver(cls,self.depth+1,self.EK,self.max_states); sub.EK=self.EK
            res=sub.run(class_fields(cls),[d for d,_ in starts],[p for _,p in starts],exact=False)
            cache[key]=[(f"{cls}{{{','.join(k for _,k in lay)}}}",('seq',[s for s,_ in specs])) for cost,lay,specs in res[:self.EK]]
        return cache[key]

    def run(self, fl, sd, start, exact=True):
        """Best layouts: list of (cost, [(field,label)], [(spec,label)])."""
        levels=[{tuple(start):(0.0,None)}]
        for i,f in enumerate(fl):
            nxt={}
            cur=levels[-1]
            # candidates depend on positions only for structures; compute per state
            for st,(c0,_) in cur.items():
                for lab,spec in self.cands(f,sd,st):
                    new=[];vals=[]
                    for p,d in zip(st,sd):
                        q=step(spec,d,p)
                        if q is None: break
                        new.append(q)
                        if spec[0]=='fix': vals.append(d[p:q])
                    else:
                        kind=lab
                        if kind.startswith('ref') and not check_ref(f,kind,vals): continue
                        c=vcost(kind,vals) if vals else 2*len(sd)
                        if c==INF: continue
                        t=tuple(new)
                        if c0+c<nxt.get(t,(INF,))[0]: nxt[t]=(c0+c,(st,f,lab,spec))
            if not nxt:
                if self.depth==0: self.fail=(i,f,len(cur))
                return []
            if len(nxt)>self.max_states:
                nxt=dict(sorted(nxt.items(),key=lambda kv:kv[1][0])[:self.max_states])
            levels.append(nxt)
        ends=levels[-1]
        if exact:
            final=tuple(len(d) for d in sd)
            ends={final:ends[final]} if final in ends else {}
        out=[]
        for st,(c,_) in sorted(ends.items(),key=lambda kv:kv[1][0])[:max(self.K,1)]:
            lay=[];specs=[];s=st
            for i in range(len(fl),0,-1):
                _,(prev,f,lab,spec)=levels[i][s]
                lay.append((f,lab)); specs.append((spec,lab)); s=prev
            out.append((c,list(reversed(lay)),list(reversed(specs))))
        return out

def solve_table(table, sample=200):
    ks,recs=records(table); cls,_=fields(table)
    if not cls: return None,"no class"
    data=[p for _,_,p in recs]
    idx=sorted(range(len(data)),key=lambda i:len(data[i]))
    pick=sorted(set(idx[:40]+idx[-40:]+idx[::max(1,len(idx)//sample)]))
    sd=[data[i] for i in pick]
    s=Solver(cls)
    fl=class_fields(cls)
    res=s.run(fl,sd,[0]*len(sd),exact=True)
    if not res: return None,f"no exact layout (stuck {getattr(s,'fail',None)})"
    cost,lay,specs=res[0]
    full=('seq',[sp for sp,_ in specs])
    ok=all(step(full,d,0)==len(d) for d in data)
    return (lay,specs,ok),f"cost {cost:.0f}"

if __name__=="__main__":
    for t in sys.argv[1:]:
        r,note=solve_table(t)
        if not r: print(f"== {t}: {note}"); continue
        lay,specs,ok=r
        print(f"== {t}: {note}; exact on every record: {ok}")
        for f,lab in lay: print(f"     {f:40} {lab[:110]}")
