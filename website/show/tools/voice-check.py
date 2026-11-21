# Per-sentence loudness of the processed tape, to catch quiet passages.

import json,subprocess,re
m=json.load(open(__import__('os').path.dirname(__file__)+'/../../public/show/audio/manifest.json'))
tk=m['voice']['tape']['takes'][0]
f=__import__('os').path.dirname(__file__)+'/../../public/show/audio/'+tk['file']
dur=tk['dur']
# per-sentence loudness: split by words ending in . ? 
ws=tk['words']; sents=[]; cur=[]
for w in ws:
    cur.append(w)
    if w['text'][-1] in '.?':
        sents.append(cur); cur=[]
for s in sents:
    a,b=s[0]['t']-0.05,s[-1]['end']+0.05
    r=subprocess.run(['ffmpeg','-hide_banner','-ss',str(a),'-to',str(b),'-i',f,'-af','ebur128','-f','null','-'],capture_output=True,text=True).stderr
    I=re.findall(r'I:\s+(-?[\d.]+) LUFS',r)[-1]
    print(round(a,2), I, ' '.join(w['text'] for w in s))
r=subprocess.run(['ffmpeg','-hide_banner','-i',f,'-af','ebur128','-f','null','-'],capture_output=True,text=True).stderr
print('integrated', re.findall(r'I:\s+(-?[\d.]+) LUFS',r)[-1], 'LRA', re.findall(r'LRA:\s+([\d.]+) LU',r)[-1], 'marks', tk['marks'], 'dur', dur)
