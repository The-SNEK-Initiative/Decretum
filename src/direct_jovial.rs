// JOVIAL J73: Military Ada-ish language runtime - compact VM bytecode.
// 8-bit opcodes, stack-based, 16 GPRs.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct JovialBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct JovialBuilder;

impl JovialBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<JovialBuildOutput,String>{
    if p.target!="jovial"{return Err(format!("need 'jovial'"))}
    let mut bin:Vec<u8>=Vec::new();
    struct CfFrame{kind:CfKind,endif_label:String,else_label:String,br_indices:Vec<usize>,cond_br:usize,start_pos:usize,has_else:bool}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:u32=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x30,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x40]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('r').parse::<u8>().ok()).unwrap_or(0);
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let endif_lbl=format!("__cf_{}_endif",cf_counter);
            let else_lbl=format!("__cf_{}_else",cf_counter);
            cf_counter+=1;
            bin.extend(&[0x14,rn,0]);
            bin.push(0x22);
            let cond_br=bin.len();
            bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::If,endif_label:endif_lbl,else_label:else_lbl,br_indices:Vec::new(),cond_br,start_pos:0,has_else:false});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            if frame.has_else{return Err("elif after else".to_string())}
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            bin[frame.cond_br]=bin.len() as u8;
            bin.push(0x20);
            let uncond_idx=bin.len();
            bin.push(0x00);
            frame.br_indices.push(uncond_idx);
            bin.extend(&[0x14,rn,0]);
            bin.push(0x22);
            frame.cond_br=bin.len();
            bin.push(0x00);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            if frame.has_else{return Err("duplicate else".to_string())}
            frame.has_else=true;
            bin[frame.cond_br]=bin.len() as u8;
            bin.push(0x20);
            let uncond_idx=bin.len();
            bin.push(0x00);
            frame.br_indices.push(uncond_idx);
            frame.cond_br=0;
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif without matching if".to_string())}
            let end=bin.len();
            if frame.cond_br>0{bin[frame.cond_br]=end as u8}
            for&b in &frame.br_indices{bin[b]=end as u8}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();
            let endwhile_lbl=format!("__cf_{}_endwhile",cf_counter);
            cf_counter+=1;
            bin.extend(&[0x14,rn,0]);
            bin.push(0x22);
            let cond_br=bin.len();
            bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::While,endif_label:endwhile_lbl,else_label:String::new(),br_indices:Vec::new(),cond_br,start_pos,has_else:false});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile without matching while".to_string())}
            bin.push(0x20);
            bin.push(frame.start_pos as u8);
            let end=bin.len();
            if frame.cond_br>0{bin[frame.cond_br]=end as u8}
            continue;
        }
        bin.extend(match m{
            "add"=>vec![0x10,rp(0),rp(1)],"sub"=>vec![0x11,rp(0),rp(1)],
            "mul"=>vec![0x12,rp(0),rp(1)],"div"=>vec![0x13,rp(0),rp(1)],
            "mov"=>vec![0x01,rp(0),rp(1)],"cmp"=>vec![0x14,rp(0),rp(1)],
            "jmp"=>vec![0x20,ap(0)],"jne"=>vec![0x21,ap(0)],"jeq"=>vec![0x22,ap(0)],
            "call"=>vec![0x30,ap(0),ap(1)],"ret"=>vec![0x40],
            "nop"=>vec![0x00],
            _=>return Err(format!("unknown jovial '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(JovialBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
