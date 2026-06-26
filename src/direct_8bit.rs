// 6502 + Z80 + 6809 - classic 8-bit CPUs, each with distinct 1-byte encoding
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! e8make {
 ($B:ident, $O:ident, $T:expr) => {
  pub struct $B; pub struct $O{pub bin_path:PathBuf,pub bin_size:usize}
  impl $B{pub fn build_bin(p:&Program,out:&Path)->Result<$O,String>{
   if p.target!=$T{return Err(format!("need '{}', got '{}'",$T,p.target));}
   let k=e8(p,$T)?;std::fs::write(out,&k).map_err(|e|e.to_string())?;
   Ok($O{bin_path:out.to_path_buf(),bin_size:k.len()})
  }}
 }
}
e8make!(Direct6502Builder,SixFiveOhTwoBuildOutput,"6502");
e8make!(DirectZ80Builder,Z80BuildOutput,"z80");
e8make!(Direct6809Builder,Six809BuildOutput,"6809");

#[derive(Clone)]
enum E8Inst{Label(String),Bytes(Vec<u8>),OpR(u8,u8,u8),OpI(u8,u8),Ret,
    Branch(String,u8),Jmp(String)}

fn e8enc(i:&E8Inst,arch:&str,off:u32,lm:&BTreeMap<String,u32>) -> Result<Vec<u8>,String> {
    Ok(match i {
        E8Inst::Label(_) => vec![], E8Inst::Bytes(b) => b.clone(),
        E8Inst::OpR(mode,op1,op2) => {
            match arch {
                "6502" => vec![*op1 as u8],
                "z80" => match mode { 0 => vec![*op1 as u8], 1 => vec![0xCB, *op1 as u8], _ => vec![0xDD, *op1 as u8] },
                "6809" => vec![*op1 as u8, *op2 as u8],
                _ => vec![0],
            }
        }
        E8Inst::OpI(imm1,imm2) => {
            match arch {
                "6502" => vec![*imm1 as u8],
                "z80" => vec![*imm1 as u8],
                "6809" => vec![*imm1 as u8, *imm2 as u8],
                _ => vec![0],
            }
        }
        E8Inst::Branch(l,op) => {
            let tgt = *lm.get(l).ok_or("bad label")?;
            let rel = (tgt as i32).wrapping_sub(off as i32 + 2) as i8 as u8;
            vec![*op, rel]
        }
        E8Inst::Jmp(l) => {
            let tgt = *lm.get(l).ok_or("bad label")?;
            match arch {
                "6502" => vec![0x4C, (tgt&0xFF) as u8, ((tgt>>8)&0xFF) as u8],
                "z80" => vec![0xC3, (tgt&0xFF) as u8, ((tgt>>8)&0xFF) as u8],
                "6809" => vec![0x7E, ((tgt>>8)&0xFF) as u8, (tgt&0xFF) as u8],
                _ => vec![0;3],
            }
        }
        E8Inst::Ret => match arch {
            "6502" => vec![0x60], // RTS
            "z80" => vec![0xC9],  // RET
            "6809" => vec![0x39], // RTS
            _ => vec![0],
        },
    })
}

