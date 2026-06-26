// FR: Fujitsu 32-bit RISC. 32 GPRs, fixed 32-bit encodings.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct FrBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct FrBuilder;

impl FrBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<FrBuildOutput,String>{
    if p.target!="fr"{return Err(format!("need 'fr'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x9B,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x9C,0,0,0]);continue}
        if let Some(r)=t.strip_prefix("if "){let _=r.trim();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(&[0x88,0,0,0]);cf_stack.push(CfFrame{kind:CfKind::If,endif_label:l,br_indices:vec![p],has_else:false,start_pos:0});continue;}
        if let Some(r)=t.strip_prefix("elif "){let _=r.trim();let f=cf_stack.last_mut().ok_or("elif without if")?;if f.has_else{return Err("elif after else".into())}let prev=f.br_indices.pop().ok_or("internal")?;let tgt=(bin.len()+4)as i32;let d=tgt-(prev as i32+4);let du=d as u32;bin[prev+1]=((du>>16)&0xFF)as u8;bin[prev+2]=((du>>8)&0xFF)as u8;bin[prev+3]=(du&0xFF)as u8;let up=bin.len();bin.extend(&[0x80,0,0,0]);f.br_indices.push(up);let cp=bin.len();bin.extend(&[0x88,0,0,0]);f.br_indices.push(cp);continue;}
        if t=="else"{let f=cf_stack.last_mut().ok_or("else without if")?;if f.has_else{return Err("duplicate else".into())}f.has_else=true;let prev=f.br_indices.pop().ok_or("internal")?;let tgt=bin.len()as i32;let d=tgt-(prev as i32+4);let du=d as u32;bin[prev+1]=((du>>16)&0xFF)as u8;bin[prev+2]=((du>>8)&0xFF)as u8;bin[prev+3]=(du&0xFF)as u8;let up=bin.len();bin.extend(&[0x80,0,0,0]);f.br_indices.push(up);continue;}
        if t=="endif"{let f=cf_stack.pop().ok_or("endif without if/while")?;if !matches!(f.kind, CfKind::If){return Err("endif without matching if".into())}let tgt=bin.len()as i32;for&idx in&f.br_indices{let d=tgt-(idx as i32+4);let du=d as u32;bin[idx+1]=((du>>16)&0xFF)as u8;bin[idx+2]=((du>>8)&0xFF)as u8;bin[idx+3]=(du&0xFF)as u8;}continue;}
        if let Some(r)=t.strip_prefix("while "){let _=r.trim();let sp=bin.len();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(&[0x88,0,0,0]);cf_stack.push(CfFrame{kind:CfKind::While,endif_label:l,br_indices:vec![p],has_else:false,start_pos:sp});continue;}
        if t=="endwhile"{let f=cf_stack.pop().ok_or("endwhile without while")?;if !matches!(f.kind, CfKind::While){return Err("endwhile without matching while".into())}let sw=f.start_pos as i32;let up=bin.len();let d=sw-(up as i32+4);let du=d as u32;bin.extend(&[0x80,((du>>16)&0xFF)as u8,((du>>8)&0xFF)as u8,(du&0xFF)as u8]);let tgt=bin.len()as i32;for&idx in&f.br_indices{let d=tgt-(idx as i32+4);let du=d as u32;bin[idx+1]=((du>>16)&0xFF)as u8;bin[idx+2]=((du>>8)&0xFF)as u8;bin[idx+3]=(du&0xFF)as u8;}continue;}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0u32);
        let w=|v:u32|v.to_be_bytes().to_vec();
        bin.extend(match m{
            "add"=>w(0x10000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "sub"=>w(0x20000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "mul"=>w(0x30000000u32|(rp(0)<<21)|(rp(1)<<16)|(rp(2)<<11)),
            "mov"=>w(0x04000000u32|(rp(0)<<21)|(rp(1)<<16)),
            "ld"=>w(0x50000000u32|(rp(0)<<21)|(rp(1)<<16)),
            "st"=>w(0x60000000u32|(rp(0)<<21)|(rp(1)<<16)),
            "cmp"=>w(0x40000000u32|(rp(0)<<21)|(rp(1)<<16)),
            "jmp"=>w(0x80000000u32),"call"=>w(0x9B000000u32),
            "ret"=>w(0x9C000000u32),"nop"=>w(0),
            _=>return Err(format!("unknown fr '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(FrBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
