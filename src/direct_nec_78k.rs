// NEC 78K: 8-bit MCU. 8 GPRs (r0-r7), accumulator A.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Nec78kBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Nec78kBuilder;

impl Nec78kBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<Nec78kBuildOutput,String>{
    if p.target!="nec78k"{return Err(format!("need 'nec78k'"))}
    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xCD,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xCB]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "mov"=>vec![0x40u8|ap(0),0],"xch"=>vec![0x60],
            "add"=>vec![0x20u8|ap(0),0],"sub"=>vec![0x30u8|ap(0),0],
            "cmp"=>vec![0x50u8|ap(0),0],
            "jmp"=>vec![0x90,ap(0)],"call"=>vec![0xCD,ap(0)],
            "ret"=>vec![0xCB],"nop"=>vec![0x00],
            _=>return Err(format!("unknown nec78k '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Nec78kBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
