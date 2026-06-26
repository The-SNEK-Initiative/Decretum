// MSP430: TI 16-bit MCU. 16 GPRs (R0=PC, R1=SP, R2=SR, R3=CG, R4-R15).

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Msp430BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Msp430Builder;

impl Msp430Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Msp430BuildOutput,String>{
    if p.target!="msp430"{return Err(format!("need 'msp430'"))}
    let mut bin:Vec<u8>=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x12,0xB0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x41,0x30]);continue}
        let parts:Vec<&str>=t.split(|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let j=parts[1..].join(" ");let a:Vec<&str>=j.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|a.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0u16);
        let w=|v:u16|v.to_le_bytes().to_vec();
        bin.extend(match m{
            "mov"=>w(0x4000u16|((rp(0)&0xF)<<8)|(rp(1)&0xF)),
            "add"=>w(0x5000u16|((rp(0)&0xF)<<8)|(rp(1)&0xF)),
            "sub"=>w(0x6000u16|((rp(0)&0xF)<<8)|(rp(1)&0xF)),
            "cmp"=>w(0x8000u16|((rp(0)&0xF)<<8)|(rp(1)&0xF)),
            "inc"=>w(0x5300u16|((rp(0)&0xF)<<8)),"dec"=>w(0x7300u16|((rp(0)&0xF)<<8)),
            "jmp"=>vec![0x30,0],"jne|jnz"=>vec![0x20,0],"jeq|jz"=>vec![0x24,0],
            "call"=>vec![0x12,0xB0,0,0],"ret"=>vec![0x41,0x30],
            "rra"=>vec![0x11,0],"rrc"=>vec![0x10,0],"push"=>vec![0x12,0],
            "pop"=>vec![0x41,0],"nop"=>vec![0x03,0],
            _=>return Err(format!("unknown msp430 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Msp430BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
