use std::{collections::{BTreeMap,BTreeSet},path::{Path,PathBuf},sync::{Arc,OnceLock,RwLock}};
use crate::{sprites::Sprites,assets::Assets,continuity_core::{properties,expand_tiles}};
use horizon_world::registry::BlockStateId;

#[derive(Default)]
pub(crate) struct Data {
    pub rules:Vec<Rule>,
    pub emissive:BTreeMap<u32,u32>,
    pub opaque:Vec<bool>,
    pub translucent:Vec<bool>,
}
pub(crate) struct Rule {
    pub method:String, pub connect:String, pub dimensions:[usize;2],
    pub tiles:Vec<Option<u32>>, pub states:BTreeSet<u32>,
    pub textures:Vec<String>, pub faces:Vec<String>,
}
impl Rule {
    pub fn matches(&self,state:u32,texture:&str,normal:[i32;3])->bool {
        if !self.states.is_empty() && !self.states.contains(&state) {return false;}
        if !self.textures.is_empty() && !self.textures.iter().any(|t|t==texture) {return false;}
        let face=match normal {[0,1,0]=>"top",[0,-1,0]=>"bottom",[0,0,-1]=>"north",[0,0,1]=>"south",[1,0,0]=>"east",_=>"west"};
        self.faces.is_empty() || self.faces.iter().any(|f|f==face||f=="all"||f=="sides"&&normal[1]==0)
    }
}
static DATA:OnceLock<RwLock<Option<Arc<Data>>>>=OnceLock::new();
pub(crate) fn snapshot()->Option<Arc<Data>> { DATA.get_or_init(Default::default).read().ok()?.clone() }

fn walk(path:&Path,out:&mut Vec<PathBuf>) {
    if let Ok(entries)=std::fs::read_dir(path) {for e in entries.flatten() {
        let p=e.path();
        if e.file_type().is_ok_and(|t|t.is_dir()) {walk(&p,out);}
        else if p.extension().is_some_and(|x|x=="properties") {out.push(p);}
    }}
}
fn qualified(s:&str)->String {if s.contains(':') {s.into()} else {format!("minecraft:{s}")}}
fn match_texture(s:&str)->String {
    let s=s.trim_end_matches(".png");
    let (ns,p)=s.split_once(':').unwrap_or(("minecraft",s));
    let p=p.strip_prefix("textures/").unwrap_or(p);
    if p.contains('/') {format!("{ns}:{p}")} else {format!("{ns}:block/{p}")}
}
fn tile_name(root:&Path,file:&Path,value:&str)->Option<String> {
    let value=value.trim_end_matches(".png");
    if value.starts_with('<') {return None;}
    let rel=file.strip_prefix(root.join("assets")).ok()?;
    let ns=rel.components().next()?.as_os_str().to_str()?;
    let path=if value.contains(':') {let (ns,p)=value.split_once(':')?; PathBuf::from(format!("{ns}/{}",p.trim_start_matches('/')))}
        else if value.starts_with("~/") {PathBuf::from(format!("{ns}/optifine/{}",value.trim_start_matches("~/")))}
        else if value.starts_with("/assets/") {PathBuf::from(value.trim_start_matches("/assets/"))}
        else if value.starts_with("optifine/")||value.starts_with("textures/") {PathBuf::from(format!("{ns}/{value}"))}
        else {rel.parent()?.join(value.trim_start_matches("./"))};
    let mut parts=Vec::new();
    for c in path.components() {
        match c { std::path::Component::Normal(p)=>parts.push(p.to_str()?.to_owned()),std::path::Component::ParentDir=>{if parts.len()<=1{return None;}parts.pop();},std::path::Component::CurDir=>{},_=>return None }
    }
    let ns=parts.first()?;let p=parts[1..].join("/");
    Some(format!("{ns}:{p}"))
}
fn insert(root:&Path,sprites:&mut Sprites,name:&str)->Option<u32> {
    let (ns,p)=name.split_once(':')?;
    if !root.join(format!("assets/{ns}/{p}.png")).is_file() {return None;}
    match crate::sprites::continuity_decode::insert(sprites,root,name) {
        Ok(layer) if layer<32768 =>Some(layer),
        Ok(_)=>{log::warn!("Continuity Native: sprite array limit reached");None},
        Err(e)=>{log::warn!("Continuity Native: {name}: {e}");None},
    }
}

