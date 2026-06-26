// TI C6x VLIW DSP. Fetch packets of 8 x 32-bit instructions executed in
// parallel across functional units (.L, .S, .M, .D). Simplified to 4 units
// and 16 general registers.

use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct VliwBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectVliwBuilder;

enum CfKind { If, While }
struct CfFrame { kind: CfKind, br_indices: Vec<usize>, start_pos: usize, else_label: usize }

#[derive(Clone, Copy, PartialEq)]
enum VReg { A0,A1,A2,A3,A4,A5,A6,A7,B0,B1,B2,B3,B4,B5,B6,B7 }

fn vrp(s: &str) -> Option<VReg> {
    let s = s.to_lowercase();
    let n: u8 = s[1..].parse().ok()?;
    match s.chars().next()? {
        'a' if n <= 7 => Some(match n {0=>VReg::A0,1=>VReg::A1,2=>VReg::A2,3=>VReg::A3,4=>VReg::A4,5=>VReg::A5,6=>VReg::A6,7=>VReg::A7,_=>return None}),
        'b' if n <= 7 => Some(match n {0=>VReg::B0,1=>VReg::B1,2=>VReg::B2,3=>VReg::B3,4=>VReg::B4,5=>VReg::B5,6=>VReg::B6,7=>VReg::B7,_=>return None}),
        _ => None,
    }
}
fn vrn(r: VReg) -> u32 { r as u32 }

#[derive(Clone)]
struct VInst {
    op: u8,
    dst: u8,
    src1: u8,
    src2: u8,
    func: u8,     // functional unit: 0=L, 1=S, 2=M, 3=D
    side: u8,     // 0=A side, 1=B side, 2=cross
}

fn venc(i: &VInst) -> Vec<u8> {
    let mut w = (i.op as u32) << 4 | (i.func as u32) << 2 | i.side as u32;
    w |= (i.dst as u32) << 23 | (i.src1 as u32) << 18 | (i.src2 as u32) << 13;
    w.to_le_bytes().to_vec()
}

fn vliw_cb(reg: u8, side: u8) -> VInst { VInst { op: 0x0E, dst: reg, src1: 0, src2: 0, func: 0, side } }
fn vliw_ub() -> VInst { VInst { op: 0x0F, dst: 0, src1: 0, src2: 0, func: 0, side: 0 } }
fn vliw_patch(inst: &mut VInst, target: usize) {
    let t = target as u32;
    if inst.op == 0x0E {
        inst.side = (t & 3) as u8;
        inst.func = ((t >> 2) & 3) as u8;
        inst.src2 = ((t >> 4) & 0x1F) as u8;
        inst.src1 = ((t >> 9) & 0x1F) as u8;
    } else {
        inst.dst = (t & 0x1F) as u8;
        inst.side = ((t >> 5) & 3) as u8;
        inst.func = ((t >> 7) & 3) as u8;
        inst.src2 = ((t >> 9) & 0x1F) as u8;
        inst.src1 = ((t >> 14) & 0x1F) as u8;
    }
}

fn vpack(insts: &[VInst]) -> Vec<u8> {
    let mut bin = Vec::new();
    for chunk in insts.chunks(8) {
        for inst in chunk { bin.extend(venc(inst)); }
        // Pad to 8 instructions (32 bytes per fetch packet)
        while bin.len() % 32 != 0 { bin.extend_from_slice(&[0u8; 4]); }
    }
    bin
}

fn vlower(t: &str) -> Result<VInst, String> {
    let t = t.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    let parts: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
    if parts.is_empty() { return Err("".into()); }
    let m = parts[0];
    let joined = parts[1..].join(" ");
    let args: Vec<&str> = joined.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();
    let gr = |s: &str| vrp(s).ok_or_else(|| format!("bad reg '{}'", s));
    Ok(match m {
        "add" if args.len() == 3 => {
            let d = gr(args[0])?; let s1 = gr(args[1])?; let s2 = gr(args[2])?;
            let (func, side) = if vrn(d) < 8 { (0, 0) } else { (0, 1) };
            VInst { op: 0x01, dst: vrn(d) as u8 % 8, src1: vrn(s1) as u8 % 8, src2: vrn(s2) as u8 % 8, func, side }
        }
        "sub" if args.len() == 3 => {
            let d = gr(args[0])?; let s1 = gr(args[1])?; let s2 = gr(args[2])?;
            let (func, side) = if vrn(d) < 8 { (0, 0) } else { (0, 1) };
            VInst { op: 0x02, dst: vrn(d) as u8 % 8, src1: vrn(s1) as u8 % 8, src2: vrn(s2) as u8 % 8, func, side }
        }
        "mpy"|"mul" if args.len() == 3 => {
            let d = gr(args[0])?; let s1 = gr(args[1])?; let s2 = gr(args[2])?;
            VInst { op: 0x03, dst: vrn(d) as u8 % 8, src1: vrn(s1) as u8 % 8, src2: vrn(s2) as u8 % 8, func: 2, side: if vrn(d) < 8 { 0 } else { 1 } }
        }
        "ldw"|"ld" if args.len() == 2 => {
            let d = gr(args[0])?; let s = gr(args[1])?;
            VInst { op: 0x10, dst: vrn(d) as u8 % 8, src1: vrn(s) as u8 % 8, src2: 0, func: 3, side: if vrn(d) < 8 { 0 } else { 1 } }
        }
        "stw"|"st" if args.len() == 2 => {
            let s = gr(args[0])?; let d = gr(args[1])?;
            VInst { op: 0x11, dst: vrn(s) as u8 % 8, src1: vrn(d) as u8 % 8, src2: 0, func: 3, side: if vrn(s) < 8 { 0 } else { 1 } }
        }
        "b"|"jmp" if args.len() == 1 => VInst { op: 0x20, dst: 0, src1: 0, src2: 0, func: 1, side: 0 },
        "ret" => VInst { op: 0x21, dst: 0, src1: 0, src2: 0, func: 1, side: 0 },
        "nop" => VInst { op: 0x00, dst: 0, src1: 0, src2: 0, func: 0, side: 0 },
        "mv"|"mov" if args.len() == 2 => {
            let d = gr(args[0])?; let s = gr(args[1])?;
            VInst { op: 0x01, dst: vrn(d) as u8 % 8, src1: vrn(s) as u8 % 8, src2: 0, func: 0, side: if vrn(d) < 8 { 0 } else { 1 } }
        }
        _ => return Err(format!("unknown vliw '{}'", m)),
    })
}

