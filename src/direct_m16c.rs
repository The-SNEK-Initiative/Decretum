// M16C: Renesas 16-bit MCU. 8 GPRs, bit-addressable.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct M16cBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct M16cBuilder;

impl M16cBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<M16cBuildOutput,String>{
    if p.target!="m16c"{return Err(format!("need 'm16c'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xFA,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xFB]);continue}
        if let Some(r)=t.strip_prefix("if "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            cf_counter+=1;
            bin.extend(&[0x40|((rn&7)<<4),0]);
            let p=bin.len();bin.extend(&[0xE3,0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=((bin.len() as i32)-(prev as i32)-3)as i16 as u16;
            bin[prev+1]=rel as u8;bin[prev+2]=(rel>>8)as u8;
            let jmp_idx=bin.len();bin.extend(&[0xFA,0,0]);
            f.br_indices.push(jmp_idx);
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            bin.extend(&[0x40|((rn&7)<<4),0]);
            let p=bin.len();bin.extend(&[0xE3,0,0]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=((bin.len() as i32)-(prev as i32)-3)as i16 as u16;
            bin[prev+1]=rel as u8;bin[prev+2]=(rel>>8)as u8;
            let jmp_idx=bin.len();bin.extend(&[0xFA,0,0]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            for &idx in &f.br_indices{
                let rel=((bin.len() as i32)-(idx as i32)-3)as i16 as u16;
                bin[idx+1]=rel as u8;bin[idx+2]=(rel>>8)as u8;
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let sp=bin.len();cf_counter+=1;
            bin.extend(&[0x40|((rn&7)<<4),0]);
            let p=bin.len();bin.extend(&[0xE3,0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let rel=((f.start_pos as i32)-(bin.len() as i32)-3)as i16 as u16;
            bin.extend(&[0xFA,rel as u8,(rel>>8)as u8]);
            for &idx in &f.br_indices{
                let rel=((bin.len() as i32)-(idx as i32)-3)as i16 as u16;
                bin[idx+1]=rel as u8;bin[idx+2]=(rel>>8)as u8;
            }
            continue;
        }
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|{let s=*a.get(i).unwrap_or(&"");s.trim_start_matches('r').parse::<u8>().unwrap_or(0)};
        bin.extend(match m{
            "add"=>vec![0x20u8|((rp(0)&7)<<4)|(rp(1)&7),0],
            "sub"=>vec![0x30u8|((rp(0)&7)<<4)|(rp(1)&7),0],
            "mov"=>vec![0x10u8|((rp(0)&7)<<4)|(rp(1)&7),0],
            "cmp"=>vec![0x40u8|((rp(0)&7)<<4)|(rp(1)&7),0],
            "jmp"=>vec![0xFA,0,0],"jsr"=>vec![0xFC,0,0],
            "rts"=>vec![0xFB],"nop"=>vec![0x00],
            _=>return Err(format!("unknown m16c '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(M16cBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
