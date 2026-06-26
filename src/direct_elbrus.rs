// Elbrus: Soviet VLIW. 32-bit instructions emitted sequentially, 32 GPRs.
// Separate architecture from TI C6x VLIW - different encoding, different ISA lineage.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct ElbrusBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct ElbrusBuilder;

impl ElbrusBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<ElbrusBuildOutput,String>{
    if p.target!="elbrus"{return Err(format!("need 'elbrus'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x09,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x04,0,0,0]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let w=|v:u32|v.to_be_bytes().to_vec();
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&(0x1C000000u32|(rn<<21)).to_be_bytes());
            let pos=bin.len();bin.extend(&[0;4]);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            let word_off=(here as i32-last as i32-4)>>2;
            let enc=(0x28000000u32)|((word_off as u32)&0x3FFFFFF);
            bin[last..last+4].copy_from_slice(&enc.to_be_bytes());
            let bra=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(bra);
            bin.extend(&(0x1C000000u32|(rn<<21)).to_be_bytes());
            let beq=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            let word_off=(here as i32-last as i32-4)>>2;
            let enc=(0x28000000u32)|((word_off as u32)&0x3FFFFFF);
            bin[last..last+4].copy_from_slice(&enc.to_be_bytes());
            let bra=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{
                let word_off=(target as i32-pos as i32-4)>>2;
                let enc=(0x20000000u32)|((word_off as u32)&0x3FFFFFF);
                bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes());
            }
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&(0x1C000000u32|(rn<<21)).to_be_bytes());
            let pos=bin.len();bin.extend(&[0;4]);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            let word_off=(frame.start_pos as i32-bin.len() as i32-4)>>2;
            bin.extend(&(0x20000000u32|((word_off as u32)&0x3FFFFFF)).to_be_bytes());
            let target=bin.len();
            for &pos in &frame.br_indices{
                let word_off=(target as i32-pos as i32-4)>>2;
                let enc=(0x28000000u32)|((word_off as u32)&0x3FFFFFF);
                bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes());
            }
            continue;
        }
        bin.extend(match m{
            "add"=>w(0x04000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "sub"=>w(0x08000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "mul"=>w(0x0C000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "mov"=>w(0x10000000u32|(rp(0)<<21)|(rp(1)<<16)),
            "ld"=>w(0x14000000u32|(rp(0)<<21)|(rp(1)<<16)|(ap(2)&0xFFFF)),
            "st"=>w(0x18000000u32|(rp(0)<<21)|(rp(1)<<16)|(ap(2)&0xFFFF)),
            "cmp"=>w(0x1C000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "jmp"=>w(0x20000000u32|(ap(0)&0x03FFFFFF)),
            "call"=>w(0x24000000u32|(ap(0)&0x03FFFFFF)),
            "ret"=>w(0x04000000u32),"nop"=>w(0),
            _=>return Err(format!("unknown elbrus '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(ElbrusBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
