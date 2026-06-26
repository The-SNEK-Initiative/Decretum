// Harvard architecture: separate program and data memories.
// 16-bit instructions, 16 GPRs, LPM instruction to load from program space.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct HarvardBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct HarvardBuilder;

impl HarvardBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<HarvardBuildOutput,String>{
    if p.target!="harvard"{return Err(format!("need 'harvard'"))}

    struct CfFrame {
        kind: CfKind,
        br_indices: Vec<usize>,
        has_else: bool,
        start_pos: usize,
    }
    enum CfKind { If, While }

    let mut cf_stack: Vec<CfFrame> = Vec::new();

    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}

        let patch_branch = |bin: &mut [u8], idx: usize, target: u16| {
            let old = ((bin[idx] as u16) << 8) | bin[idx + 1] as u16;
            let new = if (old >> 12) == 0xC {
                (old & 0xFF00) | (target & 0xFF)
            } else {
                (old & 0xF000) | (target & 0xFFF)
            };
            let bs = new.to_be_bytes();
            bin[idx] = bs[0]; bin[idx + 1] = bs[1];
        };

        if let Some(cond_str) = t.strip_prefix("if ") {
            let reg: u16 = cond_str.trim().trim_start_matches('r').parse::<u16>().map_err(|_| "bad reg".to_string())?;
            let br_idx = bin.len();
            bin.extend((0xC000u16 | ((reg & 0xF) << 8) | 0).to_be_bytes());
            cf_stack.push(CfFrame { kind: CfKind::If, br_indices: vec![br_idx], has_else: false, start_pos: 0 });
            continue;
        }
        if let Some(cond_str) = t.strip_prefix("elif ") {
            let frame = cf_stack.last_mut().ok_or("elif without if".to_string())?;
            if matches!(frame.kind, CfKind::While) { return Err("elif in while".to_string()); }
            if frame.has_else { return Err("elif after else".to_string()); }
            let prev = frame.br_indices.pop().ok_or("internal")?;
            let elif_body = (bin.len() / 2) as u16;
            patch_branch(&mut bin, prev, elif_body);
            let jr_idx = bin.len();
            bin.extend((0xE000u16 | 0).to_be_bytes()); // jr 0 placeholder
            let reg: u16 = cond_str.trim().trim_start_matches('r').parse::<u16>().map_err(|_| "bad reg".to_string())?;
            let jz_idx = bin.len();
            bin.extend((0xC000u16 | ((reg & 0xF) << 8) | 0).to_be_bytes());
            frame.br_indices.push(jr_idx);
            frame.br_indices.push(jz_idx);
            continue;
        }
        if t == "else" {
            let frame = cf_stack.last_mut().ok_or("else without if".to_string())?;
            if matches!(frame.kind, CfKind::While) { return Err("else in while".to_string()); }
            if frame.has_else { return Err("duplicate else".to_string()); }
            frame.has_else = true;
            let prev = frame.br_indices.pop().ok_or("internal")?;
            let else_body = (bin.len() / 2) as u16;
            patch_branch(&mut bin, prev, else_body);
            let jr_idx = bin.len();
            bin.extend((0xE000u16 | 0).to_be_bytes());
            frame.br_indices.push(jr_idx);
            continue;
        }
        if t == "endif" {
            let frame = cf_stack.pop().ok_or("endif without if".to_string())?;
            if matches!(frame.kind, CfKind::While) { return Err("endif while expecting endwhile".to_string()); }
            let target = (bin.len() / 2) as u16;
            for &idx in &frame.br_indices { patch_branch(&mut bin, idx, target); }
            continue;
        }
        if let Some(cond_str) = t.strip_prefix("while ") {
            let reg: u16 = cond_str.trim().trim_start_matches('r').parse::<u16>().map_err(|_| "bad reg".to_string())?;
            let start_pos = bin.len() / 2;
            let br_idx = bin.len();
            bin.extend((0xC000u16 | ((reg & 0xF) << 8) | 0).to_be_bytes());
            cf_stack.push(CfFrame { kind: CfKind::While, br_indices: vec![br_idx], has_else: false, start_pos });
            continue;
        }
        if t == "endwhile" {
            let frame = cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if !matches!(frame.kind, CfKind::While) { return Err("endwhile without matching while".to_string()); }
            let start_addr = frame.start_pos as u16;
            bin.extend((0xE000u16 | (start_addr & 0xFFF)).to_be_bytes());
            let target = (bin.len() / 2) as u16;
            for &idx in &frame.br_indices { patch_branch(&mut bin, idx, target); }
            continue;
        }

        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0u16);
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        let w=|v:u16|v.to_be_bytes().to_vec();
        bin.extend(match m{
            "add"=>w(0x1000u16|(rp(0)<<8)|rp(1)),"sub"=>w(0x2000u16|(rp(0)<<8)|rp(1)),
            "mul"=>w(0x3000u16|(rp(0)<<8)|rp(1)),"div"=>w(0x4000u16|(rp(0)<<8)|rp(1)),
            "mov"=>w(0x5000u16|(rp(0)<<8)|rp(1)),
            "ld"|"load"=>w(0x6000u16|(rp(0)<<8)|rp(1)|(ap(2)&0xF)),
            "st"|"store"=>w(0x7000u16|(rp(0)<<8)|rp(1)|(ap(2)&0xF)),
            "lpm"=>w(0x8000u16|(rp(0)<<8)|(ap(1)&0xFF)),
            "cmp"=>w(0x9000u16|(rp(0)<<8)|rp(1)),
            "jmp"=>w(0xA000u16|(ap(0)&0x0FFF)),"call"=>w(0xB000u16|(ap(0)&0x0FFF)),
            "ret"=>w(0x0000),"nop"=>w(0xFFFF),
            _=>return Err(format!("unknown harvard '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed if/while block".into());}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(HarvardBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
