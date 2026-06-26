// R8C: Renesas 8-bit MCU (M16C subset). 8 GPRs.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct R8cBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct R8cBuilder;

impl R8cBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<R8cBuildOutput,String>{
    if p.target!="r8c"{return Err(format!("need 'r8c'"))}
    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xFA,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xFB]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|{let s=*a.get(i).unwrap_or(&"");s.trim_start_matches('r').parse::<u8>().unwrap_or(0)};
        bin.extend(match m{
            "mov"=>vec![0x10u8|((rp(0)&7)<<4)|(rp(1)&7)],
            "add"=>vec![0x20u8|((rp(0)&7)<<4)|(rp(1)&7)],
            "sub"=>vec![0x30u8|((rp(0)&7)<<4)|(rp(1)&7)],
            "cmp"=>vec![0x40u8|((rp(0)&7)<<4)|(rp(1)&7)],
            "jmp"=>vec![0xFA,0],"jsr"=>vec![0xFC,0],
            "rts"=>vec![0xFB],"nop"=>vec![0x00],
            _=>return Err(format!("unknown r8c '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(R8cBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
