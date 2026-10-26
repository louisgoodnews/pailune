import struct,json,os
S=os.environ.get('PYLUNE_LAYOUT_WORK', os.getcwd())  # holds tables/ and names.json
def index(name):
    h=open(f"{S}/tables/{name}.staticinfoheader",'rb').read()
    for cs in (2,4,1):
        n=int.from_bytes(h[:cs],'little')
        for ks in (4,2,8,12):
            if n>0 and len(h)==cs+n*(ks+4):
                rows=[(h[cs+i*(ks+4):cs+i*(ks+4)+ks], int.from_bytes(h[cs+i*(ks+4)+ks:cs+i*(ks+4)+ks+4],'little')) for i in range(n)]
                return ks,rows
def records(name):
    ks,rows=index(name); b=open(f"{S}/tables/{name}.staticinfobody",'rb').read()
    offs=sorted(o for _,o in rows)+[len(b)]
    out=[]
    for s,e in zip(offs,offs[1:]):
        nl=int.from_bytes(b[s+ks:s+ks+4],'little')
        out.append((b[s:s+ks], b[s+ks+4:s+ks+4+nl].decode('utf8','replace'), b[s+ks+4+nl:e]))
    return ks,out
names=json.load(open(f"{S}/names.json"))
lower={c.lower():c for c in names}
def fields(table):
    c=lower.get(table)
    return c,[f for _,f,_ in sorted(names[c])] if c else []