pub(crate) fn load(root:&Path,sprites:&mut Sprites,registry:&horizon_world::registry::BlockRegistry) {
    let mut files=Vec::new();
    if let Ok(namespaces)=std::fs::read_dir(root.join("assets")) {
        for ns in namespaces.flatten() {walk(&ns.path().join("optifine/ctm"),&mut files);}
    }
    files.sort();
    // Tile rules precede block rules; weight and filename settle order.
    let mut configs:Vec<_>=files.into_iter().filter_map(|file| {
        let text=std::fs::read_to_string(&file).ok()?;
        let p=properties(&text);Some((file,p))
    }).collect();
    configs.sort_by_key(|(file,p)|(!p.contains_key("matchTiles"),-p.get("weight").and_then(|v|v.parse::<i64>().ok()).unwrap_or(0),file.clone()));
    let mut data=Data::default();
    for (file,mut p) in configs {
        let method=p.get("method").map(String::as_str).unwrap_or("ctm").to_owned();
        if !["ctm","horizontal","vertical","horizontal+vertical","vertical+horizontal","top","fixed","random","repeat"].contains(&method.as_str()) {
            log::warn!("Continuity Native: unsupported method {method} in {}",file.display());continue;
        }
        // Fail visibly rather than apply a rule to blocks outside its restrictions.
        if ["biomes","heights","minHeight","maxHeight","name","weights","linked","symmetry","randomLoops"].iter().any(|k|p.contains_key(*k)) || p.get("innerSeams").is_some_and(|v|v=="true") {
            log::warn!("Continuity Native: unsupported restriction in {}",file.display());continue;
        }
        if !p.contains_key("matchTiles")&&!p.contains_key("matchBlocks") {
            let stem=file.file_stem().unwrap_or_default().to_string_lossy();
            if let Some(block)=stem.strip_prefix("block_") {p.insert("matchBlocks".into(),block.into());}
            else {p.insert("matchTiles".into(),stem.into());}
        }
        let mut states=BTreeSet::new();
        if let Some(blocks)=p.get("matchBlocks") {
            for pattern in blocks.split_whitespace() {
                let bits:Vec<_>=pattern.split(':').collect();
                let (name,filters)=if bits.len()>1&&!bits[1].contains('=') {(format!("{}:{}",bits[0],bits[1]),&bits[2..])} else {(qualified(bits[0]),&bits[1..])};
                for i in 0..registry.len() {
                    let id=BlockStateId(i as u32);let Some(s)=registry.state(id) else{continue};
                    if s.name==name && filters.iter().all(|f|f.split_once('=').is_some_and(|(k,vs)|s.properties.get(k).is_some_and(|v|vs.split(',').any(|a|a==v)))) {states.insert(id.0);}
                }
            }
            if states.is_empty(){continue;}
        }
        let textures=p.get("matchTiles").map(|s|s.split_whitespace().map(match_texture).collect()).unwrap_or_default();
        let tokens=expand_tiles(p.get("tiles").map(String::as_str).unwrap_or(""));
        if tokens.is_empty(){continue;}
        let tiles:Vec<_>=tokens.iter().map(|t|tile_name(root,&file,t).and_then(|n|insert(root,sprites,&n))).collect();
        if tiles.iter().all(Option::is_none){continue;}
        let connect=p.get("connect").cloned().unwrap_or_else(||if p.contains_key("matchTiles"){"tile".into()}else{"block".into()});
        let dimensions=["width","height"].map(|k|p.get(k).and_then(|s|s.parse().ok()).unwrap_or(0));
        let faces=p.get("faces").map(|s|s.split_whitespace().map(str::to_owned).collect()).unwrap_or_default();
        data.rules.push(Rule{method,connect,dimensions,tiles,states,textures,faces});
    }
    let cfg=std::fs::read_to_string(root.join("assets/minecraft/optifine/emissive.properties")).ok();
    if let Some(text)=cfg {
        let cfg=properties(&text);
        if let Some(suffix)=cfg.get("suffix.emissive").filter(|s|!s.is_empty()&&!s.contains(['/', '\\'])) {
            let layers=sprites.layers.clone();
            for (name,layer) in layers {
                let Some((ns,p))=name.split_once(':') else {continue};
                let p=p.strip_prefix("textures/").unwrap_or(p);
                let path=if p.starts_with("optifine/"){p.to_owned()}else{format!("textures/{p}")};
                if let Some(e)=insert(root,sprites,&format!("{ns}:{path}{suffix}")){data.emissive.insert(layer,e);}
            }
        }
    }
    data.opaque=sprites.opaque.clone();data.translucent=sprites.translucent.clone();
    log::info!("Continuity Native: loaded {} CTM rules and {} emissive layers",data.rules.len(),data.emissive.len());
    if let Ok(mut slot)=DATA.get_or_init(Default::default).write(){*slot=Some(Arc::new(data));}
}

thread_local! { static PENDING: std::cell::RefCell<Option<(PathBuf, &'static horizon_world::registry::BlockRegistry)>> = const { std::cell::RefCell::new(None) }; }
pub(crate) fn configure(root:&Path, assets:&Assets) { PENDING.with(|p| *p.borrow_mut()=Some((root.to_owned(), assets.registry))); }
pub(crate) fn finish(sprites:&mut Sprites) { let pending=PENDING.with(|p|p.borrow_mut().take());if let Some((root,registry))=pending {load(&root,sprites,registry);} }
