# Renders one Game & Watch (or similar handheld) for Slot's shelf, from kit.py's textures.
#   blender -b -P brick/gw-art/render.py -- KEY KIT OUT
# One screen: OUT/00.png unlit, OUT/01.png lit. A clamshell: OUT/00..17.png, closed to
# open, its screens lighting over the last frames. A frame already there is kept, so a
# stopped run picks up where it was. pack.py turns these into the card's Labels/GW.
import bpy,bmesh,sys,os,math,json
from mathutils import Vector,Matrix
args=sys.argv[sys.argv.index('--')+1:]
key,K,out=args[0],args[1],args[2]
info=json.load(open(f'{K}/{key}.json'));W,H=info['size'];kind=info['kind']
S=1/100.0
bpy.ops.wm.read_factory_settings(use_empty=True)
sc=bpy.context.scene;sc.render.engine='CYCLES'
try:
 sc.cycles.device='GPU';bpy.context.preferences.addons['cycles'].preferences.compute_device_type='METAL'
 bpy.context.preferences.addons['cycles'].preferences.get_devices()
 for d in bpy.context.preferences.addons['cycles'].preferences.devices:d.use=True
except Exception as e:print('gpu',e)
sc.cycles.samples=24;sc.cycles.use_denoising=True;sc.render.film_transparent=True
sc.view_settings.view_transform='Standard'
w=bpy.data.worlds.new('w');sc.world=w;w.use_nodes=True;w.node_tree.nodes['Background'].inputs['Strength'].default_value=0.35
def sun(n,rot,e,ang):
 l=bpy.data.lights.new(n,'SUN');l.energy=e;l.angle=math.radians(ang);o=bpy.data.objects.new(n,l);sc.collection.objects.link(o);o.rotation_euler=[math.radians(a) for a in rot]
sun('key',(-40,-30,0),2.6,14);sun('fill',(-20,35,0),0.9,40);sun('rim',(60,0,180),0.6,30)
def img(path):
 return bpy.data.images.load(path,check_existing=True)
def body_mat(rgb,name):
 m=bpy.data.materials.new(name);m.use_nodes=True;b=m.node_tree.nodes['Principled BSDF']
 b.inputs['Base Color'].default_value=[ (c/255)**2.2 for c in rgb]+[1];b.inputs['Roughness'].default_value=0.42
 return m
def face_mat(name,a,b=None):
 m=bpy.data.materials.new(name);m.use_nodes=True;nt=m.node_tree;p=nt.nodes['Principled BSDF']
 p.inputs['Roughness'].default_value=0.32
 ta=nt.nodes.new('ShaderNodeTexImage');ta.image=img(a);ta.interpolation='Cubic'
 if b:
  tb=nt.nodes.new('ShaderNodeTexImage');tb.image=img(b);tb.interpolation='Cubic'
  mix=nt.nodes.new('ShaderNodeMix');mix.data_type='RGBA';mix.name='lightup'
  nt.links.new(ta.outputs['Color'],mix.inputs[6]);nt.links.new(tb.outputs['Color'],mix.inputs[7]);nt.links.new(mix.outputs[2],p.inputs['Base Color'])
 else: nt.links.new(ta.outputs['Color'],p.inputs['Base Color'])
 nt.links.new(ta.outputs['Alpha'],p.inputs['Alpha'])
 return m
def box(name,x0,y0,x1,y1,z0,z1,mat,r):
 bm=bmesh.new();bmesh.ops.create_cube(bm,size=1);me=bpy.data.meshes.new(name);bm.to_mesh(me)
 o=bpy.data.objects.new(name,me);sc.collection.objects.link(o)
 o.scale=((x1-x0),(y1-y0),(z1-z0));o.location=((x0+x1)/2,(y0+y1)/2,(z0+z1)/2)
 bpy.context.view_layer.objects.active=o;o.select_set(True);bpy.ops.object.transform_apply(scale=True,location=True);o.select_set(False)
 mod=o.modifiers.new('b','BEVEL');mod.width=r;mod.segments=6;mod.limit_method='ANGLE'
 o.data.materials.append(mat);bpy.ops.object.shade_auto_smooth() if False else None
 return o
def plane(name,x0,y0,x1,y1,z,mat,uv,flip=False):
 # uv: (u0,v0,u1,v1) image coords mapped to x0..x1, y0..y1 (v up)
 me=bpy.data.meshes.new(name);vs=[(x0,y0,z),(x1,y0,z),(x1,y1,z),(x0,y1,z)]
 fs=[(0,1,2,3)] if not flip else [(3,2,1,0)]
 me.from_pydata(vs,[],fs);uvl=me.uv_layers.new()
 u0,v0,u1,v1=uv;co={0:(u0,v0),1:(u1,v0),2:(u1,v1),3:(u0,v1)}
 for l in me.loops:uvl.data[l.index].uv=co[l.vertex_index]
 o=bpy.data.objects.new(name,me);sc.collection.objects.link(o);o.data.materials.append(mat);return o
