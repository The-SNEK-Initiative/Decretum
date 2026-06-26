// Balanced Ternary - inspired by Setun
// Each trit: -1 (T), 0, +1 (1). Stored as 2 bits: T=0b10, 0=0b00, 1=0b01
// Tryte = 6 trits, instruction = 8 trits. 8 general-purpose ternary registers (R0-R7) plus PC and flags.
// my favorite target to be honest

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct TernaryBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectTernaryBuilder;

impl DirectTernaryBuilder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<TernaryBuildOutput, String> {
        if p.target != "ternary" { return Err(format!("need 'ternary', got '{}'", p.target)); }
        let k = tassemble(p)?;
        std::fs::write(out, &k).map_err(|e| e.to_string())?;
        Ok(TernaryBuildOutput { bin_path: out.to_path_buf(), bin_size: k.len() })
    }
}

// Balanced trit encoding: T=-1, 0, 1 as 2-bit packed values
fn trit_encode(v: i8) -> u8 {
    match v { -1 => 0b10, 0 => 0b00, 1 => 0b01, _ => 0b00 }
}

// Pack trits (each as 2 bits) into bytes, 4 trits per byte
fn pack_trits(trits: &[i8]) -> Vec<u8> {
    let mut bits: Vec<u8> = Vec::new();
    for &t in trits { bits.push(trit_encode(t)); }
    // Pack into bytes
    let mut bytes = Vec::new();
    for chunk in bits.chunks(4) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() { byte |= b << (i * 2); }
        bytes.push(byte);
    }
    bytes
}

#[derive(Clone, Copy)]
enum TReg { R0, R1, R2, R3, R4, R5, R6, R7, Pc, Fl }
fn trp(s: &str) -> Option<TReg> {
    Some(match s.to_lowercase().as_str() {
        "r0" => TReg::R0, "r1" => TReg::R1, "r2" => TReg::R2,
        "r3" => TReg::R3, "r4" => TReg::R4, "r5" => TReg::R5,
        "r6" => TReg::R6, "r7" => TReg::R7,
        "pc" => TReg::Pc, "fl" | "flags" => TReg::Fl, _ => return None
    })
}
// Register encoded as 2 trits (balanced ternary value covering 0..9)
fn reg2trits(r: TReg) -> Vec<i8> {
    let v = r as i8; // 0..8
    // Encode v as 2-trit balanced ternary
    let t0 = ((v % 3 + 3) % 3) as i8;
    let t1 = ((v / 3) % 3) as i8;
    vec![
        match t0 { 0 => 0, 1 => 1, 2 => -1, _ => 0 },
        match t1 { 0 => 0, 1 => 1, 2 => -1, _ => 0 },
    ]
}

// 2-trit opcodes: ADD=01, SUB=0T(-1), MOV=1T, JMP=11, JZ=10, JE=-11, HLT=-10, NOP=00
#[derive(Clone)]
enum TInst {
    Label(String), Bytes(Vec<u8>),
    // 3-address: op src1 src2 dst (each 2 trits = 6 trits total for regs)
    TAdd(TReg, TReg, TReg), TSub(TReg, TReg, TReg), TMov(TReg, TReg),
    TJmp(String), TJz(String, TReg), TJe(String, TReg),
    TNop, THlt,
    // Immediate load: set register to balanced ternary immediate value (up to 3 trytes = 18 trits)
    TLi(TReg, i64),
}

