import sys, json, bisect, re
import analyze as A
f=sys.argv[1]
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
T={'Newbold':2492,'Axios':2232,'Barthien':784,'Drakan':15902,'Shujakan':10486,'Karuka':1126,'Vishwada':14523,'Shylak':15493,'Gartua':13227,'Lagta':13590,'Kashapa':14625,'Dartan':14298}
def parse(b):
    pos=[m.start() for m in re.finditer(rb'[\x80-\xff]\xe3\x06',b) if 0x99<=b[m.start()]<=0xd0 and m.start()>=8]
    out={}
    for k,p in enumerate(pos):
        q=pos[k+1] if k+1<len(pos) else len(b)
        seg=b[p:q]
        idv=(seg[0]&0x7f)|((seg[1]&0x7f)<<7)|(seg[2]<<14)
        body=seg[3:]
        # timestamp: 6 bytes ending in a1 01 followed by 00 00
        m=re.search(rb'(.{4}[\x20-\x23]\xa1\x01)\x00\x00',body,re.S) if False else None
        j=body.find(b'\xa1\x01\x00\x00')
        if j>=4:
            v=int.from_bytes(body[j-4:j+2],'little')
            out[idv]=(v,len(body)>=21)
    return out
best=None
for ts,b in pk:
    if b[:2]!=b'\x01\x91': continue
    ent=parse(b); vals=[v for v,_ in ent.values()]
    for v0 in vals:
        for n0,t0 in list(T.items())[:3]:
            c=v0-t0*1000
            n=sum(any(abs(v-(c+t*1000))<1500 for v in vals) for t in T.values())
            if best is None or n>best[0]: best=(n,ts,c,ent)
n,ts,c,ent=best
print('matched',n,'/',len(T))
rem={i:(v-c)/1000 for i,(v,sp) in ent.items()}
for name,t in T.items():
    ids=[i for i,r in rem.items() if abs(r-t)<2]
    print(name,t,'-> id',ids)
print('spawned ids',[i for i,(v,sp) in ent.items() if sp])
print('all ids/rem',{i:round(r) for i,r in sorted(rem.items())})