body=body_mat(info['body'],'body');dark=body_mat([max(0,c-50) for c in info['body']],'edge')
silver=bpy.data.materials.new('silver');silver.use_nodes=True;sb=silver.node_tree.nodes['Principled BSDF'];sb.inputs['Metallic'].default_value=1;sb.inputs['Roughness'].default_value=0.25;sb.inputs['Base Color'].default_value=(0.8,0.8,0.82,1)
fm=face_mat('face',f'{K}/{key}-blank.png',f'{K}/{key}-lit.png')
Wd,Hd=W*S,H*S;D=max(Wd,Hd)*0.045;R=min(Wd,Hd)*0.035;e=0.004
hinge=None;parts=[]
if kind=='single':
 outline=info.get('outline')
 if outline:
  points=[((x-.5)*Wd,(.5-y)*Hd) for x,y in outline]
  n=len(points);vs=[(x,y,z) for z in (-2.2*D,0) for x,y in points]
  fs=[tuple(range(n-1,-1,-1)),tuple(range(n,n*2))]+[(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
  me=bpy.data.meshes.new('outline');me.from_pydata(vs,[],fs);me.update()
  o=bpy.data.objects.new('body',me);sc.collection.objects.link(o);o.data.materials.append(body);parts.append(o)
 else:parts.append(box('body',-Wd/2,-Hd/2,Wd/2,Hd/2,-2.2*D,0,body,R))
 parts.append(plane('front',-Wd/2,-Hd/2,Wd/2,Hd/2,e,fm,(0,0,1,1)))
else:
 lm=face_mat('lid',f'{K}/{key}-lid.png')
 hf=info['hinge'];piv=bpy.data.objects.new('hinge',None);sc.collection.objects.link(piv);hinge=piv
 if kind=='clam_v':
  ht=hf*Hd;hb=Hd-ht
  parts.append(box('bottom',-Wd/2,-hb,Wd/2,0,-D,0,body,R))
  parts.append(plane('bfront',-Wd/2,-hb,Wd/2,0,e,fm,(0,0,1,1-hf)))
  piv.location=(0,0,0)
  lid=[box('lid',-Wd/2,-ht,Wd/2,0,0,D,body,R),
       plane('lfront',-Wd/2,-ht,Wd/2,0,-e,fm,(0,1,1,1-hf),flip=True),
       plane('lback',-Wd/2,-ht,Wd/2,0,D+e,lm,(0,0,1,1))]
  # hinge spring bar along X at the fold
  bpy.ops.mesh.primitive_cylinder_add(radius=D*0.35,depth=Wd*0.55,location=(0,0,0),rotation=(0,math.pi/2,0));cyl=bpy.context.object;cyl.data.materials.append(silver);parts.append(cyl)
  axis=0
 else:
  hl=hf*Wd;hr=Wd-hl
  parts.append(box('right',0,-Hd/2,hr,Hd/2,-D,0,body,R))
  parts.append(plane('rfront',0,-Hd/2,hr,Hd/2,e,fm,(hf,0,1,1)))
  lid=[box('lid',0,-Hd/2,hl,Hd/2,0,D,body,R),
       plane('lfront',0,-Hd/2,hl,Hd/2,-e,fm,(hf,0,0,1),flip=True),
       plane('lback',0,-Hd/2,hl,Hd/2,D+e,lm,(0,0,1,1))]
  bpy.ops.mesh.primitive_cylinder_add(radius=D*0.35,depth=Hd*0.55,location=(0,0,0),rotation=(math.pi/2,0,0));cyl=bpy.context.object;cyl.data.materials.append(silver);parts.append(cyl)
  axis=1
 for o in lid:o.parent=piv
 parts+=lid
root=bpy.data.objects.new('root',None);sc.collection.objects.link(root)
for o in parts:
 if o.parent is None:o.parent=root
if hinge:hinge.parent=root
root.rotation_euler=(math.radians(-14),0,0)   # seen a little from above, like the shelf carts
cam=bpy.data.cameras.new('c');cam.type='ORTHO';co=bpy.data.objects.new('c',cam);sc.collection.objects.link(co);sc.camera=co;co.location=(0,0,30);cam.clip_end=100
span=max(Wd,Hd)*1.25;cam.ortho_scale=span
sc.render.resolution_x=sc.render.resolution_y=900
def set_state(open_u,lit_u):
 if hinge:
  a=-math.pi*open_u
  hinge.rotation_euler=(a,0,0) if axis==0 else (0,a,0)
 fm.node_tree.nodes['lightup'].inputs[0].default_value=lit_u
 # centre the visible device in frame
 bpy.context.view_layer.update()
 vs=[o.matrix_world@Vector(c) for o in parts if o.type=='MESH' for c in o.bound_box]
 cx=(min(v.x for v in vs)+max(v.x for v in vs))/2;cy=(min(v.y for v in vs)+max(v.y for v in vs))/2
 co.location=(cx,cy,30)
def render(path):
 sc.render.filepath=path;bpy.ops.render.render(write_still=True)

os.makedirs(out,exist_ok=True)
if kind=='single':
 for i,lit_u in enumerate((0,1)):
  path=f'{out}/{i:02}.png'
  if not os.path.isfile(path):set_state(0,lit_u);render(path)
else:
 for i in range(18):
  u=i/17;open_u=min(1,u/0.78);open_u=open_u*open_u*(3-2*open_u)
  light=max(0,(u-.75)/.25)
  path=f'{out}/{i:02}.png'
  if not os.path.isfile(path):set_state(open_u,light);render(path)