fn tencode(i: &TInst, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    // Encode a signed integer into balanced ternary trits (least significant first)
    fn encode_bt(val: i32, num_trits: usize) -> Vec<i8> {
        let mut trits = Vec::with_capacity(num_trits);
        let mut v = val;
        for _ in 0..num_trits {
            let rem = ((v % 3 + 3) % 3) as i8;
            let t = match rem { 0 => 0, 1 => 1, 2 => -1, _ => 0 };
            trits.push(t);
            v = (v + match rem { 0 => 0, 1 => 0, 2 => 1, _ => 0 }) / 3;
        }
        trits
    }
    Ok(match i {
        TInst::Label(_) => vec![],
        TInst::Bytes(b) => b.clone(),
        TInst::TAdd(d, s1, s2) => {
            let op = [1, 0]; // ADD = 10
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*s1)); trits.extend(reg2trits(*s2)); trits.extend(reg2trits(*d));
            pack_trits(&trits)
        }
        TInst::TSub(d, s1, s2) => {
            let op = [1, -1]; // SUB = 1T
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*s1)); trits.extend(reg2trits(*s2)); trits.extend(reg2trits(*d));
            pack_trits(&trits)
        }
        TInst::TMov(d, s) => {
            let op = [1, 1]; // MOV = 11
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*s)); trits.extend(reg2trits(*d)); trits.extend([0,0]);
            pack_trits(&trits)
        }
        TInst::TJmp(l) => {
            let tgt = *lm.get(l).ok_or("unknown label")?;
            let rel = tgt.wrapping_sub(off + 2) as i32; // +2 = instruction size
            let op = [-1, 1]; // JMP = T1
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(encode_bt(rel / 2, 6)); // tryte offset, 6 trits
            pack_trits(&trits)
        }
        TInst::TJz(l, reg) => {
            let tgt = *lm.get(l).ok_or("unknown label")?;
            let rel = tgt.wrapping_sub(off + 2) as i32;
            let op = [1, 0]; // JZ = 10
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*reg));
            trits.extend(encode_bt(rel / 2, 4)); // tryte offset, 4 trits
            pack_trits(&trits)
        }
        TInst::TJe(l, reg) => {
            let tgt = *lm.get(l).ok_or("unknown label")?;
            let rel = tgt.wrapping_sub(off + 2) as i32;
            let op = [0, -1]; // JE = 0T
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*reg));
            trits.extend(encode_bt(rel / 2, 4)); // tryte offset, 4 trits
            pack_trits(&trits)
        }
        TInst::THlt => {
            let op = [-1, 0]; // HLT = T0
            pack_trits(&[op[0], op[1], 0,0,0,0,0,0])
        }
        TInst::TNop => pack_trits(&[0,0,0,0,0,0,0,0]),
        TInst::TLi(reg, val) => {
            // LI: op=TT, dst=2 trits, then 4 trits of immediate value
            let op = [-1, -1]; // LI = TT
            let mut trits = vec![]; trits.push(op[0]); trits.push(op[1]);
            trits.extend(reg2trits(*reg));
            let mut v = *val;
            for _ in 0..4 {
                let t = ((v % 3 + 3) % 3) as i8;
                trits.push(match t { 0 => 0, 1 => 1, 2 => -1, _ => 0 });
                v = (v + match t { 0 => 0, 1 => 0, 2 => 1, _ => 0 }) / 3;
            }
            pack_trits(&trits)
        }
    })
}

