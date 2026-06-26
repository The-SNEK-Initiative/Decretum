// DSPs: TI TMS320C54x, ADI Blackfin, ADI SHARC
// Fixed-point and floating-point DSP architectures, Harvard dual-bus, SIMD MAC units

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Tms320BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Tms320Builder;
pub struct BlackfinBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct BlackfinBuilder;
pub struct SharcBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct SharcBuilder;

// TMS320C54x: 16-bit fixed-point DSP, 8 accumulators (A,B), 16-bit I/O, 8 aux regs (AR0-AR7)
impl Tms320Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Tms320BuildOutput,String>{
    if p.target!="tms320"{return Err(format!("need 'tms320', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xF0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xE8]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0) as u8;
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('a').trim_start_matches('b').parse::<u8>().ok()).unwrap_or(0);
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x80|(rn&7),0]);
            bin.push(0xF0);let pos=bin.len();bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u8;
            bin[last]=here;
            let bra=bin.len();bin.push(0xF0);bin.push(0x00);
            frame.br_indices.push(bra);
            bin.extend(&[0x80|(rn&7),0]);
            bin.push(0xF0);let beq=bin.len();bin.push(0x00);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u8;
            bin[last]=here;
            let bra=bin.len();bin.push(0xF0);bin.push(0x00);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len()as u8;
            for &pos in &frame.br_indices{bin[pos]=target}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x80|(rn&7),0]);
            bin.push(0xF0);let pos=bin.len();bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.push(0xF0);bin.push(frame.start_pos as u8);
            let target=bin.len()as u8;
            for &pos in &frame.br_indices{bin[pos]=target}
            continue;
        }
        bin.extend(match m{
            "add"|"adda"=>vec![0xA0u8|((rp(0)&0x7)<<4)|(rp(1)&0x7),0],
            "sub"|"suba"=>vec![0xB0u8|((rp(0)&0x7)<<4)|(rp(1)&0x7),0],
            "mpy"|"mpya"=>vec![0xC0u8|((rp(0)&0x7)<<4)|(rp(1)&0x7),0],
            "mac"=>vec![0xC8u8|((rp(0)&0x7)<<4)|(rp(1)&0x7),0],
            "ld"|"load"=>vec![0x80|rp(0),0],
            "st"|"store"=>vec![0x90|rp(0),0],
            "ldim"=>vec![0x01,ap(0)],
            "bc"|"jmp"=>vec![0xF0,ap(0) as u8],
            "bcd"=>vec![0xF2,ap(0) as u8],
            "call"=>vec![0xB8,ap(0) as u8],
            "ret"=>vec![0xE8],"nop"=>vec![0x00],
            "mar"=>vec![0x60|(rp(0) as u8)],
            "rpt"|"repeat"=>vec![0xE0|(ap(0) as u8)],
            _=>return Err(format!("unknown tms320 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Tms320BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// Blackfin ADSP-BF5xx: 32-bit, 8 register pairs (R0-R7 paired), SIMD, circular buffers
impl BlackfinBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<BlackfinBuildOutput,String>{
    if p.target!="blackfin"{return Err(format!("need 'blackfin', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x5C,0,0,0]);continue}
        if t=="ret"||t=="rts"{bin.extend(&[0x5D,0,0,0]);continue}
        if t=="hlt"{bin.extend(&[0x00,0,0,0]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let inst=|v:u32|v.to_le_bytes().to_vec();
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&inst(0xE0u32|((rn&7)<<4)|(rn&7)));
            let pos=bin.len();bin.extend(&[0;4]);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u32;
            let enc=(0x58u32|(here&0xFFFFFF)).to_le_bytes();
            bin[last..last+4].copy_from_slice(&enc);
            let bra=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(bra);
            bin.extend(&inst(0xE0u32|((rn&7)<<4)|(rn&7)));
            let beq=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u32;
            let enc=(0x58u32|(here&0xFFFFFF)).to_le_bytes();
            bin[last..last+4].copy_from_slice(&enc);
            let bra=bin.len();bin.extend(&[0;4]);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len()as u32;
            for &pos in &frame.br_indices{
                let enc=(0x50u32|(target&0xFFFFFF)).to_le_bytes();
                bin[pos..pos+4].copy_from_slice(&enc);
            }
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&inst(0xE0u32|((rn&7)<<4)|(rn&7)));
            let pos=bin.len();bin.extend(&[0;4]);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.extend(&(0x50u32|(frame.start_pos as u32&0xFFFFFF)).to_le_bytes());
            let target=bin.len()as u32;
            for &pos in &frame.br_indices{
                let enc=(0x58u32|(target&0xFFFFFF)).to_le_bytes();
                bin[pos..pos+4].copy_from_slice(&enc);
            }
            continue;
        }
        bin.extend(match m{
            "mov" if args.len()==2=>inst(0xE0u32|((rp(0)&7)<<4)|(rp(1)&7)),
            "mov.imm"=>vec![0xE1,0,(rp(0)&7) as u8,ap(1) as u8],
            "add" if args.len()==3=>inst(0x10u32|((rp(0)&7)<<8)|((rp(1)&7)<<4)|(rp(2)&7)),
            "sub" if args.len()==3=>inst(0x20u32|((rp(0)&7)<<8)|((rp(1)&7)<<4)|(rp(2)&7)),
            "mul"|"mpy"=>inst(0x30u32|((rp(0)&7)<<8)|((rp(1)&7)<<4)|(rp(2)&7)),
            "mac"=>inst(0x38u32|((rp(0)&7)<<8)|((rp(1)&7)<<4)|(rp(2)&7)),
            "ld"|"load"=>inst(0x40u32|((rp(0)&7)<<4)|(rp(1)&7)),
            "st"|"store"=>inst(0x48u32|((rp(0)&7)<<4)|(rp(1)&7)),
            "jmp"|"br"=>inst(0x50u32|ap(0)&0xFFFFFF),
            "call"=>inst(0x5Cu32|ap(0)&0xFFFFFF),
            "rts"=>inst(0x5D000000u32),
            "nop"=>inst(0),
            _=>return Err(format!("unknown blackfin '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(BlackfinBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// SHARC ADSP-21000: 32/40-bit floating-point, 10 ALU regs, 10 multiplier regs, SIMD
impl SharcBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<SharcBuildOutput,String>{
    if p.target!="sharc"{return Err(format!("need 'sharc', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xE0,0,0]);continue}
        if t=="ret"||t=="rts"{bin.extend(&[0xE8,0,0]);continue}
        if t=="hlt"{bin.extend(&[0x00,0,0]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').trim_start_matches('f').parse::<u8>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let inst= |v:u32|v.to_le_bytes()[..3].to_vec();
        let pi=|bin:&mut Vec<u8>,pos:usize,op:u32,off:u16|{
            let v=op|(off as u32);let b=v.to_le_bytes();
            bin[pos]=b[0];bin[pos+1]=b[1];bin[pos+2]=b[2];
        };
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&inst(0x80u32|((rn as u32&0xF)<<4)));
            let pos=bin.len();bin.extend(&inst(0));
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u16;
            pi(&mut bin,last,0xD0u32,here);
            let bra=bin.len();bin.extend(&inst(0));
            frame.br_indices.push(bra);
            bin.extend(&inst(0x80u32|((rn as u32&0xF)<<4)));
            let beq=bin.len();bin.extend(&inst(0));
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len()as u16;
            pi(&mut bin,last,0xD0u32,here);
            let bra=bin.len();bin.extend(&inst(0));
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len()as u16;
            for &pos in &frame.br_indices{pi(&mut bin,pos,0xC0u32,target)}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&inst(0x80u32|((rn as u32&0xF)<<4)));
            let pos=bin.len();bin.extend(&inst(0));
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.extend(&inst(0xC0u32|(frame.start_pos as u32&0xFFFF)));
            let target=bin.len()as u16;
            for &pos in &frame.br_indices{pi(&mut bin,pos,0xD0u32,target)}
            continue;
        }
        bin.extend(match m{
            "fadd"|"fadds"=>inst(0x10u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "fsub"|"fsubs"=>inst(0x20u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "fmul"=>inst(0x30u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "fmac"=>inst(0x38u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "fld"|"load"=>inst(0x40u32|((rp(0) as u32&0xF)<<4)|(rp(1) as u32&0xF)),
            "fst"|"store"=>inst(0x50u32|((rp(0) as u32&0xF)<<4)|(rp(1) as u32&0xF)),
            "add"|"iadd"=>inst(0x14u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "sub"|"isub"=>inst(0x24u32|((rp(0) as u32&0xF)<<8)|((rp(1) as u32&0xF)<<4)|(rp(2) as u32&0xF)),
            "cmp"=>inst(0x80u32|((rp(0) as u32&0xF)<<4)|(rp(1) as u32&0xF)),
            "jmp"=>inst(0xC0u32|ap(0)&0xFFFF),
            "call"=>inst(0xE0u32|ap(0)&0xFFFF),
            "rts"=>inst(0xE8u32),
            "nop"=>inst(0),
            _=>return Err(format!("unknown sharc '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(SharcBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
