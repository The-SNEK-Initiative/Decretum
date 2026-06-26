// PIC + AVR - 8-bit microcontrollers (RISC-like, 14/16-bit instruction words)
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! mcmake {
 ($B:ident, $O:ident, $T:expr) => {
  pub struct $B; pub struct $O{pub bin_path:PathBuf,pub bin_size:usize}
  impl $B{pub fn build_bin(p:&Program,out:&Path)->Result<$O,String>{
   if p.target!=$T{return Err(format!("need '{}', got '{}'",$T,p.target));}
   let k=mc(p,$T)?;std::fs::write(out,&k).map_err(|e|e.to_string())?;
   Ok($O{bin_path:out.to_path_buf(),bin_size:k.len()})
  }}
 }
}
mcmake!(DirectPICBuilder,PicBuildOutput,"pic");
mcmake!(DirectAvrBuilder,AvrBuildOutput,"avr");

#[derive(Clone)]
enum MCInst{
 Label(String),Bytes(Vec<u8>),Op2(u16),Op4(Vec<u8>),Ret,
 CondBranch(u8,String),UncondBranch(String),
}
fn mci2(v:u16)->Vec<u8>{v.to_le_bytes().to_vec()}
fn mci4(v:u32)->Vec<u8>{v.to_le_bytes().to_vec()}

fn mcenc(i:&MCInst,arch:&str)->Result<Vec<u8>,String>{Ok(match i{
 MCInst::Label(_)=>vec![],MCInst::Bytes(b)=>b.clone(),
 MCInst::Op2(w)=>match arch{
  "pic"=>vec![(*w>>8)as u8,(*w&0xFF)as u8],
  "avr"=>vec![(*w&0xFF)as u8,(*w>>8)as u8],
  _=>vec![0,0]
 },
 MCInst::Op4(b)=>b.clone(),
 MCInst::Ret=>match arch{"pic"=>vec![0x00,0x08],"avr"=>vec![0x08,0x95],"pic16"=>vec![0x00,0x08],_=>vec![0,0]},
 MCInst::CondBranch(..)|MCInst::UncondBranch(_)=>vec![],
})}

fn mcparse(t:&str,arch:&str)->Result<MCInst,String>{
 let t=t.trim();if t.is_empty()||t.starts_with(';'){return Err("".into());}
 if t.ends_with(':'){return Ok(MCInst::Label(t[..t.len()-1].to_string()));}
 let p:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
 if p.is_empty(){return Err("".into());}let m=p[0];let r=p[1..].join(" ");
 let v:Vec<&str>=r.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
 match m{
  "nop"=>match arch{"pic"=>Ok(MCInst::Op2(0x0000)),"avr"=>Ok(MCInst::Op2(0x0000)),_=>Err("bad".into())},
  "ret"=>Ok(MCInst::Ret),
  "mov"|"movwf" if v.len()==2=>{let d:u8=v[1].trim_start_matches('f').trim_start_matches('r').parse().unwrap_or(0);
   match arch{
    "pic"=>Ok(MCInst::Op2(0x0080|((d as u16)<<7))), // MOVWF
    "avr"=>Ok(MCInst::Op2(0x2E00|((d as u16)<<4))), // MOV
    _=>Err("bad".into())
   }}
  "add"|"addwf" if v.len()==2=>{let f:u16=v[1].trim_start_matches('f').parse().unwrap_or(0);
   match arch{"pic"=>Ok(MCInst::Op2(0x0700|(f&0x7F))),"avr"=>Ok(MCInst::Op2(0x0C00|(f&0xF))),_=>Err("bad".into())}}
  "sub"|"subwf" if v.len()==2=>{let f:u16=v[1].trim_start_matches('f').parse().unwrap_or(0);
   match arch{"pic"=>Ok(MCInst::Op2(0x0800|(f&0x7F))),"avr"=>Ok(MCInst::Op2(0x1800|(f&0xF))),_=>Err("bad".into())}}
  "bra"|"goto" if v.len()==1=>match arch{"pic"=>Ok(MCInst::Op2(0x2800)),"avr"=>Ok(MCInst::Op2(0xC000)),_=>Err("bad".into())}
  "bne" if v.len()==1=>match arch{"avr"=>Ok(MCInst::Op2(0xF401)),_=>Err("bad".into())}
  "beq" if v.len()==1=>match arch{"avr"=>Ok(MCInst::Op2(0xF001)),_=>Err("bad".into())}
  _=>Err(format!("unknown '{m}' for {arch}"))
 }
}

