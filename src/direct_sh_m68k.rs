// SH-2 + SH-4 + M68k - Hitachi SuperH (16-bit fixed) and Motorola 68k (variable CISC)
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! shmake {
 ($B:ident, $O:ident, $T:expr) => {
  pub struct $B; pub struct $O{pub bin_path:PathBuf,pub bin_size:usize}
  impl $B{pub fn build_bin(p:&Program,out:&Path)->Result<$O,String>{
   let ok = if stringify!($B) == "DirectSh2Builder" { p.target == "sh2" || p.target == "sh4" } else { p.target == $T };
   if !ok { return Err(format!("need '{}', got '{}'",$T,p.target)); }
   let k=sharch(p,$T)?;std::fs::write(out,&k).map_err(|e|e.to_string())?;
   Ok($O{bin_path:out.to_path_buf(),bin_size:k.len()})
  }}
 }
}
shmake!(DirectSh2Builder,Sh2BuildOutput,"sh2");
shmake!(DirectSh4Builder,Sh4BuildOutput,"sh4");
shmake!(DirectM68kBuilder,M68kBuildOutput,"m68k");

#[derive(Clone,Copy)]enum SReg{R0,R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14,R15}
fn srp(s:&str)->Option<SReg>{let n=s.trim_start_matches('r').trim_start_matches('d').trim_start_matches('a').parse::<u8>().ok()?;
 if n<=15{Some(match n{0=>SReg::R0,1=>SReg::R1,2=>SReg::R2,3=>SReg::R3,4=>SReg::R4,5=>SReg::R5,6=>SReg::R6,7=>SReg::R7,8=>SReg::R8,9=>SReg::R9,10=>SReg::R10,11=>SReg::R11,12=>SReg::R12,13=>SReg::R13,14=>SReg::R14,15=>SReg::R15,_=>return None})}else{None}}
fn srn(r:SReg)->u32{r as u32}

#[derive(Clone)]enum SInst{
 Label(String),Bytes(Vec<u8>),Su16(u16),Ret,
 CondBranch(u8,String),UncondBranch(String),
}

fn senc(i:&SInst,arch:&str)->Result<Vec<u8>,String>{Ok(match i{
 SInst::Label(_)=>vec![],SInst::Bytes(b)=>b.clone(),
 SInst::Su16(v)=>match arch{
  "sh2"|"sh4"=>v.to_be_bytes().to_vec(),
  "m68k"=>v.to_le_bytes().to_vec(),
  _=>vec![0,0]
 },
 SInst::Ret=>{
  let v:u16=match arch{"sh2"|"sh4"=>0x000B,"m68k"=>0x4E75,_=>0x0000};
  vec![(v>>8)as u8,(v&0xFF)as u8]
 }
 SInst::CondBranch(..)|SInst::UncondBranch(_)=>vec![],
})}

