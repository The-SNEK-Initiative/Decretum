// OpenRISC + Nios II + MicroBlaze - FPGA soft-core RISC
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! fmake {
 ($B:ident, $O:ident, $T:expr) => {
  pub struct $B; pub struct $O{pub bin_path:PathBuf,pub bin_size:usize}
  impl $B{pub fn build_bin(p:&Program,out:&Path)->Result<$O,String>{
   if p.target!=$T{return Err(format!("need '{}', got '{}'",$T,p.target));}
   let k=fpga(p,$T)?;std::fs::write(out,&k).map_err(|e|e.to_string())?;
   Ok($O{bin_path:out.to_path_buf(),bin_size:k.len()})
  }}
 }
}
fmake!(DirectOpenriscBuilder,OpenriscBuildOutput,"openrisc");
fmake!(DirectNios2Builder,Nios2BuildOutput,"nios2");
fmake!(DirectMicroblazeBuilder,MicroblazeBuildOutput,"microblaze");

#[derive(Clone,Copy)]enum FReg{F0,F1,F2,F3,F4,F5,F6,F7,F8,F9,F10,F11,F12,F13,F14,F15,
 F16,F17,F18,F19,F20,F21,F22,F23,F24,F25,F26,F27,F28,F29,F30,F31}
fn frp(s:&str)->Option<FReg>{let s2=s.trim_start_matches('$').trim_start_matches('r').to_lowercase();let n=s2.parse::<u8>().ok()?;
 if n<=31{Some(match n{0=>FReg::F0,1=>FReg::F1,2=>FReg::F2,3=>FReg::F3,4=>FReg::F4,5=>FReg::F5,6=>FReg::F6,7=>FReg::F7,8=>FReg::F8,9=>FReg::F9,10=>FReg::F10,11=>FReg::F11,12=>FReg::F12,13=>FReg::F13,14=>FReg::F14,15=>FReg::F15,16=>FReg::F16,17=>FReg::F17,18=>FReg::F18,19=>FReg::F19,20=>FReg::F20,21=>FReg::F21,22=>FReg::F22,23=>FReg::F23,24=>FReg::F24,25=>FReg::F25,26=>FReg::F26,27=>FReg::F27,28=>FReg::F28,29=>FReg::F29,30=>FReg::F30,31=>FReg::F31,_=>return None})}else{None}}
fn frn(r:FReg)->u32{r as u32}fn fu4(u:u32)->Vec<u8>{u.to_le_bytes().to_vec()}

#[derive(Clone)]
enum FInst{
 Label(String),Bytes(Vec<u8>),Op3(FReg,FReg,FReg,u32,u32),Op2(FReg,FReg,u32,u32),Fret,
 CondBranch(FReg,String),Jump(String),
}

fn fenc(i:&FInst,arch:&str)->Result<Vec<u8>,String>{Ok(match i{
 FInst::Label(_)=>vec![],FInst::Bytes(b)=>b.clone(),
 FInst::Op3(rd,rs,rt,op,funct)=>{let d=frn(*rd);let s=frn(*rs);let t=frn(*rt);
  match arch{
   "openrisc"=>fu4(op|(d<<21)|(s<<16)|(t<<11)|funct),
   "nios2"=>fu4(op|(s<<21)|(t<<16)|(d<<11)|funct),
   "microblaze"=>fu4(op|(d<<21)|(s<<16)|(t<<11)|funct),
   _=>vec![0;4]
  }}
 FInst::Op2(rd,rs,imm,op)=>{let d=frn(*rd);let s=frn(*rs);
  match arch{
   "openrisc"=>fu4(op|(d<<21)|(s<<16)|(*imm&0xFFFF)),
   "nios2"=>fu4(op|(s<<21)|(d<<16)|(*imm&0xFFFF)),
   "microblaze"=>fu4(op|(d<<21)|(s<<16)|(*imm&0xFFFF)),
   _=>vec![0;4]
  }}
 FInst::Fret=>match arch{"openrisc"=>fu4(0x44000001),"nios2"=>fu4(0xE8080000),"microblaze"=>fu4(0x98087D60),_=>vec![0;4]},
 FInst::CondBranch(..)|FInst::Jump(_)=>vec![],
})}

