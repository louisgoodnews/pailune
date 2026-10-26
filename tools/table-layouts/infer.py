import sys,os,re,struct
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from tbl import *
import os as _os
TABLES=[f[:-15] for f in _os.listdir(f"{S}/tables") if f.endswith('.staticinfobody')]
KEYSIZE={}
for t in TABLES:
    try: KEYSIZE[t]=index(t)[0]
    except Exception: pass

def order(cls):
    fl=[f for _,f,_ in sorted(names[cls])] if cls in names else []
    fl=[f for f in dict.fromkeys(fl) if f.startswith('_') and f not in ('_key','_stringKey')]
    if '_isBlocked' in fl:
        i=fl.index('_isBlocked'); fl=fl[i:]+fl[:i]
    return fl

# --- primitive readers: (name, fn(data,pos)->newpos or None)
def fixed(n): return lambda d,p: p+n if p+n<=len(d) else None
def string(d,p):
    if p+4>len(d): return None
    n=int.from_bytes(d[p:p+4],'little')
    return p+4+n if n<100000 and p+4+n<=len(d) else None
def locstr(d,p):
    if p+9>len(d): return None
    return string(d,p+9)
def listof(elem):
    def f(d,p):
        if p+4>len(d): return None
        n=int.from_bytes(d[p:p+4],'little')
        if n>200000: return None
        p+=4
        for _ in range(n):
            p=elem(d,p)
            if p is None: return None
        return p
    return f

def ref_size(field):
    base=re.sub(r'^_','',field); base=re.sub(r'(List|Arr)$','',base)
    cand=base.lower()
    for t,ks in KEYSIZE.items():
        if t==cand or t==cand.replace('info','')+'info': return ks
    return None

def candidates(field):
    f=field.lower(); out=[]
    rs=ref_size(field)
    isref=re.search(r'(key|info)$',f) is not None
    if re.match(r'_(is|use|has|enable|disable|can|apply|block|hide|show|need|check|always|only|allow|ignore|lock|auto|keep)',field) and not re.search(r'(list|arr)$',f):
        out.append(('u8',fixed(1)))
    if isref and rs: out.append((f'ref{rs}',fixed(rs)))
    if re.search(r'(list|arr|array|set)$',f):
        for n in (4,2,1,8,12,16):
            out.append((f'list<{n}>',listof(fixed(n))))
        out.append(('list<str>',listof(string)))
        out.append(('list<loc>',listof(locstr)))
    if re.search(r'(name|desc|string|memo|path|text|title)',f):
        out+= [('loc',locstr),('str',string),('hash4',fixed(4))]
    if re.search(r'(position|scale3|offset3|min$|max$)',f): out.append(('vec3',fixed(12)))
    for n in (1,4,2,8,12,16):
        out.append((f'b{n}',fixed(n)))
    seen=set(); res=[]
    for n,fn in out:
        if n not in seen: seen.add(n); res.append((n,fn))
    return res

def solve(table, limit_nodes=200000, sample=None):
    ks,recs=records(table)
    cls,_=fields(table)
    fl=order(cls) if cls else []
    data=[p for _,_,p in recs]
    if sample: 
        step=max(1,len(data)//sample); data=data[::step][:sample]+data[-3:]
    nodes=[0]; result=[]
    def dfs(i, positions, chosen):
        nodes[0]+=1
        if nodes[0]>limit_nodes: return False
        if i==len(fl):
            if all(p==len(d) for p,d in zip(positions,data)):
                result.append(list(chosen)); return True
            return False
        for name,fn in candidates(fl[i]):
            new=[]
            for p,d in zip(positions,data):
                q=fn(d,p)
                if q is None: break
                new.append(q)
            else:
                chosen.append((fl[i],name))
                if dfs(i+1,new,chosen): return True
                chosen.pop()
        return False
    ok=dfs(0,[0]*len(data),[])
    return cls,fl,(result[0] if ok else None),nodes[0]

if __name__=="__main__":
    for t in sys.argv[1:]:
        cls,fl,res,n=solve(t)
        print(f"== {t} ({cls}, {len(fl)} fields) nodes {n}: {'SOLVED' if res else 'no layout'}")
        if res: print("   ",res)
