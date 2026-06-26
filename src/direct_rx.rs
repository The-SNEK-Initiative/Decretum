// RX: Renesas 32-bit MCU, 16 GPRs (R0-R15), 32-bit data paths

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct RxBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct RxBuilder;

enum CfKind{If,While}
struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}

impl RxBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<RxBuildOutput,String>{
    if p.target!="rx"{return Err(format!("need 'rx'"))}
    let mut bin:Vec<u8>=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x08,0,0,0]);continue}
        if t=="ret"{bin.extend(&[0x0C,0,0,0]);continue}
        if t=="hlt"{bin.extend(&[0x00,0,0,0]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();bin.extend((0x50000000u32|(reg as u32)<<4).to_le_bytes());let pos=bin.len();bin.extend(&[0,0,0,0]);cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len() as u32;let enc=0x61000000u32|(here&0xFFFFFF);bin[last..last+4].copy_from_slice(&enc.to_le_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);bin.extend((0x50000000u32|(reg as u32)<<4).to_le_bytes());let beq_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len() as u32;let enc=0x61000000u32|(here&0xFFFFFF);bin[last..last+4].copy_from_slice(&enc.to_le_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len() as u32;for &pos in &frame.br_indices{let enc=0x62000000u32|(target&0xFFFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_le_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();bin.extend((0x50000000u32|(reg as u32)<<4).to_le_bytes());let pos=bin.len();bin.extend(&[0,0,0,0]);cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u32;bin.extend((0x62000000u32|(back&0xFFFFFF)).to_le_bytes());let here=bin.len() as u32;let enc=0x61000000u32|(here&0xFFFFFF);for &pos in &frame.br_indices{bin[pos..pos+4].copy_from_slice(&enc.to_le_bytes())}continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|{let n=s.trim_start_matches('r').parse::<u8>().ok()?;Some(n as u32)}).unwrap_or(0u32);
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let w=|v:u32|v.to_le_bytes().to_vec();
        bin.extend(match m{
            "mov"|"ld"=>w(0x04000000u32|(rp(0)<<4)|rp(1)),
            "add"=>w(0x10000000u32|(rp(0)<<4)|rp(1)),
            "sub"=>w(0x20000000u32|(rp(0)<<4)|rp(1)),
            "mul"=>w(0x30000000u32|(rp(0)<<4)|rp(1)),
            "div"=>w(0x40000000u32|(rp(0)<<4)|rp(1)),
            "cmp"=>w(0x50000000u32|(rp(0)<<4)|rp(1)),
            "jmp"=>w(0x60000000u32),"jsr"=>w(0x08000000u32),
            "rts"=>w(0x0C000000u32),
            "shll"=>w(0x70000000u32|(rp(0)<<4)|rp(1)),
            "shar"=>w(0x80000000u32|(rp(0)<<4)|rp(1)),
            "nop"=>w(0),
            _=>return Err(format!("unknown rx '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(RxBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
