// Soft cores: Mico32 (Lattice 32-bit RISC), PicoBlaze (Xilinx 8-bit)
// Academic: MMIX (Knuth 64-bit RISC), DLX (Hennessy-Patterson), LC-3 (LC3 educational)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

enum CfKind{If,While}
struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}

pub struct Mico32BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Mico32Builder;
pub struct PicoblazeBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct PicoblazeBuilder;
pub struct MmixBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct MmixBuilder;
pub struct DlxBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct DlxBuilder;
pub struct Lc3BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Lc3Builder;

// Mico32: Lattice 32-bit RISC, 32 GPRs, 3-operand, fixed 32-bit encoding
impl Mico32Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Mico32BuildOutput,String>{
    if p.target!="mico32"{return Err(format!("need 'mico32', got '{}'",p.target))}
    let mut bin=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x04,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x04,0x80,0,0]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();let v=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x03E00000;let here=bin.len()as u32;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);let bv=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&bv.to_be_bytes());let beq_pos=bin.len()-4;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x03E00000;let here=bin.len()as u32;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len()as u32;for &pos in &frame.br_indices{let enc=0x02u32<<26|(target&0x3FFFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();let v=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u32;bin.extend((0x02u32<<26|(back&0x3FFFFFF)).to_be_bytes());let here=bin.len()as u32;for &pos in &frame.br_indices{let word=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);let reg_bits=word&0x03E00000;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[pos..pos+4].copy_from_slice(&patched.to_be_bytes())}continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add"=>{let v=0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "sub"=>{let v=0x01u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "mul"=>{let v=0x02u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "or"=>{let v=0x08u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "and"=>{let v=0x09u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "xor"=>{let v=0x0Au32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11;v.to_be_bytes().to_vec()}
            "sw"|"st"=>{let v=0x2Bu32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF;v.to_be_bytes().to_vec()}
            "lw"|"ld"=>{let v=0x23u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF;v.to_be_bytes().to_vec()}
            "addi"=>{let v=0x10u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF;v.to_be_bytes().to_vec()}
            "beq"=>{let v=0x04u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF;v.to_be_bytes().to_vec()}
            "bne"=>{let v=0x05u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF;v.to_be_bytes().to_vec()}
            "jmp"=>{let v=0x02u32<<26|ap(0)&0x3FFFFFF;v.to_be_bytes().to_vec()}
            "call"=>{let v=0x04u32<<26|ap(0)&0x3FFFFFF;v.to_be_bytes().to_vec()}
            "ret"=>{let v=0x00u32<<26|0x80u32<<16;v.to_be_bytes().to_vec()}
            "nop"=>{let v=0u32;v.to_be_bytes().to_vec()}
            _=>return Err(format!("unknown mico32 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Mico32BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// PicoBlaze: Xilinx 8-bit soft-core, 16 regs (s0-sF), 3-bit ALU op, 8-bit addr
impl PicoblazeBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<PicoblazeBuildOutput,String>{
    if p.target!="picoblaze"{return Err(format!("need 'picoblaze', got '{}'",p.target))}
    let mut bin=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x20,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x00]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();bin.extend(&[0x54|(reg<<4),0]);let pos=bin.len();bin.extend(&[0xE0,0]);cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len()as u16;if bin[last]==0xE0{bin[last]|=((here>>8)&0x0F)as u8;bin[last+1]=here as u8}else{bin[last]|=((here>>8)&0x0F)as u8;bin[last+1]=here as u8}let bra_pos=bin.len();bin.extend(&[0xF0,0]);frame.br_indices.push(bra_pos);bin.extend(&[0x54|(reg<<4),0]);let beq_pos=bin.len();bin.extend(&[0xE0,0]);frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len()as u16;if bin[last]==0xE0{bin[last]|=((here>>8)&0x0F)as u8;bin[last+1]=here as u8}else{bin[last]|=((here>>8)&0x0F)as u8;bin[last+1]=here as u8}let bra_pos=bin.len();bin.extend(&[0xF0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len()as u16;for &pos in &frame.br_indices{if bin[pos]==0xE0{bin[pos]|=((target>>8)&0x0F)as u8;bin[pos+1]=target as u8}else{bin[pos]|=((target>>8)&0x0F)as u8;bin[pos+1]=target as u8}}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();bin.extend(&[0x54|(reg<<4),0]);let pos=bin.len();bin.extend(&[0xE0,0]);cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u16;bin.extend(&[0xF0|((back>>8)&0x0F)as u8,back as u8]);let here=bin.len()as u16;for &pos in &frame.br_indices{bin[pos]|=((here>>8)&0x0F)as u8;bin[pos+1]=here as u8}continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('s').parse::<u8>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        bin.extend(match m{
            "load"|"ld"=>vec![0x80|(rp(0)<<4)|(ap(1)&0xF),0],
            "add"=>vec![0x40|(rp(0)<<4)|(rp(1)&0xF),0],
            "sub"=>vec![0x44|(rp(0)<<4)|(rp(1)&0xF),0],
            "and"=>vec![0x48|(rp(0)<<4)|(rp(1)&0xF),0],
            "or"=>vec![0x4C|(rp(0)<<4)|(rp(1)&0xF),0],
            "xor"=>vec![0x50|(rp(0)<<4)|(rp(1)&0xF),0],
            "compare"|"cmp"=>vec![0x54|(rp(0)<<4)|(rp(1)&0xF),0],
            "jmp"=>vec![0x20|ap(0),0],"call"=>vec![0x30|ap(0),0],
            "ret"=>vec![0x00,0],"nop"=>vec![0x00,0],
            "input"|"in"=>vec![0x60|(rp(0)<<4),0],"output"|"out"=>vec![0x70|(rp(0)<<4),0],
            _=>return Err(format!("unknown picoblaze '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(PicoblazeBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// MMIX: Knuth 64-bit RISC, 32 GPRs, 32-bit instructions, 256 special regs
impl MmixBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<MmixBuildOutput,String>{
    if p.target!="mmix"{return Err(format!("need 'mmix', got '{}'",p.target))}
    let mut bin=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xC0,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xE8,0,0,0]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();let v=(0xD0u32<<24)|((reg&0xFF)<<16);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x00FF0000;let here=bin.len()as u32;let patched=(0xD0u32<<24)|reg_bits|(here&0xFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);let bv=(0xD0u32<<24)|((reg&0xFF)<<16);bin.extend(&bv.to_be_bytes());let beq_pos=bin.len()-4;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x00FF0000;let here=bin.len()as u32;let patched=(0xD0u32<<24)|reg_bits|(here&0xFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len()as u32;for &pos in &frame.br_indices{let enc=0xC0u32<<24|(target&0xFFFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();let v=(0xD0u32<<24)|((reg&0xFF)<<16);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u32;bin.extend((0xC0u32<<24|(back&0xFFFFFF)).to_be_bytes());let here=bin.len()as u32;for &pos in &frame.br_indices{let word=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);let reg_bits=word&0x00FF0000;let patched=(0xD0u32<<24)|reg_bits|(here&0xFFFF);bin[pos..pos+4].copy_from_slice(&patched.to_be_bytes())}continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').trim_start_matches('$').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let shift=|v:u32|->[u8;4]{v.to_be_bytes()};
        bin.extend(match m{
            "add"=>shift(0x08u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "sub"=>shift(0x09u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "mul"=>shift(0x0Au32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "div"=>shift(0x0Bu32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "ldou"|"ld"=>shift(0x12u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "stou"|"st"=>shift(0x13u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "cmp"=>shift(0x18u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(rp(2)&0xFF)),
            "set"=>shift(0x20u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|0),
            "bne"=>shift(0x32u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(ap(2)&0xFF)),
            "beq"=>shift(0x31u32<<24|(rp(0)&0xFF)<<16|(rp(1)&0xFF)<<8|(ap(2)&0xFF)),
            "jmp"=>shift(0xC0u32<<24|ap(0)&0xFFFFFF),
            "pushj"|"call"=>shift(0xC5u32<<24|(rp(0)&0xFF)<<16|ap(1)&0xFFFF),
            "pop"|"ret"=>shift(0xE8u32<<24|rp(0)&0xFFFFFF),
            "nop"=>shift(0x00),
            _=>return Err(format!("unknown mmix '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(MmixBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// DLX: Hennessy-Patterson educational RISC, 32 GPRs, 3 types (R/I/J)
impl DlxBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<DlxBuildOutput,String>{
    if p.target!="dlx"{return Err(format!("need 'dlx', got '{}'",p.target))}
    let mut bin=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x00,0,0,0x00]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x04,0x80,0,0]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();let v=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x03E00000;let here=bin.len()as u32;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);let bv=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&bv.to_be_bytes());let beq_pos=bin.len()-4;frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let word=u32::from_be_bytes([bin[last],bin[last+1],bin[last+2],bin[last+3]]);let reg_bits=word&0x03E00000;let here=bin.len()as u32;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[last..last+4].copy_from_slice(&patched.to_be_bytes());let bra_pos=bin.len();bin.extend(&[0,0,0,0]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len()as u32;for &pos in &frame.br_indices{let enc=0x02u32<<26|(target&0x3FFFFFF);bin[pos..pos+4].copy_from_slice(&enc.to_be_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();let v=(0x15u32<<26)|((reg&0x1F)<<21);bin.extend(&v.to_be_bytes());let pos=bin.len()-4;cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos as u32;bin.extend((0x02u32<<26|(back&0x3FFFFFF)).to_be_bytes());let here=bin.len()as u32;for &pos in &frame.br_indices{let word=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);let reg_bits=word&0x03E00000;let patched=(0x15u32<<26)|reg_bits|(here&0x1FFFFF);bin[pos..pos+4].copy_from_slice(&patched.to_be_bytes())}continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let b4=|v:u32|v.to_be_bytes();
        bin.extend(match m{
            "add"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x20),
            "sub"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x22),
            "and"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x24),
            "or"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x25),
            "xor"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x26),
            "sll"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x04),
            "srl"=>b4(0x00u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11|0x06),
            "addi"=>b4(0x08u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "andi"=>b4(0x0Cu32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "lw"|"ld"=>b4(0x23u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "sw"|"st"=>b4(0x2Bu32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "beq"=>b4(0x04u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "bne"=>b4(0x05u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "jmp"=>b4(0x02u32<<26|ap(0)&0x3FFFFFF),
            "call"|"jal"=>b4(0x03u32<<26|ap(0)&0x3FFFFFF),
            "nop"=>b4(0x00),
            _=>return Err(format!("unknown dlx '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(DlxBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// LC-3: 16-bit educational ISA, 8 GPRs, TRAP-based syscalls
impl Lc3Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Lc3BuildOutput,String>{
    if p.target!="lc3"{return Err(format!("need 'lc3', got '{}'",p.target))}
    let mut bin=Vec::new();
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut else_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x04,0x00]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x0E,0x00]);continue}
        if t.starts_with("if "){let reg=t[3..].trim_start_matches('r').parse::<u16>().map_err(|_|"bad reg in if".to_string())?;let start_pos=bin.len();let tv=0x1020u16|((reg&7)<<9)|((reg&7)<<6);bin.extend(&tv.to_le_bytes());let pos=bin.len();bin.extend(&[0x00,0x04]);cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t.starts_with("elif "){let reg=t[5..].trim_start_matches('r').parse::<u16>().map_err(|_|"bad reg in elif".to_string())?;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("elif in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len();let offset=((here as i32)-(last as i32)-2)/2;let op=bin[last+1]as u16;let v=(op<<8)|((offset as u16)&0x1FF);bin[last..last+2].copy_from_slice(&v.to_le_bytes());let bra_pos=bin.len();bin.extend(&[0x00,0x0E]);frame.br_indices.push(bra_pos);let tv=0x1020u16|((reg&7)<<9)|((reg&7)<<6);bin.extend(&tv.to_le_bytes());let beq_pos=bin.len();bin.extend(&[0x00,0x04]);frame.br_indices.push(beq_pos);continue}
        if t=="else"{let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("else in non-if".to_string())}let last=frame.br_indices.pop().ok_or("no branch to patch".to_string())?;let here=bin.len();let offset=((here as i32)-(last as i32)-2)/2;let op=bin[last+1]as u16;let v=(op<<8)|((offset as u16)&0x1FF);bin[last..last+2].copy_from_slice(&v.to_le_bytes());let bra_pos=bin.len();bin.extend(&[0x00,0x0E]);frame.br_indices.push(bra_pos);continue}
        if t=="endif"{let frame=cf_stack.pop().ok_or("endif without if".to_string())?;if !matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}let target=bin.len();for &pos in &frame.br_indices{let offset=((target as i32)-(pos as i32)-2)/2;let op=bin[pos+1]as u16;let v=(op<<8)|((offset as u16)&0x1FF);bin[pos..pos+2].copy_from_slice(&v.to_le_bytes())}continue}
        if t.starts_with("while "){let reg=t[6..].trim_start_matches('r').parse::<u16>().map_err(|_|"bad reg in while".to_string())?;let start_pos=bin.len();let tv=0x1020u16|((reg&7)<<9)|((reg&7)<<6);bin.extend(&tv.to_le_bytes());let pos=bin.len();bin.extend(&[0x00,0x04]);cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:{let v=else_counter;else_counter+=1;v}});continue}
        if t=="endwhile"{let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;if !matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}let back=frame.start_pos;let here=bin.len();let boff=((back as i32)-(here as i32)-2)/2;let bv=0x0E00u16|((boff as u16)&0x1FF);bin.extend(&bv.to_le_bytes());let then=bin.len();for &pos in &frame.br_indices{let offset=((then as i32)-(pos as i32)-2)/2;let op=bin[pos+1]as u16;let v=(op<<8)|((offset as u16)&0x1FF);bin[pos..pos+2].copy_from_slice(&v.to_le_bytes())}continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u16>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        bin.extend(match m{
            "add" if args.len()==3=>{let v=0x1000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|(rp(2)&7);v.to_le_bytes().to_vec()}
            "addi"=>{let v=0x1000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|0x20u16|(ap(2)&0x1F);v.to_le_bytes().to_vec()}
            "sub"=>{let v=0x1000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|0x20u16|0x20u16|(rp(2)&7);v.to_le_bytes().to_vec()}
            "and"=>{let v=0x5000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|(rp(2)&7);v.to_le_bytes().to_vec()}
            "or"=>{let v=0x5000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|0x20u16|0x20u16|(rp(2)&7);v.to_le_bytes().to_vec()}
            "ldr"|"ld"=>{let v=0x6000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|ap(2)&0x3F;v.to_le_bytes().to_vec()}
            "str"|"st"=>{let v=0x7000u16|(rp(0)&7)<<9|(rp(1)&7)<<6|ap(2)&0x3F;v.to_le_bytes().to_vec()}
            "lea"=>{let v=0xE000u16|(rp(0)&7)<<9|ap(1)&0x1FF;v.to_le_bytes().to_vec()}
            "jmp"=>{let v=0xC000u16|(rp(0)&7)<<6;v.to_le_bytes().to_vec()}
            "br"|"brnzp"=>{let v=0x0E00u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brn"=>{let v=0x0800u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brz"=>{let v=0x0400u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brp"=>{let v=0x0200u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brnp"=>{let v=0x0A00u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brzp"=>{let v=0x0600u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "brnz"=>{let v=0x0C00u16|ap(0)&0x1FF;v.to_le_bytes().to_vec()}
            "jsr"=>{let v=0x4000u16|ap(0)&0x7FF;v.to_le_bytes().to_vec()}
            "jsrr"=>{let v=0x4000u16|0x800u16|(rp(0)&7)<<6;v.to_le_bytes().to_vec()}
            "ret"=>vec![0x0E,0x00],"nop"=>vec![0x00,0x00],
            "trap"=>vec![0xF0,0x00],
            _=>return Err(format!("unknown lc3 '{}'",m)),
        });
    }}
    if !cf_stack.is_empty(){return Err("unclosed cf frame".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Lc3BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
