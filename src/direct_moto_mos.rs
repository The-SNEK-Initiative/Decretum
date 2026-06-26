// Motorola 6800 (8-bit, accumulator A+B, index X, 16-bit stack pointer)
// MOS 6501 (pins-compatible with 6800, same family but simpler, 6502 predecessor)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct M6800BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct M6800Builder;
pub struct Mos6501BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Mos6501Builder;

// 6800: 8-bit, 2 accumulators (A,B), index reg (X), cond code reg (CC)
impl M6800Builder{pub fn build_bin(p:&Program,out:&Path)->Result<M6800BuildOutput,String>{
    if p.target!="m6800"{return Err(format!("need 'm6800', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x8D,0]);continue}
        if t=="ret"||t=="rts"{bin.extend(&[0x39]);continue}
        if t=="hlt"||t=="wai"{bin.extend(&[0x3E]);continue}
        if let Some(r)=t.strip_prefix("if "){
            let _=r.trim();cf_counter+=1;
            bin.extend(&[0x81,0x00]); // CMPA #0
            let p=bin.len();bin.extend(&[0x27,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i16-prev as i16-2)as i8 as u8;bin[prev+1]=rel;
            let jmp_idx=bin.len();bin.extend(&[0x20,0]);
            f.br_indices.push(jmp_idx);
            let _=r.trim();bin.extend(&[0x81,0x00]);
            let p=bin.len();bin.extend(&[0x27,0]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i16-prev as i16-2)as i8 as u8;bin[prev+1]=rel;
            let jmp_idx=bin.len();bin.extend(&[0x20,0]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            for &idx in &f.br_indices{
                let rel=(bin.len() as i16-idx as i16-2)as i8 as u8;bin[idx+1]=rel;
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let _=r.trim();let sp=bin.len();cf_counter+=1;
            bin.extend(&[0x81,0x00]);
            let p=bin.len();bin.extend(&[0x27,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let rel=(f.start_pos as i16-bin.len() as i16-2)as i8 as u8;
            bin.extend(&[0x20,rel]);
            for &idx in &f.br_indices{
                let rel=(bin.len() as i16-idx as i16-2)as i8 as u8;bin[idx+1]=rel;
            }
            continue;
        }
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        let imm=args.get(0).map(|s|s.starts_with('#')).unwrap_or(false);
        let val=||ap(if imm{0}else{0});
        bin.extend(match m{
            "clra"=>vec![0x4F],"clrb"=>vec![0x5F],"clr"=>vec![0x6F,ap(0)],
            "inca"=>vec![0x4C],"incb"=>vec![0x5C],"inc"=>vec![0x6C,ap(0)],
            "deca"=>vec![0x4A],"decb"=>vec![0x5A],"dec"=>vec![0x6A,ap(0)],
            "tab"=>vec![0x16],"tba"=>vec![0x17],
            "ldaa"|"lda"=>{if imm{vec![0x86,ap(1)]}else{vec![0x86,ap(0)]}}
            "ldab"|"ldb"=>{if imm{vec![0xC6,ap(1)]}else{vec![0xC6,ap(0)]}}
            "staa"|"sta"=>vec![0x97,ap(0)],"stab"|"stb"=>vec![0xD7,ap(0)],
            "addd"=>vec![0xC3,ap(0),if args.len()>1{ap(1)}else{0}],
            "adda"|"add"=>{if imm{vec![0x8B,ap(1)]}else{vec![0x9B,ap(0)]}}
            "subb"=>vec![0xE0,ap(0)],"cmpa"=>vec![0x91,ap(0)],"cmpb"=>vec![0xD1,ap(0)],
            "anda"=>vec![0x84,ap(0)],"oraa"=>vec![0x8A,ap(0)],"eora"=>vec![0x88,ap(0)],
            "asla"=>vec![0x48],"asra"=>vec![0x47],"lsra"=>vec![0x44],"rola"=>vec![0x49],"rora"=>vec![0x46],
            "ldx"=>vec![0xCE,ap(0),ap(1)],"stx"=>vec![0xDF,ap(0)],
            "inx"=>vec![0x08],"dex"=>vec![0x09],
            "cpx"=>vec![0x8C,ap(0),ap(1)],
            "bne"=>vec![0x26,ap(0)],"beq"=>vec![0x27,ap(0)],
            "bgt"=>vec![0x2E,ap(0)],"blt"=>vec![0x2D,ap(0)],
            "bge"=>vec![0x2C,ap(0)],"ble"=>vec![0x2F,ap(0)],
            "bra"|"jmp"=>vec![0x20,ap(0)],"bsr"=>vec![0x8D,ap(0)],
            "jsr"=>vec![0xBD,ap(0),ap(1)],
            "nop"=>vec![0x01],"rts"=>vec![0x39],
            "sei"=>vec![0x0F],"cli"=>vec![0x0E],
            _=>return Err(format!("unknown m6800 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(M6800BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// MOS 6501: 8-bit, 2 accumulators, dedicated A/X/Y regs, like 6502 base but with external bus
impl Mos6501Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Mos6501BuildOutput,String>{
    if p.target!="mos6501"{return Err(format!("need 'mos6501', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x20,0,0]);continue}
        if t=="ret"||t=="rts"{bin.extend(&[0x60]);continue}
        if t=="hlt"{bin.extend(&[0x00]);continue}
        if let Some(r)=t.strip_prefix("if "){
            let _=r.trim();cf_counter+=1;
            bin.extend(&[0xC9,0x00]); // CMP #0
            let p=bin.len();bin.extend(&[0xF0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i16-prev as i16-2)as i8 as u8;bin[prev+1]=rel;
            let jmp_idx=bin.len();bin.extend(&[0x4C,0,0]);
            f.br_indices.push(jmp_idx);
            let _=r.trim();bin.extend(&[0xC9,0x00]);
            let p=bin.len();bin.extend(&[0xF0,0]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i16-prev as i16-2)as i8 as u8;bin[prev+1]=rel;
            let jmp_idx=bin.len();bin.extend(&[0x4C,0,0]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            for &idx in &f.br_indices{
                if bin[idx]==0xF0{
                    let rel=(bin.len() as i16-idx as i16-2)as i8 as u8;bin[idx+1]=rel;
                }else{
                    let addr=bin.len() as u16;let ba=addr.to_le_bytes();
                    bin[idx+1]=ba[0];bin[idx+2]=ba[1];
                }
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let _=r.trim();let sp=bin.len();cf_counter+=1;
            bin.extend(&[0xC9,0x00]);
            let p=bin.len();bin.extend(&[0xF0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let addr=f.start_pos as u16;let ba=addr.to_le_bytes();
            bin.extend(&[0x4C,ba[0],ba[1]]);
            for &idx in &f.br_indices{
                if bin[idx]==0xF0{
                    let rel=(bin.len() as i16-idx as i16-2)as i8 as u8;bin[idx+1]=rel;
                }else{
                    let addr=bin.len() as u16;let ba=addr.to_le_bytes();
                    bin[idx+1]=ba[0];bin[idx+2]=ba[1];
                }
            }
            continue;
        }
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        let imm=args.get(0).map(|s|s.starts_with('#')).unwrap_or(false);
        bin.extend(match m{
            "lda"=>{if imm{vec![0xA9,ap(1)]}else{vec![0xA5,ap(0)]}}
            "ldx"=>{if imm{vec![0xA2,ap(1)]}else{vec![0xA6,ap(0)]}}
            "ldy"=>{if imm{vec![0xA0,ap(1)]}else{vec![0xA4,ap(0)]}}
            "sta"=>vec![0x85,ap(0)],"stx"=>vec![0x86,ap(0)],"sty"=>vec![0x84,ap(0)],
            "tax"=>vec![0xAA],"tay"=>vec![0xA8],"txa"=>vec![0x8A],"tya"=>vec![0x98],
            "txs"=>vec![0x9A],"tsx"=>vec![0xBA],
            "bne"=>vec![0xD0,ap(0)],"beq"=>vec![0xF0,ap(0)],
            "bcc"=>vec![0x90,ap(0)],"bcs"=>vec![0xB0,ap(0)],
            "bpl"=>vec![0x10,ap(0)],"bmi"=>vec![0x30,ap(0)],
            "bvc"=>vec![0x50,ap(0)],"bvs"=>vec![0x70,ap(0)],
            "jmp"=>vec![0x4C,ap(0),ap(1)],"jsr"=>vec![0x20,ap(0),ap(1)],
            "nop"=>vec![0xEA],"rts"=>vec![0x60],"brk"=>vec![0x00],
            _=>return Err(format!("unknown mos6501 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Mos6501BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