fn fparse(t:&str,arch:&str)->Result<FInst,String>{
 let t=t.trim();if t.is_empty()||t.starts_with(';'){return Err("".into());}
 if t.ends_with(':'){return Ok(FInst::Label(t[..t.len()-1].to_string()));}
 let p:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
 if p.is_empty(){return Err("".into());}let m=p[0];let r=p[1..].join(" ");
 let v:Vec<&str>=r.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
 let gr=|s:&str|frp(s).ok_or_else(||format!("bad reg '{s}'"));
 let gim=|s:&str|s.parse::<u32>().map_err(|_|"bad imm".to_string());
 match m{
  "nop"=>Ok(FInst::Op2(FReg::F0,FReg::F0,0,0)),
  "ret"=>Ok(FInst::Fret),
  "li" if v.len()==2=>{let rd=gr(v[0])?;let imm=gim(v[1])?;
   let op=match arch{"openrisc"=>0x20<<26,"nios2"=>0x10<<26,"microblaze"=>0x30<<26,_=>0};
   Ok(FInst::Op2(rd,FReg::F0,imm,op))}
  "add"|"sub" if v.len()==3=>{let rd=gr(v[0])?;let rs=gr(v[1])?;let rt=gr(v[2])?;
   let(op,funct)=match(m,arch){
    ("add","openrisc")=>(0x38<<26,0x00),("sub","openrisc")=>(0x38<<26,0x02),
    ("add","nios2")=>(0x20<<26,0x04),("sub","nios2")=>(0x20<<26,0x06),
    ("add","microblaze")=>(0x00<<26,0x00),("sub","microblaze")=>(0x00<<26,0x01),
    _=>return Err("not impl".into())
   };Ok(FInst::Op3(rd,rs,rt,op,funct))}
  _=>Err(format!("unknown '{m}' for {arch}"))
 }
}

