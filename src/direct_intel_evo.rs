// Intel evolution: 4004 (4-bit), 8008 (8-bit), 8080 (8-bit), 8086 (16-bit original)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct I4004BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct I4004Builder;
pub struct I8008BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct I8008Builder;
pub struct I8080BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct I8080Builder;
pub struct I8086BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct I8086Builder;

// Intel 4004: 8-bit instruction words, 4-bit data, 16 x 4-bit register file
impl I4004Builder{pub fn build_bin(p:&Program,out:&Path)->Result<I4004BuildOutput,String>{
    if p.target!="i4004"{return Err(format!("need 'i4004', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x20,0x00]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x00]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let _=rn;let el=cf_counter;cf_counter+=1;
            bin.push(0x10);
            let pos=bin.len()-1;
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let _=rn;let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len() as u8;
            bin[last]=0x10|(here&0x0F);
            let bra=bin.len();bin.push(0x30);
            frame.br_indices.push(bra);
            let beq=bin.len();bin.push(0x10);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len() as u8;
            bin[last]=0x10|(here&0x0F);
            let bra=bin.len();bin.push(0x30);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len() as u8;
            for &pos in &frame.br_indices{bin[pos]=0x30|(target&0x0F)}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let _=rn;let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.push(0x10);
            let pos=bin.len()-1;
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.push(0x30|(frame.start_pos as u8&0x0F));
            let target=bin.len() as u8;
            for &pos in &frame.br_indices{bin[pos]=0x10|(target&0x0F)}
            continue;
        }
        bin.extend(match m{
            "ldm"=>vec![0xD0|ap(0)], // Load immediate
            "add"=>vec![0x80|ap(0)], // Add register
            "sub"=>vec![0x90|ap(0)], // Sub register
            "ld"|"mov"=>vec![0xA0|ap(0)], // Load reg from acc
            "wr0"=>vec![0xC0],"wr1"=>vec![0xC1],"wr2"=>vec![0xC2],"wr3"=>vec![0xC3], // Write index
            "rd0"=>vec![0xC4],"rd1"=>vec![0xC5],"rd2"=>vec![0xC6],"rd3"=>vec![0xC7], // Read index
            "jcn"|"jmps"=>vec![0x10|ap(0)], // Jump condition
            "jmp"=>vec![0x30|ap(0)], // Jump
            "jun"=>vec![0x40|ap(0)], // Unconditional jump
            "jms"=>vec![0x50|ap(0)], // Jump to subroutine
            "bbl"|"ret"=>vec![0x70|ap(0)], // Branch back + load
            "nop"=>vec![0x00],
            _=>return Err(format!("unknown i4004 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(I4004BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// Intel 8008: 8-bit, accumulator-based, 14-bit addressing, 7 regs (A,B,C,D,E,H,L)
impl I8008Builder{pub fn build_bin(p:&Program,out:&Path)->Result<I8008BuildOutput,String>{
    if p.target!="i8008"{return Err(format!("need 'i8008', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xCD,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xC9]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|s:&str|->u8{match s{"a"|"A"=>7,"b"|"B"=>0,"c"|"C"=>1,"d"|"D"=>2,"e"|"E"=>3,"h"|"H"=>4,"l"|"L"=>5,"m"|"M"=>6,_=>0}};
        let ra=|i:usize|rp(args.get(i).unwrap_or(&""));
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let pos=bin.len();bin.extend(&[0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=(here&0xFF)as u8;bin[last+1]=((here>>8)&0xFF)as u8;
            let bra=bin.len();bin.push(0xC4);bin.extend(&[0,0]);
            frame.br_indices.push(bra);
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let beq=bin.len();bin.extend(&[0,0]);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=(here&0xFF)as u8;bin[last+1]=((here>>8)&0xFF)as u8;
            let bra=bin.len();bin.push(0xC4);bin.extend(&[0,0]);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=(target&0xFF)as u8;bin[pos+1]=((target>>8)&0xFF)as u8}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let pos=bin.len();bin.extend(&[0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.push(0xC4);bin.push((frame.start_pos&0xFF)as u8);bin.push(((frame.start_pos>>8)&0xFF)as u8);
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=(target&0xFF)as u8;bin[pos+1]=((target>>8)&0xFF)as u8}
            continue;
        }
        bin.extend(match m{
            "mov"|"ld" if args.len()==2=>vec![0xC0|(ra(0)<<3)|ra(1)],
            "mvi"|"ldi"=>vec![0xC6|(ra(0)<<3),ap(1)],
            "add"=>vec![0x80|ra(0)],"sub"=>vec![0x90|ra(0)],
            "adi"=>vec![0xC6,ap(0)],"sui"=>vec![0xD6,ap(0)],
            "cmp"=>vec![0xB8|ra(0)],"cpi"=>vec![0xFE,ap(0)],
            "jmp"=>vec![0xC4,ap(0),ap(1)],"jz"=>vec![0xCA,ap(0),ap(1)],"jnz"=>vec![0xC2,ap(0),ap(1)],
            "call"=>vec![0xCD,ap(0),ap(1)],"ret"=>vec![0xC9],
            "inr"=>vec![0x04|(ra(0)<<3)],"dcr"=>vec![0x04|(ra(0)<<3)|0x08],
            "in"=>vec![0x40|ap(0)],"out"=>vec![0x41|ap(0)],
            "nop"=>vec![0x00],
            _=>return Err(format!("unknown i8008 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(I8008BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// Intel 8080: same as 8008 but extended (8-bit regs A-D-E-H-L + M = (HL), flags, stack)
impl I8080Builder{pub fn build_bin(p:&Program,out:&Path)->Result<I8080BuildOutput,String>{
    if p.target!="i8080"{return Err(format!("need 'i8080', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xCD,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0xC9]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|s:&str|->u8{match s{"b"=>0,"c"=>1,"d"=>2,"e"=>3,"h"=>4,"l"=>5,"m"=>6,"a"=>7,_=>7}};
        let ra=|i:usize|rp(args.get(i).unwrap_or(&""));
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let pos=bin.len();bin.extend(&[0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=(here&0xFF)as u8;bin[last+1]=((here>>8)&0xFF)as u8;
            let bra=bin.len();bin.push(0xC3);bin.extend(&[0,0]);
            frame.br_indices.push(bra);
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let beq=bin.len();bin.extend(&[0,0]);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=(here&0xFF)as u8;bin[last+1]=((here>>8)&0xFF)as u8;
            let bra=bin.len();bin.push(0xC3);bin.extend(&[0,0]);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=(target&0xFF)as u8;bin[pos+1]=((target>>8)&0xFF)as u8}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x3E,0x00,0xB8|(rn&7)]);
            bin.push(0xCA);let pos=bin.len();bin.extend(&[0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.push(0xC3);bin.push((frame.start_pos&0xFF)as u8);bin.push(((frame.start_pos>>8)&0xFF)as u8);
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=(target&0xFF)as u8;bin[pos+1]=((target>>8)&0xFF)as u8}
            continue;
        }
        bin.extend(match m{
            "mov" if args.len()==2=>vec![0x40|(ra(0)<<3)|ra(1)],
            "mvi"=>vec![0x06|(ra(0)<<3),ap(1)],
            "lxi"=>vec![0x01|(ra(0)<<4),ap(1),ap(2)],
            "ldax"=>vec![0x0A], // A from (DE)
            "stax"=>vec![0x02], // (DE) from A
            "add"=>vec![0x80|ra(1)],"sub"=>vec![0x90|ra(1)],
            "adi"=>vec![0xC6,ap(0)],"sui"=>vec![0xD6,ap(0)],
            "cmp"=>vec![0xB8|ra(0)],"cpi"=>vec![0xFE,ap(0)],
            "jmp"=>vec![0xC3,ap(0),ap(1)],"jz"=>vec![0xCA,ap(0),ap(1)],"jnz"=>vec![0xC2,ap(0),ap(1)],
            "call"=>vec![0xCD,ap(0),ap(1)],"ret"=>vec![0xC9],
            "inr"=>vec![0x04|(ra(0)<<3)],"dcr"=>vec![0x05|(ra(0)<<3)],
            "inx"=>vec![0x03|(ra(0)<<4)],"dcx"=>vec![0x0B|(ra(0)<<4)],
            "dad"=>vec![0x09|(ra(0)<<4)],
            "ana"=>vec![0xA0|ra(1)],"ora"=>vec![0xB0|ra(1)],"xra"=>vec![0xA8|ra(1)],
            "ani"=>vec![0xE6,ap(0)],"ori"=>vec![0xF6,ap(0)],"xri"=>vec![0xEE,ap(0)],
            "push"=>vec![0xC5|(ra(0)<<4)],"pop"=>vec![0xC1|(ra(0)<<4)],
            "xthl"=>vec![0xE3],"sphl"=>vec![0xF9],"pchl"=>vec![0xE9],
            "sta"=>vec![0x32,ap(0),ap(1)],"lda"=>vec![0x3A,ap(0),ap(1)],
            "shld"=>vec![0x22,ap(0),ap(1)],"lhld"=>vec![0x2A,ap(0),ap(1)],
            "cma"=>vec![0x2F],"stc"=>vec![0x37],"cmc"=>vec![0x3F],
            "di"=>vec![0xF3],"ei"=>vec![0xFB],
            "hlt"=>vec![0x76],"nop"=>vec![0x00],
            _=>return Err(format!("unknown i8080 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(I8080BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// Intel 8086 (original 16-bit, real mode, segment:offset addressing)
impl I8086Builder{pub fn build_bin(p:&Program,out:&Path)->Result<I8086BuildOutput,String>{
    if p.target!="i8086"{return Err(format!("need 'i8086', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xE8,0,0]);continue}
        if t=="ret"{bin.extend(&[0xC3]);continue}
        if t=="hlt"{bin.extend(&[0xF4]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|s:&str|->u8{match s.to_lowercase().as_str(){"al"=>0,"cl"=>1,"dl"=>2,"bl"=>3,"ah"=>4,"ch"=>5,"dh"=>6,"bh"=>7,
            "ax"=>0,"cx"=>1,"dx"=>2,"bx"=>3,"sp"=>4,"bp"=>5,"si"=>6,"di"=>7,_=>0}};
        let ra=|i:usize|rp(args.get(i).unwrap_or(&""));
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        let aw=|i:usize|args.get(i).and_then(|s|s.parse::<u16>().ok()).unwrap_or(0);
        let is_8r=|s:&str|matches!(s.to_lowercase().as_str(),"al"|"cl"|"dl"|"bl"|"ah"|"ch"|"dh"|"bh");
        let wide=|i:usize|if is_8r(args.get(i).unwrap_or(&"")){0}else{1};
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x84,0xC0|((rn&7)<<3)|(rn&7)]);
            bin.push(0x74);let pos=bin.len();bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=here.wrapping_sub(last+1)as u8;
            let bra=bin.len();bin.push(0xEB);bin.push(0x00);
            frame.br_indices.push(bra);
            bin.extend(&[0x84,0xC0|((rn&7)<<3)|(rn&7)]);
            bin.push(0x74);let beq=bin.len();bin.push(0x00);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();
            bin[last]=here.wrapping_sub(last+1)as u8;
            let bra=bin.len();bin.push(0xEB);bin.push(0x00);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=target.wrapping_sub(pos+1)as u8}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&[0x84,0xC0|((rn&7)<<3)|(rn&7)]);
            bin.push(0x74);let pos=bin.len();bin.push(0x00);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.push(0xEB);
            bin.push(frame.start_pos.wrapping_sub(bin.len()+1)as u8);
            let target=bin.len();
            for &pos in &frame.br_indices{bin[pos]=target.wrapping_sub(pos+1)as u8}
            continue;
        }
        bin.extend(match m{
            "mov" if args.len()==2=>{
                if is_8r(args[0]){vec![0xB0|(ra(0)&7)+(if wide(0)==0{0}else{8}),ap(1)]}
                else{vec![0xB8|ra(0),ap(1),ap(2)]}
            }
            "movr" if args.len()==2=>vec![0x88|(ra(0)<<3)|ra(1)],
            "add" if args.len()==2=>vec![0x00|(ra(0)<<3)|ra(1)],
            "sub" if args.len()==2=>vec![0x28|(ra(0)<<3)|ra(1)],
            "cmp" if args.len()==2=>vec![0x38|(ra(0)<<3)|ra(1)],
            "jmp"|"br"=>vec![0xEB,ap(0)],
            "call"=>vec![0xE8,aw(0) as u8,(aw(0)>>8)as u8],
            "ret"=>vec![0xC3],
            "push"=>vec![0x50|ra(0)],"pop"=>vec![0x58|ra(0)],
            "inc"=>vec![0x40|ra(0)],"dec"=>vec![0x48|ra(0)],
            "nop"=>vec![0x90],"hlt"=>vec![0xF4],"cli"=>vec![0xFA],"sti"=>vec![0xFB],
            _=>return Err(format!("unknown i8086 '{}'",m)),
        });
    }}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(I8086BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
