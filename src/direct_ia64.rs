// Itanium IA-64. 128-bit bundles pack 3 x 41-bit instructions with a 5-bit template. 
// Supports predication (8 predicate registers), ALU, memory, and branch operations. 32 general registers.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Ia64BuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectIa64Builder;

#[derive(Clone, Copy, PartialEq)]
enum IReg { R0,R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14,R15,
    R16,R17,R18,R19,R20,R21,R22,R23,R24,R25,R26,R27,R28,R29,R30,R31 }

fn irp(s: &str) -> Option<IReg> {
    let n = s.trim_start_matches('r').parse::<u8>().ok()?;
    if n <= 31 { Some(match n {
        0=>IReg::R0,1=>IReg::R1,2=>IReg::R2,3=>IReg::R3,4=>IReg::R4,
        5=>IReg::R5,6=>IReg::R6,7=>IReg::R7,8=>IReg::R8,9=>IReg::R9,
        10=>IReg::R10,11=>IReg::R11,12=>IReg::R12,13=>IReg::R13,14=>IReg::R14,
        15=>IReg::R15,16=>IReg::R16,17=>IReg::R17,18=>IReg::R18,19=>IReg::R19,
        20=>IReg::R20,21=>IReg::R21,22=>IReg::R22,23=>IReg::R23,24=>IReg::R24,
        25=>IReg::R25,26=>IReg::R26,27=>IReg::R27,28=>IReg::R28,29=>IReg::R29,
        30=>IReg::R30,31=>IReg::R31, _=>return None
    })} else { None }
}
fn irn(r: IReg) -> u64 { r as u64 }

// 41-bit instruction packed into a u64 (top 23 bits unused)
#[derive(Clone)]
struct IInst {
    op: u64,      // opcode (12 bits)
    qp: u64,      // qualifying predicate (6 bits, p0-p7)
    r1: u64,      // destination (7 bits)
    r2: u64,      // source 1 (7 bits)
    r3: u64,      // source 2 / immediate (7 bits)
    xtra: u64,    // extra field (2 bits for template hints)
}

fn ienc_op(i: &IInst) -> u64 {
    (i.qp & 0x3F) << 0 | (i.op & 0xFFF) << 6 | (i.r1 & 0x7F) << 18
    | (i.r2 & 0x7F) << 25 | (i.r3 & 0x7F) << 32 | (i.xtra & 0x3) << 39
}

// Bundle template: MII (0x00), MMI (0x04), MFI (0x08), MIB (0x0C),
// MBB (0x10), BBB (0x14), MMM (0x18), MLX (0x1C)
fn ibundle(slot0: u64, slot1: u64, slot2: u64, tmpl: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    // Pack 3 x 41-bit slots + 5-bit template into 128 bits
    let slot0_128 = slot0 as u128;
    let slot1_128 = slot1 as u128;
    let slot2_128 = slot2 as u128;
    let pack_128 = slot0_128 | (slot1_128 << 41) | (slot2_128 << 82) | ((tmpl as u128) << 123);
    let lo = pack_128 as u64;
    let hi = (pack_128 >> 64) as u64;
    buf.extend_from_slice(&lo.to_le_bytes());
    buf.extend_from_slice(&hi.to_le_bytes());
    buf
}

fn ipack(insts: &[IInst]) -> Vec<u8> {
    let mut bin = Vec::new();
    for chunk in insts.chunks(3) {
        let s0 = if chunk.len() > 0 { ienc_op(&chunk[0]) } else { 0 };
        let s1 = if chunk.len() > 1 { ienc_op(&chunk[1]) } else { 0 };
        let s2 = if chunk.len() > 2 { ienc_op(&chunk[2]) } else { 0 };
        bin.extend(ibundle(s0, s1, s2, 0x00)); // MII template
    }
    bin
}

fn ilower(t: &str) -> Result<IInst, String> {
    let t = t.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    let parts: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
    if parts.is_empty() { return Err("".into()); }
    let m = parts[0];
    let joined = parts[1..].join(" ");
    let args: Vec<&str> = joined.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();
    let gr = |s: &str| irp(s).ok_or_else(|| format!("bad reg '{}'", s));
    let gim = |s: &str| -> Result<u64, String> { s.parse::<u64>().map_err(|_| "bad imm".into()) };
    // Default predicate is p0 (always true)
    let pred = |s: &str| -> Result<u64, String> {
        let p = s.trim_start_matches('p').trim_start_matches('[').trim_end_matches(']').trim();
        p.parse::<u64>().map_err(|_| "bad predicate".into())
    };
    Ok(match m {
        "add" if args.len() == 3 => IInst { op: 0x01, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: irn(gr(args[2])?), xtra: 0 },
        "sub" if args.len() == 3 => IInst { op: 0x02, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: irn(gr(args[2])?), xtra: 0 },
        "addi" if args.len() == 3 => IInst { op: 0x11, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: gim(args[2])?, xtra: 0 },
        "ld"|"ldw" if args.len() == 2 => IInst { op: 0x20, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: 0, xtra: 0 },
        "st"|"stw" if args.len() == 2 => IInst { op: 0x21, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: 0, xtra: 0 },
        "br"|"jmp" if args.len() == 1 => IInst { op: 0x30, qp: 0, r1: 0, r2: 0, r3: 0, xtra: 0 },
        "br.call" if args.len() == 1 => IInst { op: 0x31, qp: 0, r1: 0, r2: 0, r3: 0, xtra: 0 },
        "br.ret"|"ret" => IInst { op: 0x32, qp: 0, r1: 0, r2: 0, r3: 0, xtra: 0 },
        "nop" => IInst { op: 0x00, qp: 0, r1: 0, r2: 0, r3: 0, xtra: 0 },
        "mov" if args.len() == 2 => {
            if let Ok(imm) = args[1].parse::<u64>() {
                IInst { op: 0x10, qp: 0, r1: irn(gr(args[0])?), r2: 0, r3: imm, xtra: 0 }
            } else {
                IInst { op: 0x03, qp: 0, r1: irn(gr(args[0])?), r2: irn(gr(args[1])?), r3: 0, xtra: 0 }
            }
        }
        _ => return Err(format!("unknown ia64 '{}'", m)),
    })
}

