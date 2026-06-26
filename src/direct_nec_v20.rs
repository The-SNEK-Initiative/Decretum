// NEC V20/V30: 8080/8086-compatible with extensions. 8-bit regs, 16-bit addresses.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct NecV20BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct NecV20Builder;

impl NecV20Builder{pub fn build_bin(p:&Program,out:&Path)->Result<NecV20BuildOutput,String>{
    if p.target!="v20"{return Err(format!("need 'v20'"))}
    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xCD,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xC9]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|a.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        let rp=|s:&str|->u8{match s{"a"=>7,"b"=>0,"c"=>1,"d"=>2,"e"=>3,"h"=>4,"l"=>5,"m"=>6,_=>0}};
        let ra=|i:usize|rp(a.get(i).unwrap_or(&""));
        bin.extend(match m{
            "mov" if a.len()==2=>vec![0x40u8|(ra(0)<<3)|ra(1)],
            "mvi"=>vec![0x06u8|(ra(0)<<3),ap(1)],
            "add"=>vec![0x80u8|ra(1)],"sub"=>vec![0x90u8|ra(1)],
            "adi"=>vec![0xC6,ap(0)],"sui"=>vec![0xD6,ap(0)],
            "cmp"=>vec![0xB8u8|ra(0)],"cpi"=>vec![0xFE,ap(0)],
            "jmp"=>vec![0xC3,ap(0)],"call"=>vec![0xCD,ap(0)],
            "ret"=>vec![0xC9],"nop"=>vec![0x00],
            "in"=>vec![0xDB,ap(0)],"out"=>vec![0xD3,ap(0)],
            _=>return Err(format!("unknown v20 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(NecV20BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