fn tparse(t: &str) -> Result<TInst, String> {
    let t = t.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(TInst::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();
    let gr = |s: &str| trp(s).ok_or_else(|| format!("bad ternary reg '{}'", s));
    match m {
        "nop" => Ok(TInst::TNop),
        "hlt" => Ok(TInst::THlt),
        "ret" => Ok(TInst::THlt),
        "mov"|"mv" if v.len() == 2 => {
            let d = gr(v[0])?;
            if let Ok(imm) = v[1].parse::<i64>() { Ok(TInst::TLi(d, imm)) }
            else { Ok(TInst::TMov(d, gr(v[1])?)) }
        }
        "add" if v.len() == 3 => Ok(TInst::TAdd(gr(v[0])?, gr(v[1])?, gr(v[2])?)),
        "sub" if v.len() == 3 => Ok(TInst::TSub(gr(v[0])?, gr(v[1])?, gr(v[2])?)),
        "jmp" if v.len() == 1 => Ok(TInst::TJmp(v[0].to_string())),
        "jz" if v.len() == 2 => Ok(TInst::TJz(v[0].to_string(), gr(v[1])?)),
        "je" if v.len() == 2 => Ok(TInst::TJe(v[0].to_string(), gr(v[1])?)),
        "li" if v.len() == 2 => {
            let d = gr(v[0])?;
            let imm = v[1].parse::<i64>().map_err(|_| "bad ternary imm")?;
            Ok(TInst::TLi(d, imm))
        }
        _ => Err(format!("unknown ternary op '{}'", m)),
    }
}

fn tassemble(p: &Program) -> Result<Vec<u8>, String> {
    let mut items: Vec<TInst> = Vec::new();

    struct CfFrame {
        kind: CfKind,
        endif_label: String,
        else_label: String,
        br_indices: Vec<usize>,
        has_else: bool,
    }
    enum CfKind { If, While }
    let mut cf_stack: Vec<CfFrame> = Vec::new();
    let mut cf_counter: u32 = 0;

    items.push(TInst::TJmp(format!("__event_{}", p.entry_event)));
    items.push(TInst::TNop);
    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        items.push(TInst::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') { continue; }
            if t.ends_with(':') { items.push(TInst::Label(format!("{}.{}", b.name, t[..t.len()-1].trim()))); continue; }
            if let Some(x) = t.strip_prefix("emit ") { items.push(TInst::TJmp(format!("__event_{}", x.trim()))); continue; }
            if let Some(x) = t.strip_prefix("call ") { items.push(TInst::TJmp(format!("__proc_{}", x.trim()))); continue; }
            if t == "ret" || t == "hlt" { items.push(TInst::THlt); continue; }

            // if <reg>
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = trp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endif_label = format!("__cf_{}_endif", cf_counter);
                let else_label = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                let br_idx = items.len();
                items.push(TInst::TJz(endif_label.clone(), reg));
                cf_stack.push(CfFrame { kind: CfKind::If, endif_label, else_label, br_indices: vec![br_idx], has_else: false });
                continue;
            }
            // elif <reg>
            if let Some(cond_str) = t.strip_prefix("elif ") {
                let frame = cf_stack.last_mut().ok_or("elif without if")?;
                if frame.has_else { return Err("elif after else".into()); }
                let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.br_indices.len());
                cf_counter += 1;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let TInst::TJz(ref mut l, _) = items[prev] { *l = elif_lbl.clone(); }
                items.push(TInst::TJmp(frame.endif_label.clone()));
                items.push(TInst::Label(elif_lbl));
                let reg = trp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let br_idx = items.len();
                items.push(TInst::TJz(frame.endif_label.clone(), reg));
                frame.br_indices.push(br_idx);
                continue;
            }
            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".into()); }
                frame.has_else = true;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let TInst::TJz(ref mut l, _) = items[prev] { *l = frame.else_label.clone(); }
                items.push(TInst::TJmp(frame.endif_label.clone()));
                items.push(TInst::Label(frame.else_label.clone()));
                continue;
            }
            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
                items.push(TInst::Label(frame.endif_label.clone()));
                continue;
            }
            // while <reg>
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = trp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                items.push(TInst::Label(start_lbl));
                let br_idx = items.len();
                items.push(TInst::TJz(endwhile_lbl.clone(), reg));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
                continue;
            }
            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                items.push(TInst::TJmp(start_lbl));
                items.push(TInst::Label(frame.endif_label.clone()));
                continue;
            }

            match tparse(t) { Ok(i) => items.push(i), Err(_) => return Err(format!("ternary: '{}'", t)) }
        }
    }

    if !cf_stack.is_empty() { return Err("unclosed if/while block".into()); }
    crate::direct_peephole::peephole(&mut items,
        |i| matches!(i, TInst::TNop),
        |i| matches!(i, TInst::TJmp(_)),
        |i| matches!(i, TInst::THlt),
        |i| matches!(i, TInst::Label(_)),
    );
    items.push(TInst::Label("__data".to_string()));
    for d in &p.data {
        match d {
            DataDecl::String { name, value } => {
                let mut b = crate::direct_arch::expand_str(value); b.push(0);
                // Convert bytes to ternary representation
                items.push(TInst::Label(format!("__data_{}", name)));
                let mut trits = Vec::new();
                for byte in &b {
                    let mut v = *byte as i16;
                    for _ in 0..6 { // 6 trits per byte in balanced ternary
                        let t = ((v % 3 + 3) % 3) as i8;
                        trits.push(match t { 0 => 0, 1 => 1, 2 => -1, _ => 0 });
                        v = (v + match t { 0 => 0, 1 => 0, 2 => 1, _ => 0 }) / 3;
                    }
                }
                items.push(TInst::Bytes(pack_trits(&trits)));
            }
            DataDecl::Scalar { name, width, value } => {
                items.push(TInst::Label(format!("__data_{}", name)));
                let raw = match width {
                    ScalarWidth::Byte => vec![*value as u8],
                    ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                    ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(),
                    ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
                };
                let mut trits = Vec::new();
                for byte in &raw {
                    let mut v = *byte as i16;
                    for _ in 0..6 {
                        let t = ((v % 3 + 3) % 3) as i8;
                        trits.push(match t { 0 => 0, 1 => 1, 2 => -1, _ => 0 });
                        v = (v + match t { 0 => 0, 1 => 0, 2 => 1, _ => 0}) / 3;
                    }
                }
                items.push(TInst::Bytes(pack_trits(&trits)));
            }
            DataDecl::Buffer { name, size } => {
                items.push(TInst::Label(format!("__data_{}", name)));
                let trits = vec![0i8; size * 6];
                items.push(TInst::Bytes(pack_trits(&trits)));
            }
        }
    }
    let mut lm = BTreeMap::new();
    let mut o: u32 = 0;
    // Each instruction is 8 trits = 2 bytes
    for i in &items {
        match i {
            TInst::Label(n) => { lm.insert(n.clone(), o); }
            TInst::Bytes(b) => o += b.len() as u32,
            _ => o += 2, // 8 trits = 2 bytes
        }
    }
    let mut bin = Vec::new();
    for i in &items {
        match i {
            TInst::Label(_) => {},
            TInst::Bytes(b) => bin.extend_from_slice(b),
            _ => {
                let mut enc = tencode(i, bin.len() as u32, &lm)?;
                // Pad to 2 bytes
                while enc.len() < 2 { enc.push(0); }
                if enc.len() > 2 { enc.truncate(2); }
                bin.extend_from_slice(&enc);
            }
        }
    }
    Ok(bin)
}
