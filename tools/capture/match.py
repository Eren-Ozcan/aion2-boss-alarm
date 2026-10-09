import sys, json, struct, bisect
import analyze as A
f=sys.argv[1]; mark_ms=int(sys.argv[2])
segs={}
for line in open(f,encoding='utf-8'):
    r=json.loads(line)
    if r['dir']=='S2C': segs.setdefault(r['seq'],(r['t'],bytes.fromhex(r['hex'])))
stream=bytearray(); starts=[]; nxt=None
for sq in sorted(segs):
    ts,d=segs[sq]
    if nxt is not None and sq<nxt:
        k=nxt-sq
        if k>=len(d): continue
        d=d[k:]; sq=nxt
    starts.append((len(stream),ts)); stream+=d; nxt=sq+len(d)
offs=[s for s,_ in starts]
frames,_=A.walk(bytes(stream))
pk=[]
for s,e,w,b in frames:
    ts=starts[max(bisect.bisect_right(offs,s)-1,0)][1]
    for body in (A.explode(bytes(stream[s+w:e])) if b else [bytes(stream[s+w:e])]): pk.append((ts,body))
T=[324,2693,6565,311,4620,2759,3822,10292]
names=['Rajit','Vargor','Garsan','Lannar','Trid','Kelpina','Nutah','Resana']
def entries(b):
    # records: id varint(3) .. ts at 6 bytes; scan every 12 bytes by locating 'a1 01' patterns
    out=[]
    for i in range(3,len(b)-5):
        if b[i+4]==0x21 and b[i+5]==0xa1 or (b[i+4]==0x22 and b[i+5]==0xa1):
            pass
    return out
def times(b):
    res=[]
    i=2
    while i<len(b)-8:
        v=int.from_bytes(b[i:i+6],'little')
        if 1.78e12<v<1.80e12 and b[i+5]==0x01 and b[i+4]==0xa1 and b[i-3+0]!=None:
            res.append((i,v)); i+=6
        else: i+=1
    return res
best=None
cands=[(ts,b) for ts,b in pk if b[:2]==b'\x01\x91' and abs(ts-mark_ms)<150000]
print(len(cands),'candidate packets')
for ts,b in cands:
    ent=times(b)
    vals=[v for _,v in ent]
    for v0 in vals:
        c=v0-T[0]*1000
        n=sum(any(abs(v-(c+t*1000))<1500 for v in vals) for t in T)
        if best is None or n>best[0]: best=(n,ts,c,b,ent)
n,ts,c,b,ent=best
print('best matches',n,'/',len(T),'at local +%.1fs from marker'%((ts-mark_ms)/1000),'server-local offset %.1fs'%((c-ts)/1000))
for i,v in ent:
    rem=(v-c)/1000
    idv=b[i-3:i]  # id varint just before ts
    # id may be 3 bytes before ts, except first entry with flag byte
    print(f'ts@{i:3d} id? {b[i-3:i].hex()} rem {rem:8.0f}s ({int(rem//3600)}h{int(rem%3600//60):02d}m{int(rem%60):02d}s)')

print('\n--- segments by id')
import re
pos=[m.start() for m in re.finditer(rb'[\x80-\xff]\xe3\x06',b) if 0x99<=b[m.start()]<=0xb8]
pos=[p for p in pos if p>=8]
for k,p in enumerate(pos):
    q=pos[k+1] if k+1<len(pos) else len(b)
    seg=b[p:q]
    idv=(seg[0]&0x7f)|((seg[1]&0x7f)<<7)|(seg[2]<<14)
    body=seg[3:]
    if len(body)>=9 and body[-9:-3] and len(body)>=6:
        pass
    print(f'id {idv} ({seg[:3].hex()}) len {len(seg)} body {body.hex(" ")}')
