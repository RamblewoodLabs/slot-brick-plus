#!/usr/bin/env python3
"""Make the textures render.py lays on each Game & Watch (and the like): from ES-DE's
media, the faceplate (physicalmedia) with its screen blank and with the game's screenshot
in it, and for a clamshell the outside of its lid. Writes <key>-blank.png, <key>-lit.png,
<key>-lid.png and <key>.json per game, the key being the game's row in devices.csv.

    python3 brick/gw-art/kit.py MEDIA KIT
        MEDIA  ES-DE's downloaded_media/gameandwatch (physicalmedia/, screenshots/)
        KIT    output directory

devices.csv holds what a picture does not say: which games are clamshells and where they
fold, whether the screen is lit from behind (dark until it is on), and where the screens are
on the faceplate (x0 y0 x1 y1 as fractions of it; two for a two-screen game).
Needs Pillow, numpy and OpenCV (opencv-python-headless).
"""
import csv,json,pathlib,sys
import numpy as np,cv2
from PIL import Image,ImageDraw,ImageFont
HERE=pathlib.Path(__file__).resolve().parent
MEDIA=pathlib.Path(sys.argv[1]);OUT=pathlib.Path(sys.argv[2]);OUT.mkdir(parents=True,exist_ok=True)
FONT=next((p for p in ('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf','/Library/Fonts/Arial Bold.ttf','/System/Library/Fonts/Supplemental/Arial Bold.ttf') if pathlib.Path(p).is_file()),None)
def font(size):
    return ImageFont.truetype(FONT,size) if FONT else ImageFont.load_default(size)
rows=list(csv.DictReader(open(HERE/'devices.csv',newline='')))
manifest=[]
for i,r in enumerate(rows):
    key=f'gw{i:02}';stem=r['file'][:-4]
    im=Image.open(MEDIA/'physicalmedia'/f'{stem}.png').convert('RGBA');W,H=im.size
    # Outline follows the device, rather than putting e.g. a round Tomytronic in a box.
    alpha=np.array(im)[...,3];contours,_=cv2.findContours((alpha>128).astype('uint8'),cv2.RETR_EXTERNAL,cv2.CHAIN_APPROX_SIMPLE)
    c=max(contours,key=cv2.contourArea);c=cv2.approxPolyDP(c,max(W,H)*.0015,True)[:,0,:]
    outline=[[round(float(x)/W,5),round(float(y)/H,5)] for x,y in c]
    data=np.array(im);border=np.concatenate([data[int(H*.03),:,:],data[-max(1,int(H*.03)),:,:],data[:,int(W*.03),:],data[:,-max(1,int(W*.03)),:]],axis=0)
    rgb=np.median(border[border[:,3]>128,:3],axis=0).astype(int).tolist()
    info=dict(kind=r['kind'],size=[W,H],hinge=float(r['hinge']) if r['hinge'] else None,body=rgb,outline=outline)
    shots=list((MEDIA/'screenshots').glob(stem+'.*'));shot=Image.open(shots[0]).convert('RGBA')
    blank=im.copy();lit=im.copy();rs=[tuple(map(float,part.split())) for part in r['rects'].split(';')]
    for j,rect in enumerate(rs):
        x0,y0,x1,y1=[round(a*b) for a,b in zip(rect,(W,H,W,H))];rw,rh=x1-x0,y1-y0
        grad=Image.new('RGBA',(rw,rh));px=np.zeros((rh,rw,4),dtype=np.uint8);px[...,3]=255
        if r['lcd']=='backlit':px[...,:3]=[14,15,18]
        else:
            v=np.linspace(1.025,.965,rh)[:,None,None]
            px[...,:3]=(np.ones((rh,rw,3))*[176,182,168]*v).astype('uint8')
        grad=Image.fromarray(px);shape=None
        if r.get('shape')=='oval':
            shape=Image.new('L',(rw,rh),0);ImageDraw.Draw(shape).ellipse((0,0,rw-1,rh-1),fill=255)
        blank.paste(grad,(x0,y0),shape)
        part=shot
        if len(rs)==2:
            sw,sh=shot.size
            if r['kind']=='clam_v':part=shot.crop((0,j*sh//2,sw,(j+1)*sh//2))
            else:part=shot.crop((j*sw//2,0,(j+1)*sw//2,sh))
        lit.paste(part.resize((rw,rh),Image.Resampling.LANCZOS),(x0,y0),shape)
    blank.save(OUT/f'{key}-blank.png');lit.save(OUT/f'{key}-lit.png')
    if r['kind']!='single':
        lw,lh=(W,round(H*info['hinge'])) if r['kind']=='clam_v' else (round(W*info['hinge']),H)
        lid=Image.new('RGBA',(lw,lh),tuple(rgb)+ (255,));d=ImageDraw.Draw(lid)
        rad=max(4,int(min(lw,lh)*.04));line=tuple(max(0,v-35) for v in rgb)+(255,)
        d.rounded_rectangle((lw*.035,lh*.055,lw*.965,lh*.95),radius=rad,outline=line,width=max(2,int(W*.006)))
        title=stem.split(' (')[0].upper();sz=int(min(lw*.07,lh*.14));f=font(sz)
        while d.textlength(title,font=f)>lw*.8:sz-=1;f=font(sz)
        tw=d.textlength(title,font=f);d.rounded_rectangle((lw*.5-tw*.5-14,lh*.32,lw*.5+tw*.5+14,lh*.51),radius=rad,fill=(28,28,28,255));d.text((lw*.5,lh*.35),title,anchor='mt',font=f,fill='white')
        d.rounded_rectangle((lw*.38,lh*.66,lw*.62,lh*.87),radius=3,fill=(205,210,212),outline=(70,70,70),width=2)
        f=font(int(lh*.065));d.multiline_text((lw*.5,lh*.68),'GAME\n&\nWATCH',anchor='ma',align='center',font=f,spacing=0,fill=(24,40,100))
        lid.save(OUT/f'{key}-lid.png')
    json.dump(info,open(OUT/f'{key}.json','w'))
    manifest.append(dict(key=key,file=r['file'],kind=r['kind'],lcd=r['lcd']))
json.dump(manifest,open(OUT/'manifest.json','w'),indent=1)
