// Original Rust implementation of the documented connected-texture format.
use std::collections::BTreeMap;

pub fn properties(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut joined = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with(['#', '!']) || line.is_empty() { continue; }
        joined.push_str(line.trim_end_matches('\\'));
        if line.ends_with('\\') { continue; }
        let split = joined.find(['=', ':', ' ', '\t']);
        if let Some(i) = split {
            out.insert(joined[..i].to_owned(), joined[i..].trim_start_matches(['=', ':', ' ', '\t']).trim().to_owned());
        }
        joined.clear();
    }
    out
}

pub fn expand_tiles(text: &str) -> Vec<String> {
    text.split_whitespace().flat_map(|word| {
        if let Some((a,b)) = word.split_once('-')
            && let (Ok(a), Ok(b)) = (a.parse::<u16>(), b.parse::<u16>())
            && b >= a && b-a < 4096 {
            return (a..=b).map(|i| i.to_string()).collect::<Vec<_>>();
        }
        vec![word.to_owned()]
    }).collect()
}

/// L,R,U,D,UL,UR,DL,DR. A corner matters only when both bordering edges join.
pub fn canonical_mask(mut m: u8) -> u8 {
    for (corner,a,b) in [(4,0,2),(5,1,2),(6,0,3),(7,1,3)] {
        if m & (1<<a) == 0 || m & (1<<b) == 0 { m &= !(1<<corner); }
    }
    m
}

// The standard 47-tile layout, expressed as connectivity rather than artwork.
// Measured from the borders of the supplied mod's resource-pack glass tiles;
// no Java code or texture pixels are included in this implementation.
pub const CTM_MASKS: [u8;47] = [0,2,3,1,10,9,14,11,47,143,95,63,8,138,203,73,6,5,7,13,31,79,207,175,12,174,255,93,142,75,46,139,127,191,111,159,4,38,55,21,39,29,23,77,223,239,15];
pub fn ctm_index(mask:u8) -> usize {
    CTM_MASKS.iter().position(|&m| m == canonical_mask(mask)).expect("all 47 canonical masks are present")
}

pub fn linear_index(first: bool, second: bool) -> usize {
    match (first,second) { (false,false)=>3, (true,false)=>2, (false,true)=>0, (true,true)=>1 }
}

/// Match adjacency to the baked texture's axes, including 90-degree model/UV rotations.
pub fn face_basis(p:[[f32;3];4],uv:[[f32;2];4],fallback:([i32;3],[i32;3]))->([i32;3],[i32;3]) {
    let a=[uv[1][0]-uv[0][0],uv[1][1]-uv[0][1]];
    let b=[uv[2][0]-uv[0][0],uv[2][1]-uv[0][1]];
    let det=a[0]*b[1]-b[0]*a[1];
    if det.abs()<0.00001 {return fallback;}
    let d1:[f32;3]=std::array::from_fn(|i|p[1][i]-p[0][i]);
    let d2:[f32;3]=std::array::from_fn(|i|p[2][i]-p[0][i]);
    let u=std::array::from_fn(|i|(d1[i]*b[1]-d2[i]*a[1])/det);
    let up=std::array::from_fn(|i|-(d2[i]*a[0]-d1[i]*b[0])/det);
    fn axis(v:[f32;3])->Option<[i32;3]> {
        let i=(0..3).max_by(|&a,&b|v[a].abs().total_cmp(&v[b].abs()))?;
        if !v[i].is_finite() || v[i].abs()<0.00001 || (0..3).any(|j|j!=i&&v[j].abs()>v[i].abs()*0.001) {return None;}
        let mut out=[0;3];out[i]=if v[i]>0.0{1}else{-1};Some(out)
    }
    match (axis(u),axis(up)) {(Some(u),Some(up))=>(u,up),_=>fallback}
}

pub fn select(method:&str, mask:u8, pos:[i32;3], plane:[i32;2], dimensions:[usize;2], count:usize) -> Option<usize> {
    if count == 0 { return None; }
    let l=mask&1!=0; let r=mask&2!=0; let u=mask&4!=0; let d=mask&8!=0;
    let idx = match method {
        "ctm" => ctm_index(mask),
        "horizontal" => linear_index(l,r),
        "vertical" => linear_index(d,u),
        "horizontal+vertical" => if l||r {linear_index(l,r)} else if u&&d {4} else if u {5} else if d {6} else {3},
        "vertical+horizontal" => if u||d {linear_index(d,u)} else if l&&r {4} else if r {5} else if l {6} else {3},
        "top" => if u {0} else {return None},
        "fixed" => 0,
        "repeat" => {
            let [w,h] = dimensions;
            if w==0 || h==0 || w.checked_mul(h) != Some(count) { return None; }
            plane[1].rem_euclid(h as i32) as usize*w + plane[0].rem_euclid(w as i32) as usize
        },
        "random" => {
            let mut n=(pos[0] as u64).wrapping_mul(0x9e3779b185ebca87) ^ (pos[1] as u64).wrapping_mul(0xc2b2ae3d27d4eb4f) ^ (pos[2] as u64).wrapping_mul(0x165667b19e3779f9);
            n ^= n>>30; n=n.wrapping_mul(0xbf58476d1ce4e5b9); n ^= n>>27;
            n as usize % count
        },
        _ => return None,
    };
    (idx<count).then_some(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn ctm_covers_every_neighbourhood() {
        let mut seen=std::collections::BTreeSet::new();
        for m in 0..=255 {let i=ctm_index(m); assert_eq!(CTM_MASKS[i],canonical_mask(m));seen.insert(i);}
        assert_eq!(seen.len(),47);
        assert_eq!(ctm_index(0),0);assert_eq!(ctm_index(255),26);assert_eq!(ctm_index(15),46);
    }
    #[test] fn isolated_corners_do_not_connect() {assert_eq!(ctm_index(240),0);}
    #[test] fn negative_repeat_wraps() {assert_eq!(select("repeat",0,[0;3],[-1,-1],[3,2],6),Some(5));}
    #[test] fn malformed_repeat_fails() {assert_eq!(select("repeat",0,[0;3],[0;2],[0,2],6),None);}
    #[test] fn property_comments_and_continuation() {let p=properties("# x\nmethod=ctm\ntiles=0-\\\n46\n");assert_eq!(p["tiles"],"0-46");}
    #[test] fn tile_ranges_preserve_named_entries() {assert_eq!(expand_tiles("0-2 test.png <skip>"),["0","1","2","test.png","<skip>"]);}
    #[test] fn rotated_uv_changes_connection_axes() {
        let p=[[1.,1.,0.],[1.,0.,0.],[0.,0.,0.],[0.,1.,0.]];
        let fallback=([-1,0,0],[0,1,0]);
        assert_eq!(face_basis(p,[[0.,0.],[0.,1.],[1.,1.],[1.,0.]],fallback),fallback);
        assert_eq!(face_basis(p,[[0.,1.],[1.,1.],[1.,0.],[0.,0.]],fallback),([0,-1,0],[-1,0,0]));
    }
}
