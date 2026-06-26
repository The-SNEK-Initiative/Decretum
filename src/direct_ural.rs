// Ural-1: Soviet 35-bit word. 6-bit opcode + 14-bit address per word.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct UralBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct UralBuilder;

enum CfKind{If,While}
struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}

impl UralBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<UralBuildOutput,String>{
    if p.target!="ural"{return Err(format!("need 'ural'"))}
    let mut bin:Vec<u8>=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x06,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0,0,0,0]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();let v=0x00800000u32|((reg as u32)<<8);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x0000FF00;let here=bin.len() as u32;let enc=0x00800000u32|reg_bits|(here&0xFFFF);bin[last..last+4].copy_from_slice(&enc.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);let bv=0x00800000u32|((reg as u32)<<8);bin.extend(&bv.to_be_bytes());let beq_pos=bin.len()-4;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x0000FF00;let here=bin.len() as u32;let enc=0x00800000u32|reg_bits|(here&0xFFFF);bin[last..last+4].copy_from_slice(&enc.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len() as u32;for &pos in &frame.br_indices{let enc=0x00600000u32|(target&0xFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();let v=0x00800000u32|((reg as u32)<<8);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u32;bin.extend((0x00600000u32|(back&0xFFFF)).to_be_bytes());let here=bin.len() as u32;for &pos in &frame.br_indices{let word=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);let reg_bits=word&0x0000FF00;let enc=0x00800000u32|reg_bits|(here&0xFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes())}continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let w=|v:u32|v.to_be_bytes().to_vec();
        bin.extend(match m{
            "add"=>w(0x00100000u32|(ap(0)&0xFFFF)),"sub"=>w(0x00200000u32|(ap(0)&0xFFFF)),
            "mul"=>w(0x00300000u32|(ap(0)&0xFFFF)),"ld"=>w(0x00400000u32|(ap(0)&0xFFFF)),
            "st"=>w(0x00500000u32|(ap(0)&0xFFFF)),"jmp"=>w(0x00600000u32|(ap(0)&0xFFFF)),
            "jneg"=>w(0x00700000u32|(ap(0)&0xFFFF)),"nop"=>w(0),
            _=>return Err(format!("unknown ural '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(UralBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
