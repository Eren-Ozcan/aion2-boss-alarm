import sys, json, bisect, re, collections
import analyze as A
f=sys.argv[1]
segs={}; marks=[]
for line in open(f,encoding='utf-8'):
    r=json.loads(line)
    if r['dir']=='MARK': marks.append((r['t'],r['text']))
    elif r['dir']=='S2C': segs.setdefault(r['seq'],(r['t'],bytes.fromhex(r['hex'])))
print(marks[-4:])
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
cnt=collections.Counter(b[:2].hex() for _,b in pk)
print('ops 91 family / big:', {k:v for k,v in cnt.items() if k.endswith('91') or k.endswith('92')})
def times(b):
    out=[]
    for i in range(2,len(b)-6):
        if b[i+4]==0xa1 and b[i+5]==0x01:
            v=int.from_bytes(b[i:i+6],'little')
            if 1.78e12<v<1.80e12: out.append((i,v))
    return out
# group by opcode: packets with many timestamps, after last 'esleme' marker
last=[t for t,x in marks if 'abis' in x]
cut=last[0] if last else marks[-1][0]
print('cut at +%.0fs'%((cut-t0)/1000))
seen=collections.Counter()
for ts,b in pk:
    if ts<cut-5000: continue
    n=len(times(b))
    if n>=3: seen[b[:2].hex()]+=1
print('ops with >=3 timestamps after cut:',seen)

print('\n-- 0191 after abyss marker')
for ts,b in pk:
    if ts>=cut-3000 and b[:2]==b'\x01\x91':
        print('+%.1fs len %d head %s'%((ts-cut)/1000,len(b),b[:14].hex(' ')))
print('\n-- 2d92 / 2e92 / 2f92 / 3292 samples')
shown=collections.Counter()
for ts,b in pk:
    k=b[:2].hex()
    if k in('2d92','2e92','2f92','3292') and shown[k]<3:
        shown[k]+=1; print(k,'+%.1fs'%((ts-cut)/1000),len(b),b[:40].hex(' '))

print('\n-- abyss packet dump')
ab=[(ts,b) for ts,b in pk if b[:6]==bytes.fromhex('019100001400') and ts>=cut-3000]
ts,b=ab[5]
for i in range(0,len(b),12): print(f'{i:4d}',b[i:i+12].hex(' '))
srv=[]
for i in range(2,len(b)-6):
    if b[i+4]==0xa1 and b[i+5]==0x01:
        v=int.from_bytes(b[i:i+6],'little')
        if 1.78e12<v<1.80e12: srv.append((i,v))
print(srv)
base=min(v for _,v in srv)
for i,v in srv: print(i, 'secs from smallest', (v-base)/1000, 'id?', b[i-3:i].hex(' '))
# continuity check: is the same list in the other packets updated?
import itertools
print('distinct payloads', len(set(b2 for _,b2 in ab)))

print('\n-- search for ts ~ cut+1190s (+-120s) in any packet after cut-30s')
lo=cut+1070_000; hi=cut+1310_000
hits=collections.Counter(); ex={}
for ts,b in pk:
    if ts<cut-30000: continue
    for i in range(0,len(b)-5):
        v=int.from_bytes(b[i:i+6],'little')
        if lo<=v<=hi:
            k=b[:2].hex(); hits[k]+=1; ex.setdefault(k,(ts,i,(v-ts)/1000,b))
for k,n in hits.most_common(10):
    ts,i,dv,b=ex[k]; print(k,n,'off',i,'rel to pkt time %.0fs'%dv,len(b),b[:30].hex(' '))

print('\n-- whole-capture search (u48 and u64, also u32 seconds)')
hits=collections.Counter(); ex={}
for ts,b in pk:
    for i in range(0,len(b)-5):
        v=int.from_bytes(b[i:i+6],'little')
        if lo<=v<=hi:
            k=b[:2].hex(); hits[k]+=1; ex.setdefault(k,(ts,i,(v-ts)/1000,b))
    for i in range(0,len(b)-3):
        v=int.from_bytes(b[i:i+4],'little')
        if (lo//1000)<=v<=(hi//1000):
            k=b[:2].hex()+'(s)'; hits[k]+=1; ex.setdefault(k,(ts,i,v-ts/1000,b))
for k,n in hits.most_common(10):
    ts,i,dv,b=ex[k]; print(k,n,'at +%.0fs'%((ts-t0)/1000),'off',i,'rel %.0fs'%dv,len(b),b[:30].hex(' '))
print('\n-- all 2d92/2e92/2f92/3292 packets in order')
for ts,b in pk:
    k=b[:2].hex()
    if k in('2d92','2e92','2f92','3292') and ts>cut-900000:
        print(k,'%+.0fs'%((ts-cut)/1000),len(b),b[:70].hex(' '))

print('\n-- 2e92 event timestamps relative to marker (server clock ~ local+-10s)')
for ts,b in pk:
    if b[:2].hex()=='2e92':
        name=b[7:7+b[6]].decode(errors="replace")
        for i in range(7+b[6],len(b)-5):
            v=int.from_bytes(b[i:i+6],'little')
            if 1.78e12<v<1.80e12:
                print(name,'| recv %+.0fs | event ts %+.0fs rel marker (%+.0fs rel recv)'%((ts-cut)/1000,(v-cut)/1000,(v-ts)/1000)); break

print('\n-- abyss list variants over time')
vs=collections.OrderedDict()
for ts,b in pk:
    if b[:6]==bytes.fromhex('019100001400'):
        vs.setdefault(b,[ts,ts]); vs[b][1]=ts
for b,(a,z) in vs.items():
    print('seen %+.0fs..%+.0fs'%((a-cut)/1000,(z-cut)/1000),len(b),b[9:60].hex(' '))