fn fpga(p:&Program,arch:&str)->Result<Vec<u8>,String>{
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

 let mut items:Vec<FInst>=Vec::new();
 items.push(FInst::Op2(FReg::F0,FReg::F0,0,0));items.push(FInst::Label(format!("__event_{}",p.entry_event)));
 for b in &p.blocks{
  let pr=match b.kind{BlockKind::Event=>"__event_",BlockKind::Proc=>"__proc_"};
  items.push(FInst::Label(format!("{}{}",pr,b.name)));
  for l in &b.lines{let t=l.trim();if t.is_empty()||t.starts_with(';'){continue;}
   if t.ends_with(':'){items.push(FInst::Label(format!("{}.{}",b.name,t[..t.len()-1].trim())));continue;}
   if let Some(x)=t.strip_prefix("emit "){items.push(FInst::Op2(FReg::F0,FReg::F0,0,0));items.push(FInst::Label(format!("__event_{}",x.trim())));continue;}
   if let Some(x)=t.strip_prefix("call "){items.push(FInst::Op2(FReg::F0,FReg::F0,0,0));items.push(FInst::Label(format!("__proc_{}",x.trim())));continue;}
   if t=="ret"{items.push(FInst::Fret);continue;}

   if let Some(cond_str) = t.strip_prefix("if ") {
    let reg = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let freg = match reg {0=>FReg::F0,1=>FReg::F1,2=>FReg::F2,3=>FReg::F3,4=>FReg::F4,5=>FReg::F5,6=>FReg::F6,7=>FReg::F7,8=>FReg::F8,9=>FReg::F9,10=>FReg::F10,11=>FReg::F11,12=>FReg::F12,13=>FReg::F13,14=>FReg::F14,15=>FReg::F15,16=>FReg::F16,17=>FReg::F17,18=>FReg::F18,19=>FReg::F19,20=>FReg::F20,21=>FReg::F21,22=>FReg::F22,23=>FReg::F23,24=>FReg::F24,25=>FReg::F25,26=>FReg::F26,27=>FReg::F27,28=>FReg::F28,29=>FReg::F29,30=>FReg::F30,31=>FReg::F31,_=>return Err("bad reg".into())};
    let endif_lbl = format!("__cf_{}_endif", cf_counter);
    let else_lbl = format!("__cf_{}_else", cf_counter);
    cf_counter += 1;
    let br_idx = items.len();
    items.push(FInst::CondBranch(freg, endif_lbl.clone()));
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
    if let FInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = elif_lbl.clone();
    }
    items.push(FInst::Jump(frame.endif_label.clone()));
    items.push(FInst::Label(elif_lbl));
    let reg = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let freg = match reg {0=>FReg::F0,1=>FReg::F1,2=>FReg::F2,3=>FReg::F3,4=>FReg::F4,5=>FReg::F5,6=>FReg::F6,7=>FReg::F7,8=>FReg::F8,9=>FReg::F9,10=>FReg::F10,11=>FReg::F11,12=>FReg::F12,13=>FReg::F13,14=>FReg::F14,15=>FReg::F15,16=>FReg::F16,17=>FReg::F17,18=>FReg::F18,19=>FReg::F19,20=>FReg::F20,21=>FReg::F21,22=>FReg::F22,23=>FReg::F23,24=>FReg::F24,25=>FReg::F25,26=>FReg::F26,27=>FReg::F27,28=>FReg::F28,29=>FReg::F29,30=>FReg::F30,31=>FReg::F31,_=>return Err("bad reg".into())};
    let br_idx = items.len();
    items.push(FInst::CondBranch(freg, frame.endif_label.clone()));
    frame.br_indices.push(br_idx);
    continue;
   }
   if t == "else" {
    let frame = cf_stack.last_mut().ok_or("else without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("else in while".to_string()); }
    if frame.has_else { return Err("duplicate else".to_string()); }
    frame.has_else = true;
    let prev = frame.br_indices.pop().ok_or("internal")?;
    if let FInst::CondBranch(_, ref mut lbl) = items[prev] {
     *lbl = frame.else_label.clone();
    }
    items.push(FInst::Jump(frame.endif_label.clone()));
    items.push(FInst::Label(frame.else_label.clone()));
    continue;
   }
   if t == "endif" {
    let frame = cf_stack.pop().ok_or("endif without if".to_string())?;
    if matches!(frame.kind, CfKind::While) { return Err("endif while expecting endwhile".to_string()); }
    items.push(FInst::Label(frame.endif_label.clone()));
    continue;
   }
   if let Some(cond_str) = t.strip_prefix("while ") {
    let reg = cond_str.trim().trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg".to_string())?;
    let freg = match reg {0=>FReg::F0,1=>FReg::F1,2=>FReg::F2,3=>FReg::F3,4=>FReg::F4,5=>FReg::F5,6=>FReg::F6,7=>FReg::F7,8=>FReg::F8,9=>FReg::F9,10=>FReg::F10,11=>FReg::F11,12=>FReg::F12,13=>FReg::F13,14=>FReg::F14,15=>FReg::F15,16=>FReg::F16,17=>FReg::F17,18=>FReg::F18,19=>FReg::F19,20=>FReg::F20,21=>FReg::F21,22=>FReg::F22,23=>FReg::F23,24=>FReg::F24,25=>FReg::F25,26=>FReg::F26,27=>FReg::F27,28=>FReg::F28,29=>FReg::F29,30=>FReg::F30,31=>FReg::F31,_=>return Err("bad reg".into())};
    let start_lbl = format!("__cf_{}_start", cf_counter);
    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
    cf_counter += 1;
    items.push(FInst::Label(start_lbl));
    let br_idx = items.len();
    items.push(FInst::CondBranch(freg, endwhile_lbl.clone()));
    cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
    continue;
   }
   if t == "endwhile" {
    let frame = cf_stack.pop().ok_or("endwhile without while".to_string())?;
    if !matches!(frame.kind, CfKind::While) { return Err("endwhile without matching while".to_string()); }
    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
    items.push(FInst::Jump(start_lbl));
    items.push(FInst::Label(frame.endif_label.clone()));
    continue;
   }

   match fparse(t,arch){Ok(i)=>items.push(i),Err(_)=>return Err(format!("line '{t}'"))}
  }
 }
 if !cf_stack.is_empty(){return Err("unclosed if/while block".into());}
 // Peephole: NOP compression + dead-code elimination
 {
     let br_ops = [0x30u32 << 26, 0x0Cu32 << 26, 0x0Eu32 << 26];
     crate::direct_peephole::peephole(&mut items,
         |i| matches!(i, FInst::Op2(FReg::F0,FReg::F0,0,0)),
         |i| match i { FInst::Op2(_,_,_,op) => br_ops.contains(op), FInst::CondBranch(..)|FInst::Jump(_) => true, _ => false },
         |i| matches!(i, FInst::Fret),
         |i| matches!(i, FInst::Label(_)),
     );
 }
 items.push(FInst::Label("__data".to_string()));
 for d in &p.data{match d{
  DataDecl::String{name,value}=>{let mut b=crate::direct_arch::expand_str(value);b.push(0);items.push(FInst::Label(format!("__data_{name}")));items.push(FInst::Bytes(b));}
  DataDecl::Scalar{name,width,value}=>{items.push(FInst::Label(format!("__data_{name}")));items.push(FInst::Bytes(match width{ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_le_bytes().to_vec(),ScalarWidth::Dword=>(*value as u32).to_le_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_le_bytes().to_vec(),}));}
  DataDecl::Buffer{name,size}=>{items.push(FInst::Label(format!("__data_{name}")));items.push(FInst::Bytes(vec![0u8;*size]));}
 }}
 let mut lm=BTreeMap::new();let mut o:u32=0;
 for i in &items{match i{
  FInst::Label(n)=>{lm.insert(n.clone(),o);},
  FInst::Bytes(b)=>o+=b.len()as u32,
  FInst::CondBranch(..)=>{o+=match arch{"openrisc"=>8,_=>4}},
  FInst::Jump(_)=>o+=4,
  _=>o+=4
 }}
 let mut bin=Vec::new();
 for i in &items{match i{
  FInst::Label(_)=>{}
  FInst::Bytes(b)=>bin.extend_from_slice(b),
  FInst::CondBranch(reg,label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "openrisc"=>{
     bin.extend(&fu4(0x10<<26|frn(*reg)<<16|frn(FReg::F0)<<11|0x1E));
     let disp=((target as i32-cur as i32-4)/4)as i32;
     if disp<0||disp>0x3FFFFFF{return Err("branch out of range".into());}
     bin.extend(&fu4(0x0C<<26|(disp as u32&0x3FFFFFF)));
    }
    "nios2"=>{
     let imm=((target as i32-cur as i32-4)/4)as i16;
     bin.extend(&fu4(0x1C<<26|frn(*reg)<<21|frn(FReg::F0)<<16|(imm as u16 as u32&0xFFFF)));
    }
    "microblaze"=>{
     let imm=((target as i32-cur as i32-4)/4)as i16;
     bin.extend(&fu4(0x26<<26|frn(*reg)<<16|(imm as u16 as u32&0xFFFF)));
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  FInst::Jump(label)=>{
   let target=*lm.get(label).ok_or_else(||format!("unknown label '{label}'"))?;
   let cur=bin.len()as u32;
   match arch{
    "openrisc"=>{
     let disp=((target as i32-cur as i32-4)/4)as i32;
     if disp<0||disp>0x3FFFFFF{return Err("branch out of range".into());}
     bin.extend(&fu4(0x30<<26|(disp as u32&0x3FFFFFF)));
    }
    "nios2"=>{
     let imm=((target as i32-cur as i32-4)/4)as i16;
     bin.extend(&fu4(0x1C<<26|(imm as u16 as u32&0xFFFF)));
    }
    "microblaze"=>{
     let imm=((target as i32-cur as i32-4)/4)as i16;
     bin.extend(&fu4(0x24<<26|(imm as u16 as u32&0xFFFF)));
    }
    _=>return Err("unsupported arch".into()),
   }
  }
  _=>bin.extend_from_slice(&fenc(i,arch)?)
 }}
 Ok(bin)
}
