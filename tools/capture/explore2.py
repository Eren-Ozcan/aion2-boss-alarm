import sys, json, collections, struct
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
import bisect
offs=[s for s,_ in starts]
frames,_=A.walk(bytes(stream))
pk=[]
for s,e,w,b in frames:
    ts=starts[max(bisect.bisect_right(offs,s)-1,0)][1]
    bodies=A.explode(bytes(stream[s+w:e])) if b else [bytes(stream[s+w:e])]
    for body in bodies: pk.append((ts,body))
t0=pk[0][0]; tm=marks[0][0]/1000
print('now-ish epoch', tm)
# 1) largest non-spawn packets
big=sorted(pk,key=lambda p:-len(p[1]))[:15]
print('largest:')
for ts,b in big: print(f'  +{(ts-t0)/1000:.1f}s op {b[:2].hex()} len {len(b)}')
# 2) unix timestamp u32/u64 within +-2 days
hits=collections.Counter(); ex={}
for ts,b in pk:
    for i in range(len(b)-3):
        v=struct.unpack_from('<I',b,i)[0]
        if abs(v-tm)<172800:
            hits[b[:2].hex()]+=1; ex.setdefault(b[:2].hex(),(ts,i,v-tm,b))
    for i in range(len(b)-7):
        v=struct.unpack_from('<Q',b,i)[0]
        if abs(v/1000-tm)<172800:
            hits[b[:2].hex()+'(ms)']+=1; ex.setdefault(b[:2].hex()+'(ms)',(ts,i,v/1000-tm,b))
print('opcodes containing epoch-like values:')
for op,n in hits.most_common(15):
    ts,i,dv,b=ex[op]; print(f'  {op}: {n}  e.g. +{(ts-t0)/1000:.1f}s off {i} delta {dv:.0f}s len {len(b)} {b[:40].hex()}')

print('\n=== op 0191 packets')
import datetime
for ts,b in pk:
    if b[:2].hex()!='0191': continue
    vals=[]
    for i in range(2,len(b)-7):
        v=struct.unpack_from('<Q',b,i)[0]
        if abs(v/1000-ts/1000)<200000: vals.append((i,round(v/1000-ts/1000)))
    print(f'+{(ts-t0)/1000:.1f}s len {len(b)} head {b[2:14].hex()} times(rel s): {vals[:12]}')

print('\n=== dump')
for k,(ts,b) in enumerate([p for p in pk if p[1][:2].hex()=='0191'][:1]+[p for p in pk if p[1][:2].hex()=='0191'][30:31]):
    print(len(b)); 
    for i in range(0,len(b),12): print(f'{i:4d}', b[i:i+12].hex(' '))
print('\nother ops with same shape:')
for ts,b in pk:
    if b[:2].hex() in ('0c91',): print('0c91',len(b),b[:40].hex(' ')); break
