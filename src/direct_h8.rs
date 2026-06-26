// H8: Renesas 8/16-bit MCU. Regs R0-R7 (8-bit), EXT regs (16-bit).

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct H8BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct H8Builder;

impl H8Builder{pub fn build_bin(p:&Program,out:&Path)->Result<H8BuildOutput,String>{
    if p.target!="h8"{return Err(format!("need 'h8'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xB0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x54,0x70]);continue}
        if let Some(r)=t.strip_prefix("if "){let _=r.trim();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(&[0x43,0]);cf_stack.push(CfFrame{kind:CfKind::If,endif_label:l,br_indices:vec![p],has_else:false,start_pos:0});continue;}
        if let Some(r)=t.strip_prefix("elif "){let _=r.trim();let f=cf_stack.last_mut().ok_or("elif without if")?;if f.has_else{return Err("elif after else".into())}let prev=f.br_indices.pop().ok_or("internal")?;let tgt=(bin.len()+2)as i32;let rel=(tgt-(prev as i32+2))as i8 as u8;bin[prev+1]=rel;let up=bin.len();bin.extend(&[0x40,0]);f.br_indices.push(up);let cp=bin.len();bin.extend(&[0x43,0]);f.br_indices.push(cp);continue;}
        if t=="else"{let f=cf_stack.last_mut().ok_or("else without if")?;if f.has_else{return Err("duplicate else".into())}f.has_else=true;let prev=f.br_indices.pop().ok_or("internal")?;let tgt=bin.len()as i32;let rel=(tgt-(prev as i32+2))as i8 as u8;bin[prev+1]=rel;let up=bin.len();bin.extend(&[0x40,0]);f.br_indices.push(up);continue;}
        if t=="endif"{let f=cf_stack.pop().ok_or("endif without if/while")?;if !matches!(f.kind, CfKind::If){return Err("endif without matching if".into())}let tgt=bin.len()as i32;for&idx in&f.br_indices{let rel=(tgt-(idx as i32+2))as i8 as u8;bin[idx+1]=rel;}continue;}
        if let Some(r)=t.strip_prefix("while "){let _=r.trim();let sp=bin.len();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(&[0x43,0]);cf_stack.push(CfFrame{kind:CfKind::While,endif_label:l,br_indices:vec![p],has_else:false,start_pos:sp});continue;}
        if t=="endwhile"{let f=cf_stack.pop().ok_or("endwhile without while")?;if !matches!(f.kind, CfKind::While){return Err("endwhile without matching while".into())}let sw=f.start_pos as i32;let up=bin.len();let rel=(sw-(up as i32+2))as i8 as u8;bin.extend(&[0x40,rel]);let tgt=bin.len()as i32;for&idx in&f.br_indices{let rel=(tgt-(idx as i32+2))as i8 as u8;bin[idx+1]=rel;}continue;}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|{let s=*a.get(i).unwrap_or(&"");s.trim_start_matches('r').parse::<u8>().unwrap_or(0)};
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add"=>vec![0x20u8|((rp(0)&7)<<4)|(rp(1)&7),ap(1)],
            "sub"=>vec![0x28u8|((rp(0)&7)<<4)|(rp(1)&7),ap(1)],
            "mov"=>vec![0x10u8|((rp(0)&7)<<4)|(rp(1)&7),ap(1)],
            "inc"=>vec![0x14u8|((rp(0)&7)<<4),ap(0)],"dec"=>vec![0x1Cu8|((rp(0)&7)<<4),ap(0)],
            "cmp"=>vec![0x30u8|((rp(0)&7)<<4)|(rp(1)&7),ap(1)],
            "jmp"=>vec![0x40u8|(rp(0)&3),0],"jsr"=>vec![0xB0,0],
            "bne"=>vec![0x42,0],"beq"=>vec![0x43,0],"bra"=>vec![0x40,0],
            "rts"=>vec![0x54,0x70],"nop"=>vec![0x00,0],
            _=>return Err(format!("unknown h8 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(H8BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
