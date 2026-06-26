// PDP-8 (12-bit), PDP-11 (16-bit), VAX (32-bit CISC), HP 3000 (16-bit stack)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Pdp8BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Pdp8Builder;
pub struct Pdp11BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Pdp11Builder;
pub struct VaxBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct VaxBuilder;
pub struct Hp3000BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Hp3000Builder;

fn w12(v:u16)->Vec<u8>{let mut b=[0u8;2];b[0]=(v>>8)as u8&0x0F;b[1]=v as u8;b.to_vec()}
fn w16(v:u16)->Vec<u8>{v.to_le_bytes().to_vec()}
fn w32(v:u32)->Vec<u8>{v.to_le_bytes().to_vec()}

// PDP-8: 3-bit opcode + 1-bit indirect + 4/5/7-bit address = 12-bit instruction
impl Pdp8Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Pdp8BuildOutput,String>{
    if p.target!="pdp8"{return Err(format!("need 'pdp8', got '{}'",p.target))}
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(w12(0x4000));continue}
        if t=="ret"||t=="hlt"{bin.extend(w12(0x0000));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        bin.extend(match m{
            "and"=>w12(0x0000|ap(0)),"tad"|"add"=>w12(0x1000|ap(0)),"isz"=>w12(0x2000|ap(0)),
            "dca"|"store"=>w12(0x3000|ap(0)),"jms"|"call"=>w12(0x4000|ap(0)),"jmp"=>w12(0x5000|ap(0)),
            "iot"=>w12(0x6000|ap(0)),"opr"|"nop"=>w12(0x7000),
            "cla"=>w12(0x7200),"cll"=>w12(0x7100),"hlt"=>w12(0x7402),
            _=>return Err(format!("unknown pdp8 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Pdp8BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// PDP-11: 16-bit, 8 GPRs (R0-R7), orthogonal instruction format
impl Pdp11Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Pdp11BuildOutput,String>{
    if p.target!="pdp11"{return Err(format!("need 'pdp11', got '{}'",p.target))}
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(w16(0x0040));continue}
        if t=="ret"{bin.extend(w16(0x0080));continue}
        if t=="hlt"{bin.extend(w16(0x0000));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0);
        bin.extend(match m{
            "mov" if args.len()==2=>w16(0x0100|(rp(0)<<6)|rp(1)),
            "add" if args.len()==2=>w16(0x0600|(rp(0)<<6)|rp(1)),
            "sub" if args.len()==2=>w16(0x1600|(rp(0)<<6)|rp(1)),
            "cmp" if args.len()==2=>w16(0x0200|(rp(0)<<6)|rp(1)),
            "bne"=>w16(0x0010|rp(0)),"beq"=>w16(0x0014|rp(0)),
            "bgt"=>w16(0x0030|rp(0)),"blt"=>w16(0x0034|rp(0)),
            "br"|"jmp"=>w16(0x0004|rp(0)),
            "jsr"=>w16(0x0040|(rp(0)<<6)|rp(1)),
            "clr"=>w16(0x0050|(rp(0)<<6)),
            "inc"=>w16(0x0052|(rp(0)<<6)),"dec"=>w16(0x0053|(rp(0)<<6)),
            "nop"=>w16(0x0240),
            _=>return Err(format!("unknown pdp11 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Pdp11BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// VAX: variable-length, 1-2 byte opcode prefix + operand specifiers
impl VaxBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<VaxBuildOutput,String>{
    if p.target!="vax"{return Err(format!("need 'vax', got '{}'",p.target))}
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(w16(0x00FB));continue}
        if t=="ret"{bin.extend(w16(0x0004));continue}
        if t=="hlt"{bin.extend(w16(0x0000));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0);
        // VAX operand specifier: 4-bit mode + 4-bit reg
        let os=|r:u16|->u16{(r&0xF)};
        bin.extend(match m{
            "movl"|"mov" if args.len()==2=>{let mut v=vec![0xD0];v.push(os(rp(0))as u8);v.push(os(rp(1))as u8);v}
            "addl2"|"add" if args.len()==2=>{let mut v=vec![0xA0];v.push(os(rp(0))as u8);v.push(os(rp(1))as u8);v}
            "subl2"|"sub" if args.len()==2=>{let mut v=vec![0xA2];v.push(os(rp(0))as u8);v.push(os(rp(1))as u8);v}
            "mull2"|"mul" if args.len()==2=>{let mut v=vec![0xA4];v.push(os(rp(0))as u8);v.push(os(rp(1))as u8);v}
            "clrl"|"clr"=>vec![0xD4,os(rp(0))as u8],
            "brb"|"jmp"=>vec![0x11,os(rp(0))as u8],
            "bneq"=>vec![0x12,os(rp(0))as u8],"beql"=>vec![0x13,os(rp(0))as u8],
            "bsbb"=>vec![0x10,os(rp(0))as u8],
            "rsb"|"ret"=>vec![0x05],
            "nop"=>vec![0x01],
            _=>return Err(format!("unknown vax '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(VaxBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// HP 3000: 16-bit stack-based with 8 GPRs
impl Hp3000Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Hp3000BuildOutput,String>{
    if p.target!="hp3000"{return Err(format!("need 'hp3000', got '{}'",p.target))}
    let mut bin=Vec::new();
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(w16(0x2A00));continue}
        if t=="ret"{bin.extend(w16(0x2E00));continue}
        if t=="hlt"{bin.extend(w16(0x0000));continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        bin.extend(match m{
            "lda"|"load"=>w16(0x0800|ap(0)),"sta"|"store"=>w16(0x0900|ap(0)),
            "add"=>w16(0x0600|ap(0)),"sub"=>w16(0x0700|ap(0)),
            "jmp"=>w16(0x2C00|ap(0)),"jsb"=>w16(0x2A00|ap(0)),
            "lda2"=>w16(0x0800|rp(0)),"add2"=>w16(0x0600|rp(0)),
            "orb"=>w16(0x1100|ap(0)),"andb"=>w16(0x1200|ap(0)),
            "nop"=>w16(0x0000),
            _=>return Err(format!("unknown hp3000 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Hp3000BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
