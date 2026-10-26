import sys,os,re,struct,math
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from dp import *

INF=float('inf')
def int_cost(b):
    n=len(b); v=int.from_bytes(b,'little'); s=v-(1<<(8*n)) if v>=1<<(8*n-1) else v
    return math.log2(min(v,abs(s))+1)+1
def f32_cost(b):
    x=struct.unpack('<f',b)[0]
    if x==0: return 1
    if not math.isfinite(x) or not (1e-6<abs(x)<1e8): return 40
    for digits in range(0,5):
        if abs(round(x,digits)-x)<=abs(x)*1e-6: return 3+digits*3.3+math.log2(abs(x)+1)
    return 26
def cost(field,kind,vals):
    if kind=='u8': return sum(1 if v[0] in (0,1) else INF for v in vals)
    if kind.startswith('ref'): return 3*len(vals)
    if kind=='f32': return sum(f32_cost(v) for v in vals)
    if kind=='vec3': return sum(f32_cost(v[i:i+4]) for v in vals for i in (0,4,8))
    if kind=='hash4': return 30*len(vals)
    if kind.startswith('i'): return sum(int_cost(v) for v in vals)
    if kind.startswith('raw'): return sum(int_cost(v[i:i+4]) for v in vals for i in range(0,len(v),4))+4*len(vals)
    return 2*len(vals)  # variable-length structures: structure cost only

def cands2(field):
    c=cands(field)
    # C++ bools are one byte; the value check (0/1) guards the name rule.
    if BOOL.match(field) and not re.search(r'(List|Arr)$',field): return [('u8',1)]
    return c

def solve2(table, sample=400, max_states=40000):
    ks,recs=records(table)
    cls,_=fields(table)
    fl=order(cls) if cls else []
    data=[p for _,_,p in recs]
    idx=sorted(range(len(data)),key=lambda i:len(data[i]))
    pick=sorted(set(idx[:60]+idx[-60:]+idx[::max(1,len(idx)//sample)]))
    sd=[data[i] for i in pick]
    start=tuple([0]*len(sd))
    levels=[{start:0.0}]; edges=[]
    for i,f in enumerate(fl):
        nxt={}; ed=[]
        for st,c0 in levels[-1].items():
            for kind,spec in cands2(f):
                new=[]; vals=[]
                for p,d in zip(st,sd):
                    q=step(spec,d,p)
                    if q is None: break
                    new.append(q)
                    if isinstance(spec,int): vals.append(d[p:q])
                else:
                    if isinstance(spec,int) and not check(f,kind,spec,vals): continue
                    c=cost(f,kind,vals) if vals else cost(f,kind,[b'']*len(sd))
                    if c==INF: continue
                    t=tuple(new); ed.append((st,t,kind,c))
                    if c0+c<nxt.get(t,INF): nxt[t]=c0+c
        if not nxt or len(nxt)>max_states: return cls,fl,None,f"stuck at {i} {f} ({len(nxt)} states)"
        levels.append(nxt); edges.append(ed)
    final=tuple(len(d) for d in sd)
    if final not in levels[-1]: return cls,fl,None,"no layout ends exactly"
    # backward costs
    B=[dict() for _ in levels]; B[-1]={final:0.0}
    for i in range(len(fl)-1,-1,-1):
        for st,t,kind,c in edges[i]:
            if t in B[i+1]:
                v=c+B[i+1][t]
                if v<B[i].get(st,INF): B[i][st]=v
    best=B[0][start]
    layout=[]; st=start
    for i,f in enumerate(fl):
        opts=[(levels[i][s]+c+B[i+1][t],s,t,k) for s,t,k,c in edges[i] if s in levels[i] and t in B[i+1]]
        here=min(o for o in opts if o[1]==st and abs(o[0]-best)<1e-6 or o[1]==st)
        # choose the edge on the best path from current state
        cand=[o for o in opts if o[1]==st]
        o=min(cand)
        alt=[x[0] for x in opts if (x[1][0] if x[1] else 0, x[3])!=(st[0] if st else 0, o[3]) or x[1]!=st]
        alts=[x[0] for x in opts if not (x[1]==st and x[2]==o[2])]
        margin=(min(alts)-best) if alts else INF
        offset=st[0]
        kinds={x[3] for x in opts if x[1]==st and x[2]==o[2] and x[0]<INF}
        layout.append((f,o[3],margin,len(kinds)))
        st=o[2]
    return cls,fl,layout,f"best cost {best:.0f}, {len(sd)} sample records"

def verify2(table, layout):
    ks,recs=records(table)
    specs=[dict(cands(f)+[('u8',1)])[k] for f,k,*_ in layout]
    for _,_,d in recs:
        p=0
        for s in specs:
            p=step(s,d,p)
            if p is None: return False
        if p!=len(d): return False
    return True

def label(m): return 'proven' if m==INF else f'inferred (margin {m:.0f})'
if __name__=="__main__":
    for t in sys.argv[1:]:
        cls,fl,layout,note=solve2(t)
        if not layout: print(f"== {t}: no ({note})"); continue
        print(f"== {t} ({cls}): {note}; exact on all {len(records(t)[1])} records: {verify2(t,layout)}")
        for f,k,m,n in layout: print(f"     {f:40} {k:12} {label(m)}{'' if n==1 else ' (meaning: '+str(n)+' readings)'}")

def prefix(table, upto, sample=150):
    """Best layout for the first `upto` fields, and each record's position after it."""
    ks,recs=records(table); cls,_=fields(table); fl=order(cls)[:upto]
    data=[p for _,_,p in recs]
    idx=sorted(range(len(data)),key=lambda i:len(data[i]))
    pick=sorted(set(idx[:40]+idx[-40:]+idx[::max(1,len(idx)//sample)]))
    sd=[data[i] for i in pick]
    levels=[{tuple([0]*len(sd)):(0.0,None,None)}]
    for f in fl:
        nxt={}
        for st,(c0,_,_) in levels[-1].items():
            for kind,spec in cands2(f):
                new=[];vals=[]
                for p,d in zip(st,sd):
                    q=step(spec,d,p)
                    if q is None: break
                    new.append(q)
                    if isinstance(spec,int): vals.append(d[p:q])
                else:
                    if isinstance(spec,int) and not check(f,kind,spec,vals): continue
                    c=cost(f,kind,vals) if vals else 2*len(sd)
                    if c==INF: continue
                    t=tuple(new)
                    if c0+c<nxt.get(t,(INF,))[0]: nxt[t]=(c0+c,st,kind)
        levels.append(nxt)
    st=min(levels[-1],key=lambda s:levels[-1][s][0]); path=[]
    for i in range(len(fl),0,-1):
        c,prev,kind=levels[i][st]; path.append((fl[i-1],kind,st[0])); st=prev
    return list(reversed(path)), sd, min(levels[-1],key=lambda s:levels[-1][s][0])
