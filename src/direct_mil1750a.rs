// MIL-STD-1750A: 16-bit military standard. 16 GPRs (R0-R15).

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Mil1750aBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Mil1750aBuilder;

impl Mil1750aBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<Mil1750aBuildOutput,String>{
    if p.target!="mil1750a"{return Err(format!("need 'mil1750a'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x7E,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x0C,0xE0]);continue}
        if let Some(r)=t.strip_prefix("if "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            cf_counter+=1;
            let cmp=0x6200u16|((rn as u16)<<8);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x06,0x00]);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:format!("_cf{}",cf_counter),else_label:format!("_cf{}_else",cf_counter),br_indices:vec![p],has_else:false,start_pos:0});
            continue;
        }
        if let Some(r)=t.strip_prefix("elif "){
            let f=cf_stack.last_mut().ok_or("elif without if")?;
            if f.has_else{return Err("elif after else".into())}
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=((bin.len() as i32-prev as i32-2)/2)as i16;
            let ow=u16::from_be_bytes([bin[prev],bin[prev+1]]);
            let nw=(ow&!0x01FF)|(rel as u16&0x01FF);let nb=nw.to_be_bytes();
            bin[prev]=nb[0];bin[prev+1]=nb[1];
            let jmp_idx=bin.len();bin.extend(&[0x02,0x00]);
            f.br_indices.push(jmp_idx);
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let cmp=0x6200u16|((rn as u16)<<8);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x06,0x00]);
            f.br_indices.push(p);
            continue;
        }
        if t=="else"{
            let f=cf_stack.last_mut().ok_or("else without if")?;
            if f.has_else{return Err("duplicate else".into())}
            f.has_else=true;
            let prev=f.br_indices.pop().ok_or("internal")?;
            let rel=((bin.len() as i32-prev as i32-2)/2)as i16;
            let ow=u16::from_be_bytes([bin[prev],bin[prev+1]]);
            let nw=(ow&!0x01FF)|(rel as u16&0x01FF);let nb=nw.to_be_bytes();
            bin[prev]=nb[0];bin[prev+1]=nb[1];
            let jmp_idx=bin.len();bin.extend(&[0x02,0x00]);
            f.br_indices.push(jmp_idx);
            continue;
        }
        if t=="endif"{
            let f=cf_stack.pop().ok_or("endif without if/while")?;
            if !matches!(f.kind,CfKind::If){return Err("endif without matching if".into())}
            for &idx in &f.br_indices{
                let rel=((bin.len() as i32-idx as i32-2)/2)as i16;
                let ow=u16::from_be_bytes([bin[idx],bin[idx+1]]);
                let nw=(ow&!0x01FF)|(rel as u16&0x01FF);let nb=nw.to_be_bytes();
                bin[idx]=nb[0];bin[idx+1]=nb[1];
            }
            continue;
        }
        if let Some(r)=t.strip_prefix("while "){
            let rn=r.trim().trim_start_matches('r').parse::<u8>().unwrap_or(0);
            let sp=bin.len();cf_counter+=1;
            let cmp=0x6200u16|((rn as u16)<<8);bin.extend(&cmp.to_be_bytes());
            let p=bin.len();bin.extend(&[0x06,0x00]);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:format!("_cf{}",cf_counter),else_label:String::new(),br_indices:vec![p],has_else:false,start_pos:sp});
            continue;
        }
        if t=="endwhile"{
            let f=cf_stack.pop().ok_or("endwhile without while")?;
            if !matches!(f.kind,CfKind::While){return Err("endwhile without matching while".into())}
            let rel=((f.start_pos as i32-bin.len() as i32-2)/2)as i16;
            let nb=(0x0200u16|(rel as u16&0x01FF)).to_be_bytes();
            bin.extend(&nb);
            for &idx in &f.br_indices{
                let rel=((bin.len() as i32-idx as i32-2)/2)as i16;
                let ow=u16::from_be_bytes([bin[idx],bin[idx+1]]);
                let nw=(ow&!0x01FF)|(rel as u16&0x01FF);let nb=nw.to_be_bytes();
                bin[idx]=nb[0];bin[idx+1]=nb[1];
            }
            continue;
        }
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|{let s=*a.get(i).unwrap_or(&"");s.trim_start_matches('r').parse::<u8>().unwrap_or(0)};
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        let w=|v:u16|v.to_be_bytes().to_vec();
        bin.extend(match m{
            "add"=>w(0x6000u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "sub"=>w(0x6400u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "mul"=>w(0x6800u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "div"=>w(0x6C00u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "mov"=>w(0x1000u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "cmp"=>w(0x6200u16|((rp(0) as u16)<<8)|(rp(1) as u16)),
            "jmp"=>w(0x0200u16|(ap(0)&0x01FF)),"jsr"=>w(0x7E00u16|(ap(0)&0x01FF)),
            "bne"=>w(0x0800u16|(ap(0)&0x01FF)),"beq"=>w(0x0600u16|(ap(0)&0x01FF)),
            "ret"=>w(0x0CE0),"nop"=>w(0),
            _=>return Err(format!("unknown mil1750a '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Mil1750aBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
