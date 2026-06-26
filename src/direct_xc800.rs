// XC800: Infineon 8-bit 8051-compatible MCU. Accumulator A, B reg, DPTR.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Xc800BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Xc800Builder;

enum CfKind{If,While}
struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}

impl Xc800Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Xc800BuildOutput,String>{
    if p.target!="xc800"{return Err(format!("need 'xc800'"))}
    let mut bin:Vec<u8>=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x12,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x22]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();if reg!=0{bin.extend(&[0xE8|reg])}bin.extend(&[0x60,0]);let pos=bin.len()-2;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len();let offset=here.wrapping_sub(last).wrapping_sub(2)as u8;bin[last+1]=offset;let bra_pos=bin.len();bin.extend(&[0x02,0,0]);frame.br_indices.push(bra_pos);if reg!=0{bin.extend(&[0xE8|reg])}bin.extend(&[0x60,0]);let beq_pos=bin.len()-2;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len();let offset=here.wrapping_sub(last).wrapping_sub(2)as u8;bin[last+1]=offset;let bra_pos=bin.len();bin.extend(&[0x02,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len()as u16;for &pos in &frame.br_indices{if bin[pos]==0x60{let offset=target.wrapping_sub(pos as u16).wrapping_sub(2)as u8;bin[pos+1]=offset}else{bin[pos+1]=(target>>8)as u8;bin[pos+2]=target as u8}}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();if reg!=0{bin.extend(&[0xE8|reg])}bin.extend(&[0x60,0]);let pos=bin.len()-2;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos;bin.extend(&[0x02,(back>>8)as u8,back as u8]);let here=bin.len();for &pos in &frame.br_indices{let offset=here.wrapping_sub(pos).wrapping_sub(2)as u8;bin[pos+1]=offset}continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add"=>vec![0x24,ap(0)],"addc"=>vec![0x34,ap(0)],
            "subb"=>vec![0x94,ap(0)],
            "inc"=>vec![0x04],"dec"=>vec![0x14],
            "mov"=>vec![0x74,ap(0)],"movx"=>vec![0xE0],
            "anl"=>vec![0x54,ap(0)],"orl"=>vec![0x44,ap(0)],"xrl"=>vec![0x64,ap(0)],
            "clr"=>vec![0xE4],"cpl"=>vec![0xF4],
            "rr"=>vec![0x03],"rl"=>vec![0x23],
            "jmp"=>vec![0x02,ap(0),ap(1)],"jz"=>vec![0x60,ap(0)],"jnz"=>vec![0x70,ap(0)],
            "call"=>vec![0x12,ap(0),ap(1)],"ret"=>vec![0x22],
            "nop"=>vec![0x00],
            _=>return Err(format!("unknown xc800 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Xc800BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
