// UNIVAC (36-bit ones' complement, 6-bit Fieldata chars, 8 I/O channels)
// CDC 6600 (60-bit, 8 address + 8 operand registers, scoreboarding, peripheral processors)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct UnivacBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct UnivacBuilder;
pub struct Cdc6600BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Cdc6600Builder;

fn write_36(v:u64)->Vec<u8>{let mut b=vec![0u8;5];b[0]=(v>>32)as u8&0x0F;b[1]=(v>>24)as u8;b[2]=(v>>16)as u8;b[3]=(v>>8)as u8;b[4]=v as u8;b}
fn write_60(v:u64)->Vec<u8>{let mut b=vec![0u8;8];b[0]=(v>>56)as u8&0x0F;b[1]=(v>>48)as u8;b[2]=(v>>40)as u8;b[3]=(v>>32)as u8;b[4]=(v>>24)as u8;b[5]=(v>>16)as u8;b[6]=(v>>8)as u8;b[7]=v as u8;b}

// UNIVAC: 4-bit opcode, 8-bit address, 4-index, 4-mod = 20 bits per instruction (in 36-bit word)
fn uenc(op:u8,addr:u8,idx:u8,modf:u8)->Vec<u8>{write_36(((op as u64)<<16)|((addr as u64)<<8)|((idx as u64)<<4)|(modf as u64))}