fn shparse(t:&str,arch:&str)->Result<SInst,String>{
 let t=t.trim();if t.is_empty()||t.starts_with(';'){return Err("".into());}
 if t.ends_with(':'){return Ok(SInst::Label(t[..t.len()-1].to_string()));}
 let p:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
 if p.is_empty(){return Err("".into());}let m=p[0];let r=p[1..].join(" ");
 let v:Vec<&str>=r.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
 let gim=|s:&str|{let s2=s.trim_start_matches('#').trim_start_matches('$');s2.parse::<u8>().map_err(|_|"bad imm".to_string())};
 match m{
  "nop"=>{
   let v:u16=match arch{"sh2"|"sh4"=>0x0009,"m68k"=>0x4E71,_=>0x0000};
   Ok(SInst::Su16(v))
  }
  "ret"|"rts"=>Ok(SInst::Ret),
  "mov" if v.len()==2=>{
   let imm=gim(v[1]).ok();
   match arch{
    "sh2"|"sh4"=>{
     if let Some(val)=imm{let d:u8=v[0].trim_start_matches('r').parse().unwrap_or(0);
      Ok(SInst::Su16(0xE000|((d as u16)<<8)|val as u16))
     }else{Ok(SInst::Su16(0x6000))} // MOV Rm,Rn
    }
    "m68k"=>{
     if let Some(val)=imm{let d=v[0].trim_start_matches('d').parse::<u8>().unwrap_or(0);
      Ok(SInst::Su16(0x303C|((d as u16)<<9)|val as u16)) // MOVEQ
     }else{Ok(SInst::Su16(0x2000))}
    }
    _=>Err("bad".into())
   }
  }
  "add"|"sub" if v.len()==2=>{
   let imm=gim(v[1]);
   match arch{
    "sh2"|"sh4"=>{
     if let Ok(val)=imm{let d:u8=v[0].trim_start_matches('r').parse().unwrap_or(0);
      Ok(SInst::Su16((if m=="add"{0x7000}else{0x8000})|((d as u16)<<8)|val as u16))
     }else{Ok(SInst::Su16(if m=="add"{0x300C}else{0x3008}))}
    }
    "m68k"=>{
     let d:u8=v[0].trim_start_matches('d').parse().unwrap_or(0);
     Ok(SInst::Su16(if m=="add"{0xD000|(d as u16)}else{0x9000|(d as u16)}))
    }
    _=>Err("bad".into())
   }
  }
  "bra" if v.len()==1=>{
   match arch{"sh2"|"sh4"=>Ok(SInst::Su16(0xA000)),// BRA disp12
    "m68k"=>Ok(SInst::Su16(0x6000)),_=>Err("bad".into())}
  }
  "bne"|"beq" if v.len()==1=>{
   let v:u16=match (m,arch){("beq","sh2")|("beq","sh4")=>0x8F00,("bne","sh2")|("bne","sh4")=>0x8E00,("beq","m68k")=>0x6700,("bne","m68k")=>0x6600,_=>0x0000};
   Ok(SInst::Su16(v))
  }
  _=>Err(format!("unknown '{m}' for {arch}"))
 }
}