fn e8parse(t:&str,arch:&str)->Result<E8Inst,String>{
 let t=t.trim();if t.is_empty()||t.starts_with(';'){return Err("".into());}
 if t.ends_with(':'){return Ok(E8Inst::Label(t[..t.len()-1].to_string()));}
 let p:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
 if p.is_empty(){return Err("".into());}let m=p[0];let r=p[1..].join(" ");
 let v:Vec<&str>=r.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
 let gim=|s:&str|{let s2=s.trim_start_matches('#').trim_start_matches('$');s2.parse::<u8>().map_err(|_|"bad imm".to_string())};
 match m{
  "nop" => match arch {
   "6502" => Ok(E8Inst::OpI(0xEA,0)), "z80" => Ok(E8Inst::OpI(0x00,0)), "6809" => Ok(E8Inst::OpI(0x12,0)),
   _ => Err("bad".into())
  },
  "ret" => Ok(E8Inst::Ret),
  "lda"|"ld"|"ldx" if v.len()==2 => {
   let imm=gim(v[1])?;
   match arch {
    "6502" => Ok(E8Inst::OpI(if m=="ldx"{0xA2}else{0xA9},imm)),
    "z80" => Ok(E8Inst::OpR(0,0x3E,imm)), // LD A,imm
    "6809" => Ok(E8Inst::OpI(0x86,imm)), // LDA #imm
    _ => Err("bad".into())
   }
  }
  "add"|"adc"|"addd" if v.len()==2 => {
   let imm=gim(v[1])?;
   match arch {
    "6502" => Ok(E8Inst::OpI(0x69,imm)), // ADC #imm
    "z80" => Ok(E8Inst::OpR(0,0xC6,imm)), // ADD A,imm
    "6809" => Ok(E8Inst::OpI(if m=="addd"{0xC3}else{0x8B},imm)), // ADDD/ADCA
    _ => Err("bad".into())
   }
  }
  "sub"|"sbc"|"subd" if v.len()==2 => {
   let imm=gim(v[1])?;
   match arch {
    "6502" => Ok(E8Inst::OpI(0xE9,imm)), // SBC #imm
    "z80" => Ok(E8Inst::OpR(0,0xD6,imm)), // SUB imm
    "6809" => Ok(E8Inst::OpI(if m=="subd"{0x83}else{0x80},imm)), // SUBD/SUBA
    _ => Err("bad".into())
   }
  }
  "bne"|"bne " if v.len()==1 => {Ok(E8Inst::OpI(0xD0,0))} // stub with NOP
  "beq" if v.len()==1 => {Ok(E8Inst::OpI(0xF0,0))}
  "bra" if v.len()==1 => {Ok(E8Inst::OpI(match arch{"6809"=>0x20,"6502"=>0x4C,"z80"=>0x18,_=>0x18},0))}
  _ => Err(format!("unknown '{m}' for {arch}"))
 }
}