impl UnivacBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<UnivacBuildOutput,String>{
    if p.target!="univac"{return Err(format!("need 'univac', got '{}'",p.target))}
    #[derive(Clone)]
    enum CfKind{If,While}
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter=0;
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim(); if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();bin.extend(uenc(0x0D,0,reg,0));let pos=bin.len()-5;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=(bin.len()/5)as u8;let idx=(bin[last+4]>>4)&0xF;bin[last..last+5].copy_from_slice(&uenc(0x0D,here,idx,0));let bra_pos=bin.len();bin.extend(uenc(0x0B,0,0,0));frame.br_indices.push(bra_pos);bin.extend(uenc(0x0D,0,reg,0));let beq_pos=bin.len()-5;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=(bin.len()/5)as u8;let idx=(bin[last+4]>>4)&0xF;bin[last..last+5].copy_from_slice(&uenc(0x0D,here,idx,0));let bra_pos=bin.len();bin.extend(uenc(0x0B,0,0,0));frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=(bin.len()/5)as u8;for &pos in &frame.br_indices{if bin[pos+2]==0x0D{let idx=(bin[pos+4]>>4)&0xF;bin[pos..pos+5].copy_from_slice(&uenc(0x0D,target,idx,0))}else{bin[pos..pos+5].copy_from_slice(&uenc(0x0B,target,0,0))}}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();bin.extend(uenc(0x0D,0,reg,0));let pos=bin.len()-5;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=(frame.start_pos/5)as u8;bin.extend(uenc(0x0B,back,0,0));let here=(bin.len()/5)as u8;for &pos in &frame.br_indices{let idx=(bin[pos+4]>>4)&0xF;bin[pos..pos+5].copy_from_slice(&uenc(0x0D,here,idx,0))}continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(uenc(0x0B,0,0,0));continue}
        if t=="ret"||t=="hlt"{bin.extend(uenc(0x00,0,0,0));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue} let m=parts[0];
        let joined=parts[1..].join(" "); let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "t"|"jmp"=>uenc(0x0B,ap(0),ap(1),ap(2)),
            "tmi"=>uenc(0x0F,ap(0),ap(1),ap(2)),
            "a"|"add"=>uenc(0x04,ap(0),ap(1),ap(2)),
            "s"|"sub"=>uenc(0x05,ap(0),ap(1),ap(2)),
            "b"|"load"=>uenc(0x06,ap(0),ap(1),ap(2)),
            "u"|"store"=>uenc(0x07,ap(0),ap(1),ap(2)),
            "e"|"jnz"=>uenc(0x0E,ap(0),ap(1),ap(2)),
            "nop"=>uenc(0x00,0,0,0),
            _=>return Err(format!("unknown univac '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(UnivacBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// CDC 6600: 6-bit opcode, 3-bit i/j/k, 18-bit address, 3+3+3 operand regs = 30 bits (padded to 60)
fn cenc(op:u8,i:u8,j:u8,k:u8)->Vec<u8>{write_60(((op as u64)<<54)|((i as u64)<<51)|((j as u64)<<48)|((k as u64)<<45))}

impl Cdc6600Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Cdc6600BuildOutput,String>{
    if p.target!="cdc6600"{return Err(format!("need 'cdc6600', got '{}'",p.target))}
    #[derive(Clone)]
    enum CfKind{If,While}
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter=0;
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim(); if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();bin.extend(cenc(0x0E,reg,0,0));let pos=bin.len()-8;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=((bin.len()/8)&7)as u8;let v=((bin[last]as u64&0x0F)<<56)|((bin[last+1]as u64)<<48)|((bin[last+2]as u64)<<40)|((bin[last+3]as u64)<<32)|((bin[last+4]as u64)<<24)|((bin[last+5]as u64)<<16)|((bin[last+6]as u64)<<8)|(bin[last+7]as u64);let i=(v>>51)&7;bin[last..last+8].copy_from_slice(&write_60((0x0Eu64<<54)|(i<<51)|(here as u64)<<48));let bra_pos=bin.len();bin.extend(cenc(0x0C,0,0,0));frame.br_indices.push(bra_pos);bin.extend(cenc(0x0E,reg,0,0));let beq_pos=bin.len()-8;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=((bin.len()/8)&7)as u8;let v=((bin[last]as u64&0x0F)<<56)|((bin[last+1]as u64)<<48)|((bin[last+2]as u64)<<40)|((bin[last+3]as u64)<<32)|((bin[last+4]as u64)<<24)|((bin[last+5]as u64)<<16)|((bin[last+6]as u64)<<8)|(bin[last+7]as u64);let i=(v>>51)&7;bin[last..last+8].copy_from_slice(&write_60((0x0Eu64<<54)|(i<<51)|(here as u64)<<48));let bra_pos=bin.len();bin.extend(cenc(0x0C,0,0,0));frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=((bin.len()/8)&7)as u8;for &pos in &frame.br_indices{let v=((bin[pos]as u64&0x0F)<<56)|((bin[pos+1]as u64)<<48)|((bin[pos+2]as u64)<<40)|((bin[pos+3]as u64)<<32)|((bin[pos+4]as u64)<<24)|((bin[pos+5]as u64)<<16)|((bin[pos+6]as u64)<<8)|(bin[pos+7]as u64);if(v>>54)&0x3F==0x0E{let i=(v>>51)&7;bin[pos..pos+8].copy_from_slice(&write_60((0x0Eu64<<54)|(i<<51)|(target as u64)<<48))}else{bin[pos..pos+8].copy_from_slice(&write_60((0x0Cu64<<54)|((target as u64)<<48)))}}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();bin.extend(cenc(0x0E,reg,0,0));let pos=bin.len()-8;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=((frame.start_pos/8)&7)as u8;bin.extend(cenc(0x0C,0,back,0));let here=((bin.len()/8)&7)as u8;for &pos in &frame.br_indices{let v=((bin[pos]as u64&0x0F)<<56)|((bin[pos+1]as u64)<<48)|((bin[pos+2]as u64)<<40)|((bin[pos+3]as u64)<<32)|((bin[pos+4]as u64)<<24)|((bin[pos+5]as u64)<<16)|((bin[pos+6]as u64)<<8)|(bin[pos+7]as u64);let i=(v>>51)&7;bin[pos..pos+8].copy_from_slice(&write_60((0x0Eu64<<54)|(i<<51)|(here as u64)<<48))}continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(cenc(0x30,0,0,0));continue}
        if t=="ret"||t=="hlt"{bin.extend(cenc(0x00,0,0,0));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue} let m=parts[0];
        let joined=parts[1..].join(" "); let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add"|"i"=>cenc(0x20,ap(0),ap(1),ap(2)),"fadd"|"f"=>cenc(0x60,ap(0),ap(1),ap(2)),
            "sub"|"d"=>cenc(0x22,ap(0),ap(1),ap(2)),"fsub"=>cenc(0x62,ap(0),ap(1),ap(2)),
            "mul"|"j"=>cenc(0x24,ap(0),ap(1),ap(2)),"fmul"=>cenc(0x64,ap(0),ap(1),ap(2)),
            "div"|"k"=>cenc(0x26,ap(0),ap(1),ap(2)),
            "ld"|"load"=>cenc(0x30,ap(0),0,ap(1)),"st"|"store"=>cenc(0x34,ap(1),0,ap(0)),
            "br"|"jmp"=>cenc(0x0C,0,ap(0),0),"brz"=>cenc(0x0E,ap(0),ap(1),0),
            "brnz"=>cenc(0x0F,ap(0),ap(1),0),"nop"=>cenc(0x00,0,0,0),
            "ret"=>cenc(0x00,0,0,0),
            _=>return Err(format!("unknown cdc6600 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Cdc6600BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}