fn sharch(p:&Program,arch:&str)->Result<Vec<u8>,String>{
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

 let mut items:Vec<SInst>=Vec::new();
 items.push(SInst::Label(format!("__event_{}",p.entry_event)));
 items.push(SInst::Ret);
 for b in &p.blocks{
  let pr=match b.kind{BlockKind::Event=>"__event_",BlockKind::Proc=>"__proc_"};
  items.push(SInst::Label(format!("{}{}",pr,b.name)));
  for l in &b.lines{let t=l.trim();if t.is_empty()||t.starts_with(';'){continue;}
   if t.ends_with(':'){items.push(SInst::Label(format!("{}.{}",b.name,t[..t.len()-1].trim())));continue;}
   if let Some(x)=t.strip_prefix("emit "){items.push(SInst::Label(format!("__event_{}",x.trim())));continue;}
   if let Some(x)=t.strip_prefix("call "){items.push(SInst::Label(format!("__proc_{}",x.trim())));continue;}
   if t=="ret"||t=="rts"{items.push(SInst::Ret);continue;}

   if let Some(cond_str) = t.strip_prefix("if ") {
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let endif_lbl = format!("__cf_{}_endif", cf_counter);
    let else_lbl = format!("__cf_{}_else", cf_counter);
    cf_counter += 1;
    let br_idx = items.len();
    items.push(SInst::CondBranch(reg, endif_lbl.clone()));
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
    if let SInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = elif_lbl.clone();
    }
    items.push(SInst::UncondBranch(frame.endif_label.clone()));
    items.push(SInst::Label(elif_lbl));
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let br_idx = items.len();
    items.push(SInst::CondBranch(reg, frame.endif_label.clone()));
    frame.br_indices.push(br_idx);
    continue;
   }
   if t == "else" {
    let frame = cf_stack.last_mut().ok_or("else without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("else in while".to_string()); }
    if frame.has_else { return Err("duplicate else".to_string()); }
    frame.has_else = true;
    let prev = frame.br_indices.pop().ok_or("internal")?;
    if let SInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = frame.else_label.clone();
    }
    items.push(SInst::UncondBranch(frame.endif_label.clone()));
    items.push(SInst::Label(frame.else_label.clone()));
    continue;
   }
   if t == "endif" {
    let frame = cf_stack.pop().ok_or("endif without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("endif while expecting endwhile".to_string()); }
    items.push(SInst::Label(frame.endif_label.clone()));
    continue;
   }
   if let Some(cond_str) = t.strip_prefix("while ") {
    let reg: u8 = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let start_lbl = format!("__cf_{}_start", cf_counter);
    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
    cf_counter += 1;
    items.push(SInst::Label(start_lbl));
    let br_idx = items.len();
    items.push(SInst::CondBranch(reg, endwhile_lbl.clone()));
    cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
    continue;
   }
   if t == "endwhile" {
    let frame = cf_stack.pop().ok_or("endwhile without while".to_string())?;
    if !matches!(frame.kind, CfKind::While) { return Err("endwhile without matching while".to_string()); }
    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
    items.push(SInst::UncondBranch(start_lbl));
    items.push(SInst::Label(frame.endif_label.clone()));
    continue;
   }

   match shparse(t,arch){Ok(i)=>items.push(i),Err(e)=>return Err(format!("line '{t}': {e}"))}
  }
 }
 if !cf_stack.is_empty(){return Err("unclosed if/while block".into());}
 // Peephole
 crate::direct_peephole::peephole(&mut items,
     |i| matches!(i, SInst::Su16(0x0009)),
     |i| matches!(i, SInst::Su16(0xA000|0x6000))||matches!(i, SInst::CondBranch(..))||matches!(i, SInst::UncondBranch(_)),
     |i| matches!(i, SInst::Ret),
     |i| matches!(i, SInst::Label(_)),
 );
 items.push(SInst::Label("__data".to_string()));
 for d in &p.data{match d{
  DataDecl::String{name,value}=>{let mut b=crate::direct_arch::expand_str(value);b.push(0);items.push(SInst::Label(format!("__data_{name}")));items.push(SInst::Bytes(b));}
  DataDecl::Scalar{name,width,value}=>{items.push(SInst::Label(format!("__data_{name}")));items.push(SInst::Bytes(match width{ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_be_bytes().to_vec(),ScalarWidth::Dword=>(*value as u32).to_be_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_be_bytes().to_vec(),}));}
  DataDecl::Buffer{name,size}=>{items.push(SInst::Label(format!("__data_{name}")));items.push(SInst::Bytes(vec![0u8;*size]));}
 }}
 let mut lm=BTreeMap::new();let mut o:u32=0;
 for i in &items{match i{
  SInst::Label(n)=>{lm.insert(n.clone(),o);},
  SInst::Bytes(b)=>o+=b.len()as u32,
  SInst::CondBranch(..)=>{o+=4}, // test(2) + cond br(2)
  SInst::UncondBranch(_)=>o+=2,
  _=>o+=2
 }}
 let mut bin=Vec::new();
 for i in &items{match i{
  SInst::Label(_)=>{}
  SInst::Bytes(b)=>bin.extend_from_slice(b),
  SInst::CondBranch(reg,label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "sh2"|"sh4"=>{
     let w=0x2008u16|((*reg as u16)<<8)|((*reg as u16)<<4);
     bin.extend(&w.to_be_bytes().to_vec());
     let disp=(target as i32-cur as i32-4)/2;
     if disp<0||disp>255{return Err("branch out of range for bt (0-255 words)".into());}
     bin.extend(&(0x8D00u16|(disp as u16&0xFF)).to_be_bytes().to_vec());
    }
    "m68k"=>{
     bin.extend(&(0x4A00u16|(*reg as u16&7)).to_le_bytes().to_vec());
     let disp=((target as i32-cur as i32-4)/2)as i8;
     bin.extend(&(0x6700u16|((disp as u8)as u16&0xFF)).to_le_bytes().to_vec());
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  SInst::UncondBranch(label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "sh2"|"sh4"=>{
     let disp=((target as i32-cur as i32-2)/2)as i16;
     if disp < -2048||disp>2047{return Err("branch out of range for bra".into());}
     bin.extend(&(0xA000u16|(disp as u16&0xFFF)).to_be_bytes().to_vec());
    }
    "m68k"=>{
     let disp=((target as i32-cur as i32-2)/2)as i8;
     bin.extend(&(0x6000u16|((disp as u8)as u16&0xFF)).to_le_bytes().to_vec());
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  _=>bin.extend_from_slice(&senc(i,arch)?)
 }}
 Ok(bin)
}