fn e8(p:&Program,arch:&str)->Result<Vec<u8>,String>{
 let mut items:Vec<E8Inst>=Vec::new();

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

 let (beq_op, bne_op) = match arch {
     "6502" => (0xF0u8, 0xD0u8),
     "z80" => (0x28u8, 0x20u8),
     "6809" => (0x27u8, 0x26u8),
     _ => (0, 0),
 };

 items.push(E8Inst::Label(format!("__event_{}",p.entry_event)));
 for b in &p.blocks{
  let pr=match b.kind{BlockKind::Event=>"__event_",BlockKind::Proc=>"__proc_"};
  items.push(E8Inst::Label(format!("{}{}",pr,b.name)));
  for l in &b.lines{let t=l.trim();if t.is_empty()||t.starts_with(';'){continue;}
   if t.ends_with(':'){items.push(E8Inst::Label(format!("{}.{}",b.name,t[..t.len()-1].trim())));continue;}
   if let Some(x)=t.strip_prefix("emit "){items.push(E8Inst::Label(format!("__event_{}",x.trim())));continue;}
   if let Some(x)=t.strip_prefix("call "){items.push(E8Inst::Label(format!("__proc_{}",x.trim())));continue;}
   if t=="ret"||t=="rts"{items.push(E8Inst::Ret);continue;}

   // if <reg>
   if let Some(cond_str) = t.strip_prefix("if ") {
       let _ = cond_str.trim();
       let endif_label = format!("__cf_{}_endif", cf_counter);
       let else_label = format!("__cf_{}_else", cf_counter);
       cf_counter += 1;
       let br_idx = items.len();
       items.push(E8Inst::Branch(endif_label.clone(), beq_op));
       cf_stack.push(CfFrame { kind: CfKind::If, endif_label, else_label, br_indices: vec![br_idx], has_else: false });
       continue;
   }
   // elif <reg>
   if let Some(cond_str) = t.strip_prefix("elif ") {
       let frame = cf_stack.last_mut().ok_or("elif without if")?;
       if frame.has_else { return Err("elif after else".into()); }
       let _ = cond_str.trim();
       let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.br_indices.len());
       cf_counter += 1;
       let prev = *frame.br_indices.last().ok_or("internal")?;
       if let E8Inst::Branch(ref mut l, _) = items[prev] { *l = elif_lbl.clone(); }
       items.push(E8Inst::Jmp(frame.endif_label.clone()));
       items.push(E8Inst::Label(elif_lbl));
       let br_idx = items.len();
       items.push(E8Inst::Branch(frame.endif_label.clone(), beq_op));
       frame.br_indices.push(br_idx);
       continue;
   }
   if t == "else" {
       let frame = cf_stack.last_mut().ok_or("else without if")?;
       if frame.has_else { return Err("duplicate else".into()); }
       frame.has_else = true;
       let prev = *frame.br_indices.last().ok_or("internal")?;
       if let E8Inst::Branch(ref mut l, _) = items[prev] { *l = frame.else_label.clone(); }
       items.push(E8Inst::Jmp(frame.endif_label.clone()));
       items.push(E8Inst::Label(frame.else_label.clone()));
       continue;
   }
   if t == "endif" {
       let frame = cf_stack.pop().ok_or("endif without if/while")?;
       match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
       items.push(E8Inst::Label(frame.endif_label.clone()));
       continue;
   }
   // while <reg>
   if let Some(cond_str) = t.strip_prefix("while ") {
       let _ = cond_str.trim();
       let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
       let start_lbl = format!("__cf_{}_start", cf_counter);
       cf_counter += 1;
       items.push(E8Inst::Label(start_lbl));
       let br_idx = items.len();
       items.push(E8Inst::Branch(endwhile_lbl.clone(), bne_op));
       cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
       continue;
   }
   if t == "endwhile" {
       let frame = cf_stack.pop().ok_or("endwhile without while")?;
       match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
       let start_lbl = frame.endif_label.replace("_endwhile", "_start");
       items.push(E8Inst::Jmp(start_lbl));
       items.push(E8Inst::Label(frame.endif_label.clone()));
       continue;
   }

   match e8parse(t,arch){Ok(i)=>items.push(i),Err(e)=>return Err(format!("line '{t}': {e}"))}
  }
 }

 if !cf_stack.is_empty() { return Err("unclosed if/while block".into()); }

 // Peephole
 crate::direct_peephole::peephole(&mut items,
     |i| matches!(i, E8Inst::OpI(0xEA,0)|E8Inst::OpI(0x00,0)),
     |i| matches!(i, E8Inst::Jmp(_)),
     |i| matches!(i, E8Inst::Ret),
     |i| matches!(i, E8Inst::Label(_)),
 );
 items.push(E8Inst::Label("__data".to_string()));
 for d in &p.data{match d{
  DataDecl::String{name,value}=>{let mut b=crate::direct_arch::expand_str(value);b.push(0);items.push(E8Inst::Label(format!("__data_{name}")));items.push(E8Inst::Bytes(b));}
  DataDecl::Scalar{name,width,value}=>{items.push(E8Inst::Label(format!("__data_{name}")));items.push(E8Inst::Bytes(match width{ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_le_bytes().to_vec(),ScalarWidth::Dword=>(*value as u32).to_le_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_le_bytes().to_vec(),}));}
  DataDecl::Buffer{name,size}=>{items.push(E8Inst::Label(format!("__data_{name}")));items.push(E8Inst::Bytes(vec![0u8;*size]));}
 }}
 let mut lm=BTreeMap::new();let mut o:u32=0;
 for i in &items{match i{E8Inst::Label(n)=>{lm.insert(n.clone(),o);},E8Inst::Bytes(b)=>o+=b.len()as u32,E8Inst::OpR(_,_,_)=>o+=if arch=="6809"{2}else{1},E8Inst::OpI(_,_)=>o+=if arch=="6809"{2}else{1},E8Inst::Branch(_,_)=>o+=2,E8Inst::Jmp(_)=>o+=3,E8Inst::Ret=>o+=1}}
 let mut bin=Vec::new();
 for i in &items{match i{E8Inst::Label(_)=>{}E8Inst::Bytes(b)=>bin.extend_from_slice(b),_=>bin.extend_from_slice(&e8enc(i,arch,bin.len()as u32,&lm)?)}}
 Ok(bin)
}