fn mc(p:&Program,arch:&str)->Result<Vec<u8>,String>{
 struct CfFrame {
  kind: CfKind,
  endif_label: String,
  else_label: String,
  br_indices: Vec<usize>,
  has_else: bool,
 }
 enum CfKind { If, While }
 let mut cf_stack: Vec<CfFrame> = Vec::new();
 let mut cf_counter: u32 = 0;

 let mut items:Vec<MCInst>=Vec::new();
 items.push(MCInst::Label(format!("__event_{}",p.entry_event)));
 items.push(MCInst::Ret);
 for b in &p.blocks{
  let pr=match b.kind{BlockKind::Event=>"__event_",BlockKind::Proc=>"__proc_"};
  items.push(MCInst::Label(format!("{}{}",pr,b.name)));
  for l in &b.lines{let t=l.trim();if t.is_empty()||t.starts_with(';'){continue;}
   if t.ends_with(':'){items.push(MCInst::Label(format!("{}.{}",b.name,t[..t.len()-1].trim())));continue;}
   if t=="ret"||t.starts_with("emit")||t.starts_with("call"){items.push(MCInst::Ret);continue;}

   if let Some(cond_str) = t.strip_prefix("if ") {
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let endif_lbl = format!("__cf_{}_endif", cf_counter);
    let else_lbl = format!("__cf_{}_else", cf_counter);
    cf_counter += 1;
    let br_idx = items.len();
    items.push(MCInst::CondBranch(reg, endif_lbl.clone()));
    cf_stack.push(CfFrame { kind: CfKind::If, endif_label: endif_lbl, else_label: else_lbl, br_indices: vec![br_idx], has_else: false });
    continue;
   }
   if let Some(cond_str) = t.strip_prefix("elif ") {
    let frame = cf_stack.last_mut().ok_or("elif without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("elif in while".to_string()); }
    if frame.has_else { return Err("elif after else".to_string()); }
    let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.br_indices.len());
    cf_counter += 1;
    let prev = frame.br_indices.pop().ok_or("internal")?;
    if let MCInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = elif_lbl.clone();
    }
    items.push(MCInst::UncondBranch(frame.endif_label.clone()));
    items.push(MCInst::Label(elif_lbl));
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let br_idx = items.len();
    items.push(MCInst::CondBranch(reg, frame.endif_label.clone()));
    frame.br_indices.push(br_idx);
    continue;
   }
   if t == "else" {
    let frame = cf_stack.last_mut().ok_or("else without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("else in while".to_string()); }
    if frame.has_else { return Err("duplicate else".to_string()); }
    frame.has_else = true;
    let prev = frame.br_indices.pop().ok_or("internal")?;
    if let MCInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = frame.else_label.clone();
    }
    items.push(MCInst::UncondBranch(frame.endif_label.clone()));
    items.push(MCInst::Label(frame.else_label.clone()));
    continue;
   }
   if t == "endif" {
    let frame = cf_stack.pop().ok_or("endif without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("endif while expecting endwhile".to_string()); }
    items.push(MCInst::Label(frame.endif_label.clone()));
    continue;
   }
   if let Some(cond_str) = t.strip_prefix("while ") {
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let start_lbl = format!("__cf_{}_start", cf_counter);
    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
    cf_counter += 1;
    items.push(MCInst::Label(start_lbl));
    let br_idx = items.len();
    items.push(MCInst::CondBranch(reg, endwhile_lbl.clone()));
    cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
    continue;
   }
   if t == "endwhile" {
    let frame = cf_stack.pop().ok_or("endwhile without while".to_string())?;
    if !matches!(frame.kind, CfKind::While) { return Err("endwhile without matching while".to_string()); }
    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
    items.push(MCInst::UncondBranch(start_lbl));
    items.push(MCInst::Label(frame.endif_label.clone()));
    continue;
   }

   match mcparse(t,arch){Ok(i)=>items.push(i),Err(e)=>return Err(format!("line '{t}': {e}"))}
  }
 }
 if !cf_stack.is_empty(){return Err("unclosed if/while block".into());}
 // Peephole
 crate::direct_peephole::peephole(&mut items,
     |i| matches!(i, MCInst::Op2(0)),
     |i| matches!(i, MCInst::Op2(0x2800|0xC000))||matches!(i, MCInst::CondBranch(..))||matches!(i, MCInst::UncondBranch(_)),
     |i| matches!(i, MCInst::Ret),
     |i| matches!(i, MCInst::Label(_)),
 );
 items.push(MCInst::Label("__data".to_string()));
 for d in &p.data{match d{
  DataDecl::String{name,value}=>{let mut b=crate::direct_arch::expand_str(value);b.push(0);items.push(MCInst::Label(format!("__data_{name}")));items.push(MCInst::Bytes(b));}
  DataDecl::Scalar{name,width,value}=>{items.push(MCInst::Label(format!("__data_{name}")));items.push(MCInst::Bytes(match width{ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_le_bytes().to_vec(),ScalarWidth::Dword=>(*value as u32).to_le_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_le_bytes().to_vec(),}));}
  DataDecl::Buffer{name,size}=>{items.push(MCInst::Label(format!("__data_{name}")));items.push(MCInst::Bytes(vec![0u8;*size]));}
 }}
 let mut lm=BTreeMap::new();let mut o:u32=0;
 for i in &items{match i{
  MCInst::Label(n)=>{lm.insert(n.clone(),o);},
  MCInst::Bytes(b)=>o+=b.len()as u32,
  MCInst::CondBranch(..)=>{o+=if arch=="pic"{6}else{4}},
  MCInst::UncondBranch(_)=>o+=2,
  _=>o+=2
 }}
 let mut bin=Vec::new();
 for i in &items{match i{
  MCInst::Label(_)=>{}
  MCInst::Bytes(b)=>bin.extend_from_slice(b),
  MCInst::CondBranch(reg,label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "pic"=>{
     bin.extend(&mci2(0x1000|(*reg as u16)&0x7F));
     bin.extend(&mci2(0x6403));
     let addr=(target/2)&0x7FF;
     bin.extend(&mci2(0x2800|addr as u16));
    }
    "avr"=>{
     bin.extend(&mci2(0x2000|(*reg as u16)*33));
     let k=((target as i32-cur as i32-4)/2)as i16;
     if k < -64||k>63{return Err("branch out of range".into());}
     bin.extend(&mci2(0xF001|((k as u16)&0x7F)<<3));
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  MCInst::UncondBranch(label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "pic"=>{
     let addr=(target/2)&0x7FF;
     bin.extend(&mci2(0x2800|addr as u16));
    }
    "avr"=>{
     let k=((target as i32-cur as i32-2)/2)as i16;
     if k < -2048||k>2047{return Err("branch out of range".into());}
     bin.extend(&mci2(0xC000|((k as u16)&0xFFF)));
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  _=>bin.extend_from_slice(&mcenc(i,arch)?)
 }}
 Ok(bin)
}
