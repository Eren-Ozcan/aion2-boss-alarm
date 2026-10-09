import sys, json, struct, bisect
import analyze as A
f=sys.argv[1]
segs={}; marks=[]
for line in open(f,encoding='utf-8'):
    r=json.loads(line)
    if r['dir']=='MARK': marks.append((r['t'],r['text']))
    elif r['dir']=='S2C': segs.setdefault(r['seq'],(r['t'],bytes.fromhex(r['hex'])))
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
t0=pk[0][0]
l=[(ts,b) for ts,b in pk if b[:2]==b'\x01\x91']
print('0191 count',len(l),'first +%.1fs last +%.1fs'%((l[0][0]-t0)/1000,(l[-1][0]-t0)/1000))
print('marks',[(round((t-t0)/1000,1),x) for t,x in marks])
# other 91 family ops
import collections
c=collections.Counter(b[:2].hex() for ts,b in pk if b[1]==0x91)
print('0x91 family',c)
# entry count byte & length
print(collections.Counter((b[8],len(b)) for ts,b in l).most_common(6))
