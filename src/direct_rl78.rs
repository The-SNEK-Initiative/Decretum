// RL78: Renesas 16-bit MCU. 8/16-bit regs, 0-1MB address space.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Rl78BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Rl78Builder;

impl Rl78Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Rl78BuildOutput,String>{
    if p.target!="rl78"{return Err(format!("need 'rl78'"))}
    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xCD,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xCB]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "mov"=>vec![0x04,ap(0),ap(1)],"xch"=>vec![0x0C,ap(0),ap(1)],
            "add"=>vec![0x08,ap(0),ap(1)],"sub"=>vec![0x18,ap(0),ap(1)],
            "cmp"=>vec![0x28,ap(0),ap(1)],
            "inc"=>vec![0x48,ap(0)],"dec"=>vec![0x58,ap(0)],
            "jmp"=>vec![0x8E,ap(0)],"call"=>vec![0xCD,ap(0),ap(1)],
            "ret"=>vec![0xCB],"nop"=>vec![0x40],
            _=>return Err(format!("unknown rl78 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Rl78BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