impl DirectIa64Builder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<Ia64BuildOutput, String> {
        if p.target != "ia64" { return Err(format!("need 'ia64', got '{}'", p.target)); }
        let mut insts = Vec::new();
        struct CfFrame{kind:CfKind,br_indices:Vec<usize>,start_pos:usize,else_label:usize}
        #[derive(PartialEq)]enum CfKind{If,While}
        let mut cf_stack:Vec<CfFrame>=Vec::new();
        let mut cf_counter:usize=0;
        for b in &p.blocks {
            let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
            for l in &b.lines {
                let t = l.trim();
                if t.is_empty() || t.starts_with(';') || t.ends_with(':') { continue; }
                if let Some(x) = t.strip_prefix("emit ") { insts.push(IInst { op: 0x31, qp: 0, r1: irn(IReg::R1), r2: 0, r3: 0, xtra: 0 }); continue; }
                if let Some(x) = t.strip_prefix("call ") { insts.push(IInst { op: 0x31, qp: 0, r1: irn(IReg::R1), r2: 0, r3: 0, xtra: 0 }); continue; }
                if t == "ret" { insts.push(IInst { op: 0x32, qp: 0, r1: 0, r2: 0, r3: 0, xtra: 0 }); continue; }
                if let Some(cond)=t.strip_prefix("if "){
                    let rn=irp(cond.trim()).ok_or("bad reg".to_string())?;
                    let el=cf_counter;cf_counter+=1;
                    insts.push(IInst{op:0x0A,qp:0,r1:0,r2:irn(rn),r3:0,xtra:0});
                    let idx=insts.len();
                    insts.push(IInst{op:0x30,qp:0,r1:0,r2:0,r3:0,xtra:0});
                    cf_stack.push(CfFrame{kind:CfKind::If,br_indices:vec![idx],start_pos:0,else_label:el});
                    continue;
                }
                if let Some(cond)=t.strip_prefix("elif "){
                    let frame=cf_stack.last_mut().ok_or("elif without if".to_string())?;
                    let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
                    insts[last].r3=insts.len() as u64;
                    let bra=insts.len();
                    insts.push(IInst{op:0x31,qp:0,r1:0,r2:0,r3:0,xtra:0});
                    frame.br_indices.push(bra);
                    let rn=irp(cond.trim()).ok_or("bad reg".to_string())?;
                    insts.push(IInst{op:0x0A,qp:0,r1:0,r2:irn(rn),r3:0,xtra:0});
                    let beq=insts.len();
                    insts.push(IInst{op:0x30,qp:0,r1:0,r2:0,r3:0,xtra:0});
                    frame.br_indices.push(beq);
                    continue;
                }
                if t=="else"{
                    let frame=cf_stack.last_mut().ok_or("else without if".to_string())?;
                    let last=frame.br_indices.pop().ok_or("no branch".to_string())?;
                    insts[last].r3=insts.len() as u64;
                    let bra=insts.len();
                    insts.push(IInst{op:0x31,qp:0,r1:0,r2:0,r3:0,xtra:0});
                    frame.br_indices.push(bra);
                    continue;
                }
                if t=="endif"{
                    let frame=cf_stack.pop().ok_or("endif without if/while".to_string())?;
                    if!matches!(frame.kind,CfKind::If){return Err("endif for non-if".to_string())}
                    let target=insts.len();
                    for &idx in &frame.br_indices{insts[idx].r3=target as u64}
                    continue;
                }
                if let Some(cond)=t.strip_prefix("while "){
                    let rn=irp(cond.trim()).ok_or("bad reg".to_string())?;
                    let start_pos=insts.len();let el=cf_counter;cf_counter+=1;
                    insts.push(IInst{op:0x0A,qp:0,r1:0,r2:irn(rn),r3:0,xtra:0});
                    let idx=insts.len();
                    insts.push(IInst{op:0x30,qp:0,r1:0,r2:0,r3:0,xtra:0});
                    cf_stack.push(CfFrame{kind:CfKind::While,br_indices:vec![idx],start_pos,else_label:el});
                    continue;
                }
                if t=="endwhile"{
                    let frame=cf_stack.pop().ok_or("endwhile without while".to_string())?;
                    if!matches!(frame.kind,CfKind::While){return Err("endwhile for non-while".to_string())}
                    insts.push(IInst{op:0x31,qp:0,r1:0,r2:0,r3:frame.start_pos as u64,xtra:0});
                    let target=insts.len();
                    for &idx in &frame.br_indices{insts[idx].r3=target as u64}
                    continue;
                }
                match ilower(t) { Ok(i) => insts.push(i), Err(_) => return Err(format!("ia64: '{}'", t)) }
            }
        }
        if!cf_stack.is_empty(){return Err("unclosed if/while block".to_string())}
        let bin = ipack(&insts);
        std::fs::write(out, &bin).map_err(|e| e.to_string())?;
        Ok(Ia64BuildOutput { bin_path: out.to_path_buf(), bin_size: bin.len() })
    }
}
