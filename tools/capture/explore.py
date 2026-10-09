import sys, json, collections, struct
sys.path.insert(0, '.')
import analyze as A
f = sys.argv[1]
c2s=[]; marks=[]; t_first=None
for line in open(f, encoding='utf-8'):
    r=json.loads(line)
    if r['dir']=='MARK': marks.append((r['t'],r['text']))
    elif r['dir']=='C2S': c2s.append((r['t'],bytes.fromhex(r['hex'])))
print('C2S segments',len(c2s))
for mt,text in marks:
    print('==',text)
    for t,b in c2s:
        if mt-500<=t<=mt+4000:
            print(f'  +{(t-mt)/1000:.2f}s len {len(b)}: {b[:48].hex()}')
