import sys, json, bisect
import analyze as A
f='captures/cap_20261009_191529.jsonl'
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
out='../../boss-alarm/src-tauri/tests/fixtures/'
# field list packet matched earlier (screenshot 3 at ~marker), and abyss list
marker=1791563206218
fl=[(ts,b) for ts,b in pk if b[:6]==bytes.fromhex('019100005604') and abs(ts-marker)<10000]
ts,b=fl[0]
open(out+'field_list.hex','w').write(b.hex()+'\n'); print('field',len(b),ts)
ab=[(ts,b) for ts,b in pk if b[:6]==bytes.fromhex('019100001400')][0]
open(out+'abyss_list.hex','w').write(ab[1].hex()+'\n'); print('abyss',len(ab[1]),ab[0])
# server-time ping packet and its capture time
sp=[(ts,b) for ts,b in pk if b[:2]==b'\x00\x36' and len(b)==10]
for ts,b in sp[100:103]:
    v=int.from_bytes(b[2:10],'little'); print('00 36',b.hex(),v, 'server-local ms',v-ts)
