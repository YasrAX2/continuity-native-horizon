// Adapted from Horizon mesher.rs (MIT); copyright Horizon contributors.
// Full upstream license is in THIRD_PARTY_NOTICES.md.
use super::*;
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(mesher: &Mesher<'_>, pad: &Padded, tints: &mut Tints<'_>, out: &mut SectionMesh, pos:[i32;3], origin:[f32;3], face:&BakedFace, ao:bool, state:&Info) {
    let snapshot = crate::assets::continuity::snapshot();
    let id = pad.blocks[pi(pos[0],pos[1],pos[2])];
    let normal=match face.quad.facing.name() {"down"=>[0,-1,0],"up"=>[0,1,0],"north"=>[0,0,-1],"south"=>[0,0,1],"west"=>[-1,0,0],_=>[1,0,0]};
    let (right,up)=match normal {
        [0,0,-1] => ([-1,0,0],[0,1,0]), [0,0,1] => ([1,0,0],[0,1,0]),
        [-1,0,0] => ([0,0,1],[0,1,0]), [1,0,0] => ([0,0,-1],[0,1,0]),
        [0,1,0] => ([1,0,0],[0,0,-1]), _ => ([1,0,0],[0,0,1]),
    };
    let (right,up)=crate::continuity_core::face_basis(face.quad.positions,face.quad.uv,(right,up));
    let mut layer=face.layer;
    let mut opaque=face.opaque;
    let mut translucent=face.translucent;
    if let Some(data)=snapshot.as_ref() {
        let world=[tints.origin[0]*16+pos[0],tints.origin[2]+pos[1],tints.origin[1]*16+pos[2]];
        let center=mesher.assets.registry.state(BlockStateId(id));
        if let Some(center)=center {
            for rule in &data.rules {
                if !rule.matches(id,&face.quad.material.sprite,normal) {continue;}
                let matches=|offset:[i32;3]| {
                    let other=pad.blocks[pi(pos[0]+offset[0],pos[1]+offset[1],pos[2]+offset[2])];
                    match rule.connect.as_str() {
                        "state" => other==id,
                        "tile" => {
                            let mut models=Vec::new();
                            mesher.render.model_catalog.models_at(BlockStateId(other),BlockPos::new(world[0]+offset[0],world[1]+offset[1],world[2]+offset[2]),&mut models).is_ok()
                                && models.iter().flat_map(|m| &m.faces).any(|f| f.quad.facing==face.quad.facing && f.quad.material.sprite==face.quad.material.sprite)
                        },
                        _ => mesher.assets.registry.state(BlockStateId(other)).is_some_and(|s|s.name==center.name),
                    }
                };
                let neg=|v:[i32;3]|v.map(|n|-n);
                let add=|a:[i32;3],b:[i32;3]|std::array::from_fn(|i|a[i]+b[i]);
                let left=neg(right);let down=neg(up);
                let offsets=[left,right,up,down,add(left,up),add(right,up),add(left,down),add(right,down)];
                let mask=offsets.iter().enumerate().fold(0u8,|m,(i,&off)|m|((matches(off) as u8)<<i));
                let plane=[(0..3).map(|i|world[i]*right[i]).sum(),-(0..3).map(|i|world[i]*up[i]).sum::<i32>()];
                if let Some(index)=crate::continuity_core::select(&rule.method,mask,world,plane,rule.dimensions,rule.tiles.len()) {
                    match rule.tiles[index] {
                        Some(tile) => {layer=tile;opaque=data.opaque.get(layer as usize).copied().unwrap_or(false);translucent=data.translucent.get(layer as usize).copied().unwrap_or(false);},
                        None => continue,
                    }
                    break;
                }
            }
        }
    }
        let q = &face.quad;
        let tier = state.tier.max(hdr_tier("", q.light_emission));
        let shade_face = q.shade_direction_override.unwrap_or(q.facing);
        let directional = lighting::cardinal(mesher.nether_shading, shade_face);
        // Grey level per vertex before tinting, and vanilla light coords.
        let (gray, coords) = if ao {
            let info = &mesher.tables.info;
            let (brightness, coords) =
                lighting::ambient_occlusion(pad, info, state, pos, &q.positions, q.facing);
            let gray = brightness.map(|b| lighting::scale(lighting::gray(b), directional));
            (gray, coords)
        } else {
            let coords = match q.cull {
                Some(d) => lighting::coords(pad, state, lighting::step(pos, d)),
                None => lighting::flat(pad, state, pos, &q.positions, q.facing),
            };
            ([lighting::gray(directional); 4], [coords; 4])
        };
        let tint = Mesher::tint(state, q.tint_index, tints, pos);
        let target = if translucent {
            &mut out.translucent
        } else {
            &mut out.solid
        };
        for i in 0..4 {
            let g = gray[i] as u32;
            let rgb = match tint {
                Some(t) => [16, 8, 0].map(|shift| g * (t >> shift & 255) / 255),
                None => [g; 3],
            };
            let p = q.positions[i];
            target.push(TerrainVertex::new(
                [p[0] + origin[0], p[1] + origin[1], p[2] + origin[2]],
                q.uv[i],
                // Opaque leaves (forceOpaque) and quads over opaque texels draw in the
                // solid layer: no alpha cutout (a dirt path's side skips its clear top row).
                if opaque && !translucent || !mesher.cutout_leaves && state.leaves {
                    layer as u16 | OPAQUE_LAYER
                } else {
                    layer as u16
                },
                emitting(pack_rgb(rgb), tier),
                pack_coords(lighting::with_emission(coords[i], q.light_emission)),
            ));
        }
    if let Some(data)=snapshot.as_ref() && let Some(&emissive)=data.emissive.get(&layer) {
        let target=if translucent {&mut out.translucent} else {&mut out.solid};
        let normal=normal.map(|v|v as f32*0.0001);
        for i in 0..4 {
            let p=q.positions[i];
            target.push(TerrainVertex::new([p[0]+origin[0]+normal[0],p[1]+origin[1]+normal[1],p[2]+origin[2]+normal[2]],q.uv[i],emissive as u16,0xffffffff,pack_coords(0x00f000f0)));
        }
    }
}
