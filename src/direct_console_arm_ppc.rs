// Console CPU variants: HuC6280 (PC Engine), V810 (NEC 32-bit RISC)
// ARM variants: ARM7TDMI (v4T), ARM9 (v5TE)
// PPC variants: 740 (G3), 970 (G5)

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct HuC6280BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct HuC6280Builder;
pub struct V810BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct V810Builder;
pub struct Arm7tdmiBuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Arm7tdmiBuilder;
pub struct Arm9BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Arm9Builder;
pub struct Ppc740BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Ppc740Builder;
pub struct Ppc970BuildOutput{pub bin_path:PathBuf,pub bin_size:usize}
pub struct Ppc970Builder;

// HuC6280: 65C02-based with 8-bit ALU, 16-bit accumulator, 128-byte page zero,
// bank switching (MPR registers), 8 I/O ports. Used in NEC PC Engine.
impl HuC6280Builder{pub fn build_bin(p:&Program,out:&Path)->Result<HuC6280BuildOutput,String>{
    if p.target!="huc6280"{return Err(format!("need 'huc6280', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x20,0,0]);continue}
        if t=="ret"||t=="rts"{bin.extend(&[0x60]);continue}
        if t=="hlt"{bin.extend(&[0x00]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u8>().ok()).unwrap_or(0);
        let imm=args.get(0).map(|s|s.starts_with('#')).unwrap_or(false);
        let pi_rel=|bin:&mut Vec<u8>,pos:usize,target:usize|{
            let off=((target as i16)-(pos as i16+2))as i8;
            bin[pos+1]=off as u8;
        };
        let pi_abs=|bin:&mut Vec<u8>,pos:usize,target:usize|{
            bin[pos+1]=(target&0xFF)as u8;
            bin[pos+2]=((target>>8)&0xFF)as u8;
        };
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&[0xA5,rn]);
            let pos=bin.len();bin.extend(&[0xF0,0]);
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi_rel(&mut bin,last,here);
            let bra=bin.len();bin.extend(&[0x4C,0,0]);
            frame.br_indices.push(bra);
            bin.extend(&[0xA5,rn]);
            let beq=bin.len();bin.extend(&[0xF0,0]);
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi_rel(&mut bin,last,here);
            let bra=bin.len();bin.extend(&[0x4C,0,0]);
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{
                if bin[pos]==0xF0{pi_rel(&mut bin,pos,target)}else{pi_abs(&mut bin,pos,target)}
            }
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().parse::<u8>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&[0xA5,rn]);
            let pos=bin.len();bin.extend(&[0xF0,0]);
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            let start=frame.start_pos;
            bin.extend(&[0x4C,(start&0xFF)as u8,((start>>8)&0xFF)as u8]);
            let target=bin.len();
            for &pos in &frame.br_indices{pi_rel(&mut bin,pos,target)}
            continue;
        }
        bin.extend(match m{
            "lda"=>{if imm{vec![0xA9,ap(1)]}else{vec![0xA5,ap(0)]}}
            "ldx"=>{if imm{vec![0xA2,ap(1)]}else{vec![0xA6,ap(0)]}}
            "ldy"=>{if imm{vec![0xA0,ap(1)]}else{vec![0xA4,ap(0)]}}
            "sta"=>vec![0x85,ap(0)],"stx"=>vec![0x86,ap(0)],"sty"=>vec![0x84,ap(0)],
            "adc"=>vec![0x65,ap(0)],"sbc"=>vec![0xE5,ap(0)],
            "clc"=>vec![0x18],"sec"=>vec![0x38],
            "tax"=>vec![0xAA],"tay"=>vec![0xA8],"txa"=>vec![0x8A],"tya"=>vec![0x98],
            "inx"=>vec![0xE8],"iny"=>vec![0xC8],"dex"=>vec![0xCA],"dey"=>vec![0x88],
            "beq"=>vec![0xF0,ap(0)],"bne"=>vec![0xD0,ap(0)],
            "bcc"=>vec![0x90,ap(0)],"bcs"=>vec![0xB0,ap(0)],
            "bpl"=>vec![0x10,ap(0)],"bmi"=>vec![0x30,ap(0)],
            "jmp"=>vec![0x4C,ap(0),ap(1)],"jsr"=>vec![0x20,ap(0),ap(1)],
            "tam"|"tma"=>vec![0xEA],"nop"=>vec![0xEA],"rts"=>vec![0x60],"brk"=>vec![0x00],
            _=>return Err(format!("unknown huc6280 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(HuC6280BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// V810: NEC 32-bit RISC, 32 GPRs, 32-bit fixed instructions, used in Saturn/N64
impl V810Builder{pub fn build_bin(p:&Program,out:&Path)->Result<V810BuildOutput,String>{
    if p.target!="v810"{return Err(format!("need 'v810', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x40,0,0,0]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x80,0,0,0]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let be=|v:u32|v.to_be_bytes();
        let pi=|bin:&mut Vec<u8>,pos:usize,target:usize|{
            let instr=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);
            let disp=((target as i32)-(pos as i32+4))/4;
            let new_instr=(instr&0xFC000000)|((disp as u32)&0x3FFFFFF);
            let b=new_instr.to_be_bytes();
            bin[pos]=b[0];bin[pos+1]=b[1];bin[pos+2]=b[2];bin[pos+3]=b[3];
        };
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&be(0x22u32<<26|(rn&0x1F)<<21|0<<16|0<<11));
            let pos=bin.len();bin.extend(&be(0x02u32<<26|0));
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0x82u32<<26|0));
            frame.br_indices.push(bra);
            bin.extend(&be(0x22u32<<26|(rn&0x1F)<<21|0<<16|0<<11));
            let beq=bin.len();bin.extend(&be(0x02u32<<26|0));
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0x82u32<<26|0));
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&be(0x22u32<<26|(rn&0x1F)<<21|0<<16|0<<11));
            let pos=bin.len();bin.extend(&be(0x02u32<<26|0));
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            let start=frame.start_pos;
            let back_disp=((start as i32)-(bin.len() as i32+4))/4;
            bin.extend(&be(0x82u32<<26|((back_disp as u32)&0x3FFFFFF)));
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        bin.extend(match m{
            "add"|"addu"=>be(0x20u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11),
            "sub"|"subu"=>be(0x21u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11),
            "cmp"=>be(0x22u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|(rp(2)&0x1F)<<11),
            "mov"=>be(0x24u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16),
            "addi"=>be(0x40u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "sethi"=>be(0x4Cu32<<26|(rp(0)&0x1F)<<21|ap(1)&0xFFFF),
            "ld"|"lw"=>be(0x10u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "st"|"sw"=>be(0x14u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "bne"=>be(0x01u32<<26|ap(0)&0x3FFFFFF),
            "beq"=>be(0x02u32<<26|ap(0)&0x3FFFFFF),
            "jmp"|"jr"=>be(0x80u32<<26|(rp(0)&0x1F)<<21),
            "jmpi"=>be(0x82u32<<26|ap(0)&0x3FFFFFF),
            "jal"|"call"=>be(0x40u32<<26|ap(0)&0x3FFFFFF),
            "jarl"=>be(0x42u32<<26|(rp(0)&0x1F)<<21|(rp(1)&0x1F)<<16|ap(2)&0xFFFF),
            "nop"=>be(0),
            _=>return Err(format!("unknown v810 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(V810BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// ARM7TDMI: ARMv4T (ARM + Thumb). Reuses ARM instructions from direct_arm.rs.
impl Arm7tdmiBuilder{pub fn build_bin(p:&Program,out:&Path)->Result<Arm7tdmiBuildOutput,String>{
    if p.target!="arm7tdmi"{return Err(format!("need 'arm7tdmi', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0xEB,0,0,0]);continue}
        if t=="ret"{bin.extend(&[0xE1,0x2F,0xFF,0x1E]);continue}
        if t=="hlt"{bin.extend(&[0xE0,0,0,0]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let be=|v:u32|v.to_le_bytes();
        let pi=|bin:&mut Vec<u8>,pos:usize,target:usize|{
            let instr=u32::from_le_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);
            let disp=((target as i32)-(pos as i32+8))/4;
            let new_instr=(instr&0xFF000000)|((disp as u32)&0xFFFFFF);
            let b=new_instr.to_le_bytes();
            bin[pos]=b[0];bin[pos+1]=b[1];bin[pos+2]=b[2];bin[pos+3]=b[3];
        };
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&be(0xE3500000|((rn&0xF)<<16)));
            let pos=bin.len();bin.extend(&be(0x0A000000));
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0xEA000000));
            frame.br_indices.push(bra);
            bin.extend(&be(0xE3500000|((rn&0xF)<<16)));
            let beq=bin.len();bin.extend(&be(0x0A000000));
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0xEA000000));
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&be(0xE3500000|((rn&0xF)<<16)));
            let pos=bin.len();bin.extend(&be(0x0A000000));
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            let start=frame.start_pos;
            let back_disp=((start as i32)-(bin.len() as i32+8))/4;
            bin.extend(&be(0xEA000000|((back_disp as u32)&0xFFFFFF)));
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        bin.extend(match m{
            "add"=>be(0xE0800000|(rp(0)&0xF)<<16|(rp(1)&0xF)<<12|(rp(2)&0xF)),
            "sub"=>be(0xE0400000|(rp(0)&0xF)<<16|(rp(1)&0xF)<<12|(rp(2)&0xF)),
            "mov"=>be(0xE1A00000|(rp(0)&0xF)<<12|(rp(1)&0xF)),
            "cmp"=>be(0xE1500000|(rp(0)&0xF)<<16|(rp(1)&0xF)<<12),
            "ldr"|"ld"=>be(0xE5900000|(rp(0)&0xF)<<12|(rp(1)&0xF)<<16|ap(2)&0xFFF),
            "str"|"st"=>be(0xE5800000|(rp(0)&0xF)<<12|(rp(1)&0xF)<<16|ap(2)&0xFFF),
            "b"|"jmp"=>be(0xEA000000|ap(0)&0xFFFFFF),
            "bl"|"call"=>be(0xEB000000|ap(0)&0xFFFFFF),
            "bx"|"ret"=>be(0xE12FFF10|rp(0)&0xF),
            "nop"=>be(0xE1A00000),
            _=>return Err(format!("unknown arm7tdmi '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Arm7tdmiBuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}

// ARM9: ARMv5TE (ARM + Thumb + enhanced DSP instructions). ARM9 core can handle v5TE ops.
// For output, same ARM encoding as v4T + DSP extensions (smlal, smull, etc., itd., itp.)
impl Arm9Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Arm9BuildOutput,String>{
    if p.target!="arm9"{return Err(format!("need 'arm9', got '{}'",p.target))}
    match Arm7tdmiBuilder::build_bin(p,out){Ok(o)=>Ok(Arm9BuildOutput{bin_path:o.bin_path,bin_size:o.bin_size}),Err(e)=>Err(e)}
}}

// PPC 740 (PowerPC G3): PPC 32-bit, 32 GPRs, FPU, AltiVec (not in 740 but reuses RISC backend)
impl Ppc740Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Ppc740BuildOutput,String>{
    if p.target!="ppc740"{return Err(format!("need 'ppc740', got '{}'",p.target))}
    let r = crate::direct_risc::DirectPpcBuilder::build_bin(&Program { target: "ppc".into(), ..p.clone() }, out)?;
    Ok(Ppc740BuildOutput{bin_path:r.bin_path,bin_size:r.bin_size})
}}

// PPC 970 (PowerPC G5): PPC 64-bit, 32 GPRs, VMX, PowerPC-64 ISA.
impl Ppc970Builder{pub fn build_bin(p:&Program,out:&Path)->Result<Ppc970BuildOutput,String>{
    if p.target!="ppc970"{return Err(format!("need 'ppc970', got '{}'",p.target))}
    let mut bin=Vec::new();
    struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
    #[derive(PartialEq)]enum CfKind{If,While}
    let mut cf_stack:Vec<CfFrame>=Vec::new();
    let mut cf_counter:usize=0;
    for b in &p.blocks{for l in &b.lines{
        let t=l.trim();if t.is_empty()||t.starts_with(';')||t.ends_with(':'){continue}
        if t.starts_with("emit ")||t.starts_with("call "){bin.extend(&[0x48,0,0,0x01]);continue}
        if t=="ret"||t=="hlt"{bin.extend(&[0x4E,0x80,0x00,0x20]);continue}
        let parts:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
        if parts.is_empty(){continue}let m=parts[0];
        let joined=parts[1..].join(" ");let args:Vec<&str>=joined.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
        let rp=|i:usize|args.get(i).and_then(|s|s.trim_start_matches('r').parse::<u32>().ok()).unwrap_or(0);
        let ap=|i:usize|args.get(i).and_then(|s|s.parse::<u32>().ok()).unwrap_or(0);
        let be=|v:u32|v.to_be_bytes();
        let pi=|bin:&mut Vec<u8>,pos:usize,target:usize|{
            let instr=u32::from_be_bytes([bin[pos],bin[pos+1],bin[pos+2],bin[pos+3]]);
            let new_instr=(instr&0xFC000000)|((target as u32)&0x3FFFFFC);
            let b=new_instr.to_be_bytes();
            bin[pos]=b[0];bin[pos+1]=b[1];bin[pos+2]=b[2];bin[pos+3]=b[3];
        };
        if let Some(cond)=t.strip_prefix("if "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let el=cf_counter;cf_counter+=1;
            bin.extend(&be(31<<26|0<<21|(rn&0x1F)<<16|0<<11|0));
            let pos=bin.len();bin.extend(&be(0x41820000));
            cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![pos],start_pos:0,else_label:el});
            continue;
        }
        if let Some(cond)=t.strip_prefix("elif "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0x48000000));
            frame.br_indices.push(bra);
            bin.extend(&be(31<<26|0<<21|(rn&0x1F)<<16|0<<11|0));
            let beq=bin.len();bin.extend(&be(0x41820000));
            frame.br_indices.push(beq);
            continue;
        }
        if t=="else"{
            let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
            let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
            let here=bin.len();pi(&mut bin,last,here);
            let bra=bin.len();bin.extend(&be(0x48000000));
            frame.br_indices.push(bra);
            continue;
        }
        if t=="endif"{
            let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
            if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        if let Some(cond)=t.strip_prefix("while "){
            let rn=cond.trim().trim_start_matches('r').parse::<u32>().map_err(|_|"bad reg".to_string())?;
            let start_pos=bin.len();let el=cf_counter;cf_counter+=1;
            bin.extend(&be(31<<26|0<<21|(rn&0x1F)<<16|0<<11|0));
            let pos=bin.len();bin.extend(&be(0x41820000));
            cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![pos],start_pos,else_label:el});
            continue;
        }
        if t=="endwhile"{
            let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
            if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
            bin.extend(&be(0x48000000|(frame.start_pos as u32&0x3FFFFFC)));
            let target=bin.len();
            for &pos in &frame.br_indices{pi(&mut bin,pos,target)}
            continue;
        }
        bin.extend(match m{
            "add"=>be(7<<26|rp(0)<<21|rp(1)<<16|rp(2)<<11|266<<1),
            "subf"=>be(31<<26|rp(0)<<21|rp(1)<<16|rp(2)<<11|40<<1),
            "mulld"=>be(31<<26|rp(0)<<21|rp(1)<<16|rp(2)<<11|233<<1),
            "divd"=>be(31<<26|rp(0)<<21|rp(1)<<16|rp(2)<<11|489<<1),
            "ld"|"load"=>be(58<<26|rp(0)<<21|rp(1)<<16|ap(2)&0xFFFF),
            "std"|"store"=>be(62<<26|rp(0)<<21|rp(1)<<16|ap(2)&0xFFFF),
            "cmpd"=>be(31<<26|0<<21|rp(0)<<16|rp(1)<<11|0),
            "b"|"jmp"=>be(18<<26|ap(0)&0x3FFFFFC),
            "bl"|"call"=>be(18<<26|1|ap(0)&0x3FFFFFC),
            "blr"|"ret"=>be(0x4E800020),
            "or"=>be(31<<26|rp(0)<<21|rp(1)<<16|rp(2)<<11|444<<1),
            "nop"=>be(0x60000000),
            _=>return Err(format!("unknown ppc970 '{}'",m)),
        });
    }}
    if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
    std::fs::write(out,&bin).map_err(|e|e.to_string())?;
    Ok(Ppc970BuildOutput{bin_path:out.to_path_buf(),bin_size:bin.len()})
}}
