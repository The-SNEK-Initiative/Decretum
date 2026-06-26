// Mill architecture: belt-based result forwarding, special/regular/next operands.
// 32-bit instructions, belt slots s0-sN, special/scratchpad memory.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct MillBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct MillBuilder;

impl MillBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<MillBuildOutput,String>{
    if p.target!="mill"{return Err(format!("need 'mill'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x50,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x04,0,0,0x04]);continue}
        if let Some(r)=t.strip_prefix("if "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            cf_counter+=1;
            let cmp=0xC0000000u32|((rn as u32)<<12);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x02,0,0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i32-prev as i32-4)as i32;
            let ow=u32::from_be_bytes([bin[prev],bin[prev+1],bin[prev+2],bin[prev+3]]);
            let nw=(ow&0xFF000000)|((rel as u32)&0x00FFFFFF);
            let nb=nw.to_be_bytes();bin[prev..prev+4].copy_from_slice(&nb);
            let jmp_idx=bin.len();bin.extend(&[0x01,0,0,0]);
            f.br_indices.push(jmp_idx);
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let cmp=0xC0000000u32|((rn as u32)<<12);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x02,0,0,0]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=(bin.len() as i32-prev as i32-4)as i32;
            let ow=u32::from_be_bytes([bin[prev],bin[prev+1],bin[prev+2],bin[prev+3]]);
            let nw=(ow&0xFF000000)|((rel as u32)&0x00FFFFFF);
            let nb=nw.to_be_bytes();bin[prev..prev+4].copy_from_slice(&nb);
            let jmp_idx=bin.len();bin.extend(&[0x01,0,0,0]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            for &idx in &f.br_indices{
                let rel=(bin.len() as i32-idx as i32-4)as i32;
                let ow=u32::from_be_bytes([bin[idx],bin[idx+1],bin[idx+2],bin[idx+3]]);
                let nw=(ow&0xFF000000)|((rel as u32)&0x00FFFFFF);
                let nb=nw.to_be_bytes();bin[idx..idx+4].copy_from_slice(&nb);
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let sp=bin.len();cf_counter+=1;
            let cmp=0xC0000000u32|((rn as u32)<<12);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x02,0,0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let rel=(f.start_pos as i32-bin.len() as i32-4)as i32;
            let nb=(0x01000000u32|((rel as u32)&0x00FFFFFF)).to_be_bytes();
            bin.extend(&nb);
            for &idx in &f.br_indices{
                let rel=(bin.len() as i32-idx as i32-4)as i32;
                let ow=u32::from_be_bytes([bin[idx],bin[idx+1],bin[idx+2],bin[idx+3]]);
                let nw=(ow&0xFF000000)|((rel as u32)&0x00FFFFFF);
                let nb=nw.to_be_bytes();bin[idx..idx+4].copy_from_slice(&nb);
            }
            continue;
        }
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let bp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('b').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let w=|v:u32|v.to_be_bytes().to_vec();
        bin.extend(match m{
            "add"=>w(0x40000000u32|(bp(0)<<18)|(bp(1)<<12)|(bp(2)<<6)),
            "sub"=>w(0x44000000u32|(bp(0)<<18)|(bp(1)<<12)|(bp(2)<<6)),
            "mul"=>w(0x48000000u32|(bp(0)<<18)|(bp(1)<<12)|(bp(2)<<6)),
            "div"=>w(0x4C000000u32|(bp(0)<<18)|(bp(1)<<12)|(bp(2)<<6)),
            "addi"=>w(0x50000000u32|(bp(0)<<18)|(bp(1)<<12)|(ap(2)&0x0FFF)),
            "ld"=>w(0x80000000u32|(bp(0)<<18)|(bp(1)<<12)|(ap(2)&0x0FFF)),
            "st"=>w(0x84000000u32|(bp(0)<<18)|(bp(1)<<12)|(ap(2)&0x0FFF)),
            "cmp"=>w(0xC0000000u32|(bp(0)<<12)|(bp(1)<<6)),
            "jmp"=>w(0x01000000u32|(ap(0)&0x00FFFFFF)),
            "call"=>w(0x05000000u32|(ap(0)&0x00FFFFFF)),
            "ret"=>w(0x04000004u32),"nop"=>w(0),
            _=>return Err(format!("unknown mill '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(MillBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
