// BESM-6: Soviet 48-bit word. 6-bit opcode + 14-bit addr + 14-bit addr2.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct BesmBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct BesmBuilder;

impl BesmBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<BesmBuildOutput,String>{
    if p.target!="besm"{return Err(format!("need 'besm'"))}
    let mut bin:Vec<u8>=Vec::new();
    let w48=|v:u64|->Vec<u8>{let b=v.to_be_bytes();b[2..8].to_vec()};
    struct CfFrame{kind:CfKind,endif_label:String,br_indices:Vec<usize>,has_else:bool,start_pos:usize}
    enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0,0,0,0x06,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0,0,0,0,0,0]);continue}
        if let Some(r)=t.strip_prefix("if "){let _=r.trim();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(w48((0x09u64<<30)|(0)));cf_stack.push(CfFrame{kind:CfKind::If,endif_label:l,br_indices:vec![p],has_else:false,start_pos:0});continue;}
        if let Some(r)=t.strip_prefix("elif "){let _=r.trim();let f=cf_stack.last_mut().ok_or("elif without if")?;if f.has_else{return Err("elif after else".into())}let prev=f.br_indices.pop().ok_or("internal")?;let tw=((bin.len()+6)/6)as u64;let mut vb=[0u8;8];vb[2..8].copy_from_slice(&bin[prev..prev+6]);let v=u64::from_be_bytes(vb);let oc=(v>>30)&0x3F;let nv=(oc<<30)|(tw&0x3FFF);let nb=nv.to_be_bytes();bin[prev..prev+6].copy_from_slice(&nb[2..8]);let up=bin.len();bin.extend(w48((0x07u64<<30)|(0)));f.br_indices.push(up);let cp=bin.len();bin.extend(w48((0x09u64<<30)|(0)));f.br_indices.push(cp);continue;}
        if t=="else"{let f=cf_stack.last_mut().ok_or("else without if")?;if f.has_else{return Err("duplicate else".into())}f.has_else=true;let prev=f.br_indices.pop().ok_or("internal")?;let tw=(bin.len()/6)as u64;let mut vb=[0u8;8];vb[2..8].copy_from_slice(&bin[prev..prev+6]);let v=u64::from_be_bytes(vb);let oc=(v>>30)&0x3F;let nv=(oc<<30)|(tw&0x3FFF);let nb=nv.to_be_bytes();bin[prev..prev+6].copy_from_slice(&nb[2..8]);let up=bin.len();bin.extend(w48((0x07u64<<30)|(0)));f.br_indices.push(up);continue;}
        if t=="endif"{let f=cf_stack.pop().ok_or("endif without if/while")?;if !matches!(f.kind, CfKind::If){return Err("endif without matching if".into())}let tw=(bin.len()/6)as u64;for&idx in&f.br_indices{let mut vb=[0u8;8];vb[2..8].copy_from_slice(&bin[idx..idx+6]);let v=u64::from_be_bytes(vb);let oc=(v>>30)&0x3F;let nv=(oc<<30)|(tw&0x3FFF);let nb=nv.to_be_bytes();bin[idx..idx+6].copy_from_slice(&nb[2..8]);}continue;}
        if let Some(r)=t.strip_prefix("while "){let _=r.trim();let sp=bin.len();let l=format!("_cf{}",cf_counter);cf_counter+=1;let p=bin.len();bin.extend(w48((0x09u64<<30)|(0)));cf_stack.push(CfFrame{kind:CfKind::While,endif_label:l,br_indices:vec![p],has_else:false,start_pos:sp});continue;}
        if t=="endwhile"{let f=cf_stack.pop().ok_or("endwhile without while")?;if !matches!(f.kind, CfKind::While){return Err("endwhile without matching while".into())}let sw=(f.start_pos/6)as u64;bin.extend(w48((0x07u64<<30)|(sw)));let tw=(bin.len()/6)as u64;for&idx in&f.br_indices{let mut vb=[0u8;8];vb[2..8].copy_from_slice(&bin[idx..idx+6]);let v=u64::from_be_bytes(vb);let oc=(v>>30)&0x3F;let nv=(oc<<30)|(tw&0x3FFF);let nb=nv.to_be_bytes();bin[idx..idx+6].copy_from_slice(&nb[2..8]);}continue;}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u64>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add"=>w48((0x01u64<<30)|(ap(0)&0x3FFF)|((ap(1)&0x3FFF)<<15)),
            "sub"=>w48((0x02u64<<30)|(ap(0)&0x3FFF)|((ap(1)&0x3FFF)<<15)),
            "mul"=>w48((0x03u64<<30)|(ap(0)&0x3FFF)|((ap(1)&0x3FFF)<<15)),
            "div"=>w48((0x04u64<<30)|(ap(0)&0x3FFF)|((ap(1)&0x3FFF)<<15)),
            "ld"=>w48((0x05u64<<30)|(ap(0)&0x3FFF)),"st"=>w48((0x06u64<<30)|(ap(0)&0x3FFF)),
            "jmp"=>w48((0x07u64<<30)|(ap(0)&0x3FFF)),
            "jneg"=>w48((0x08u64<<30)|(ap(0)&0x3FFF)),
            "nop"=>w48(0),
            _=>return Err(format!("unknown besm '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(BesmBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