impl DirectVliwBuilder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<VliwBuildOutput, String> {
        if p.target != "vliw" { return Err(format!("need 'vliw', got '{}'", p.target)); }
        let mut insts = Vec::new();
        let mut cf_stack: Vec<CfFrame> = Vec::new();
        let mut else_counter: usize = 0;
        for b in &p.blocks {
            for l in &b.lines {
                let t = l.trim();
                if t.is_empty() || t.starts_with(';') || t.ends_with(':') { continue; }
                if let Some(_) = t.strip_prefix("emit ") { insts.push(VInst { op: 0x20, dst: 0, src1: 0, src2: 0, func: 1, side: 0 }); continue; }
                if let Some(_) = t.strip_prefix("call ") { insts.push(VInst { op: 0x20, dst: 0, src1: 0, src2: 0, func: 1, side: 0 }); continue; }
                if t == "ret" { insts.push(VInst { op: 0x21, dst: 0, src1: 0, src2: 0, func: 1, side: 0 }); continue; }
                if t.starts_with("if ") { let n = t[3..].trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg in if".to_string())?; let reg = n % 8; let side = if n < 8 { 0 } else { 1 }; let start_pos = insts.len(); insts.push(vliw_cb(reg, side)); let idx = insts.len() - 1; cf_stack.push(CfFrame { kind: CfKind::If, br_indices: vec![idx], start_pos, else_label: { let v = else_counter; else_counter += 1; v } }); continue; }
                if t.starts_with("elif ") { let n = t[5..].trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg in elif".to_string())?; let reg = n % 8; let side = if n < 8 { 0 } else { 1 }; let frame = cf_stack.last_mut().ok_or("elif without if".to_string())?; if !matches!(frame.kind, CfKind::If) { return Err("elif in non-if".to_string()); } let last = frame.br_indices.pop().ok_or("no branch to patch".to_string())?; let here = insts.len(); vliw_patch(&mut insts[last], here); let bra_idx = insts.len(); insts.push(vliw_ub()); frame.br_indices.push(bra_idx); insts.push(vliw_cb(reg, side)); let beq_idx = insts.len() - 1; frame.br_indices.push(beq_idx); continue; }
                if t == "else" { let frame = cf_stack.last_mut().ok_or("else without if".to_string())?; if !matches!(frame.kind, CfKind::If) { return Err("else in non-if".to_string()); } let last = frame.br_indices.pop().ok_or("no branch to patch".to_string())?; let here = insts.len(); vliw_patch(&mut insts[last], here); let bra_idx = insts.len(); insts.push(vliw_ub()); frame.br_indices.push(bra_idx); continue; }
                if t == "endif" { let frame = cf_stack.pop().ok_or("endif without if".to_string())?; if !matches!(frame.kind, CfKind::If) { return Err("endif for non-if".to_string()); } let target = insts.len(); for &idx in &frame.br_indices { vliw_patch(&mut insts[idx], target); } continue; }
                if t.starts_with("while ") { let n = t[6..].trim_start_matches('r').parse::<u8>().map_err(|_| "bad reg in while".to_string())?; let reg = n % 8; let side = if n < 8 { 0 } else { 1 }; let start_pos = insts.len(); insts.push(vliw_cb(reg, side)); let idx = insts.len() - 1; cf_stack.push(CfFrame { kind: CfKind::While, br_indices: vec![idx], start_pos, else_label: { let v = else_counter; else_counter += 1; v } }); continue; }
                if t == "endwhile" { let frame = cf_stack.pop().ok_or("endwhile without while".to_string())?; if !matches!(frame.kind, CfKind::While) { return Err("endwhile for non-while".to_string()); } let bra_idx = insts.len(); insts.push(vliw_ub()); vliw_patch(&mut insts[bra_idx], frame.start_pos); let here = insts.len(); for &idx in &frame.br_indices { vliw_patch(&mut insts[idx], here); } continue; }
                match vlower(t) { Ok(i) => insts.push(i), Err(_) => return Err(format!("vliw: '{}'", t)) }
            }
        }
        if !cf_stack.is_empty() { return Err("unclosed cf frame".to_string()); }
        let bin = vpack(&insts);
        std::fs::write(out, &bin).map_err(|e| e.to_string())?;
        Ok(VliwBuildOutput { bin_path: out.to_path_buf(), bin_size: bin.len() })
    }
}
