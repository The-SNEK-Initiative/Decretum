use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct Aarch64BuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectAarch64Builder;

impl DirectAarch64Builder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<Aarch64BuildOutput, String> {
        if p.target != "aarch64" { return Err(format!("need aarch64, got '{}'", p.target)); }
        let k = ab_assemble(p)?;
        std::fs::write(out, &k).map_err(|e| e.to_string())?;
        Ok(Aarch64BuildOutput { bin_path: out.to_path_buf(), bin_size: k.len() })
    }
    pub fn assemble(p: &Program) -> Result<Vec<u8>, String> {
        ab_assemble(p)
    }
}

#[derive(Clone, Copy)]
enum AReg { X0, X1, X2, X3, X4, X5, X6, X7, X8, X9, X10, X11, X12, X13, X14, X15,
    X16, X17, X18, X19, X20, X21, X22, X23, X24, X25, X26, X27, X28, X29, X30, Xzr }

fn arp(s: &str) -> Option<AReg> {
    Some(match s.to_lowercase().as_str() {
        "x0" => AReg::X0, "x1" => AReg::X1, "x2" => AReg::X2, "x3" => AReg::X3,
        "x4" => AReg::X4, "x5" => AReg::X5, "x6" => AReg::X6, "x7" => AReg::X7,
        "x8" => AReg::X8, "x9" => AReg::X9, "x10" => AReg::X10, "x11" => AReg::X11,
        "x12" => AReg::X12, "x13" => AReg::X13, "x14" => AReg::X14, "x15" => AReg::X15,
        "x16" => AReg::X16, "x17" => AReg::X17, "x18" => AReg::X18, "x19" => AReg::X19,
        "x20" => AReg::X20, "x21" => AReg::X21, "x22" => AReg::X22, "x23" => AReg::X23,
        "x24" => AReg::X24, "x25" => AReg::X25, "x26" => AReg::X26, "x27" => AReg::X27,
        "x28" => AReg::X28, "x29" | "fp" => AReg::X29, "x30" | "lr" => AReg::X30,
        "xzr" | "sp" => AReg::Xzr, _ => return None
    })
}

fn arn(r: AReg) -> u32 { r as u32 }
fn aw(u: u32) -> Vec<u8> { u.to_le_bytes().to_vec() }

fn jmp(op: u32, l: &str, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let t = *lm.get(l).ok_or("unknown label")?;
    let r = t.wrapping_sub(off) as i32;
    Ok(aw(op | (((r as u32) >> 2) & 0x3FFFFFF)))
}

#[derive(Clone)]
enum AInst {
    Label(String), Bytes(Vec<u8>), MovImm(AReg, u64), MovReg(AReg, AReg),
    AddRRR(AReg, AReg, AReg), SubRRR(AReg, AReg, AReg),
    AddImm(AReg, AReg, u32), SubImm(AReg, AReg, u32),
    LdrPc(AReg, String), StrPc(AReg, String), LdrOff(AReg, AReg, i16), StrOff(AReg, AReg, i16),
    B(String), Bl(String), Br(AReg), Ret, Bcc(String, u8), CmpRR(AReg, AReg), CmpImm(AReg, u32),
    Nop, Hlt(u16),
}

fn aenc(i: &AInst, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    Ok(match i {
        AInst::Label(_) => vec![],
        AInst::Bytes(b) => b.clone(),
        AInst::MovImm(r, v) => {
            let d = arn(*r);
            if *v <= 0xFFFF { aw(0xD2800000 | d | ((*v as u32) << 5)) }
            else {
                let mut b = aw(0xD2800000 | d | ((*v as u32) << 5 & 0x1F));
                let w1 = ((*v >> 16) & 0xFFFF) as u32;
                if w1 != 0 { b.extend(aw(0xF2A00000 | d | (w1 << 5))); }
                let w2 = ((*v >> 32) & 0xFFFF) as u32;
                if w2 != 0 { b.extend(aw(0xF2C00000 | d | (w2 << 5))); }
                let w3 = ((*v >> 48) & 0xFFFF) as u32;
                if w3 != 0 { b.extend(aw(0xF2E00000 | d | (w3 << 5))); }
                return Ok(b);
            }
        }
        AInst::MovReg(d, s) => aw(0xAA000000 | (arn(*s) << 16) | arn(*d)),
        AInst::AddRRR(d, n, m) => aw(0x8B000000 | (arn(*m) << 16) | (arn(*n) << 5) | arn(*d)),
        AInst::SubRRR(d, n, m) => aw(0xCB000000 | (arn(*m) << 16) | (arn(*n) << 5) | arn(*d)),
        AInst::AddImm(d, n, i) => aw(0x91000000 | ((*i & 0xFFF) << 10) | (arn(*n) << 5) | arn(*d)),
        AInst::SubImm(d, n, i) => aw(0xD1000000 | ((*i & 0xFFF) << 10) | (arn(*n) << 5) | arn(*d)),
        AInst::LdrPc(r, l) => {
            let t = *lm.get(l).ok_or("unknown")?;
            let rel = t.wrapping_sub(off) as i64;
            let pg = ((rel >> 12) & 0x1FFFFF) as u32;
            let ad = 0x90000000 | ((pg & 3) << 29) | (((pg >> 2) & 0x7FFFF) << 5) | arn(*r);
            let po = (rel & 0xFFF) as u32;
            let ad2 = 0x91000000 | (po << 10) | (arn(*r) << 5) | arn(*r);
            let mut b = aw(ad); b.extend(aw(ad2)); return Ok(b);
        }
        AInst::StrPc(r, l) => {
            let t = *lm.get(l).ok_or("unknown")?;
            let rel = t.wrapping_sub(off) as i64;
            let pg = ((rel >> 12) & 0x1FFFFF) as u32;
            let ad = 0x90000000 | ((pg & 3) << 29) | (((pg >> 2) & 0x7FFFF) << 5) | arn(*r);
            let po = (rel & 0xFFF) as u32;
            let str_op = 0xF9000000 | ((po >> 3) << 10) | (arn(*r) << 5) | arn(*r);
            let mut b = aw(ad); b.extend(aw(str_op)); return Ok(b);
        }
        AInst::LdrOff(d, n, i) => aw(0xF9400000 | (((*i as u32) >> 3) << 10) | (arn(*n) << 5) | arn(*d)),
        AInst::StrOff(d, n, i) => aw(0xF9000000 | (((*i as u32) >> 3) << 10) | (arn(*n) << 5) | arn(*d)),
        AInst::B(l) => return jmp(0x14000000, l, off, lm),
        AInst::Bl(l) => return jmp(0x94000000, l, off, lm),
        AInst::Br(rm) => aw(0xD61F0000 | (arn(*rm) << 5)),
        AInst::Ret => aw(0xD65F03C0),
        AInst::Bcc(l, c) => {
            let t = *lm.get(l).ok_or("unknown")?;
            let r = t.wrapping_sub(off) as i32;
            aw(0x54000000 | ((((r as u32) >> 2) & 0x7FFFF) << 5) | (*c as u32))
        }
        AInst::CmpRR(n, m) => aw(0xEB000000 | (arn(*m) << 16) | (arn(*n) << 5)),
        AInst::CmpImm(n, i) => aw(0xF1000000 | ((*i & 0xFFF) << 10) | (arn(*n) << 5)),
        AInst::Nop => aw(0xD503201F),
        AInst::Hlt(v) => aw(0xD4400000 | ((*v as u32) & 0xFFFF)),
    })
}

fn alower(s: &str) -> Result<AInst, String> {
    let t = s.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(AInst::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let gr = |s: &str| arp(s).ok_or_else(|| format!("bad reg '{}'", s));
    match m {
        "mov" if v.len() == 2 => {
            if let Ok(imm) = v[1].parse::<u64>() { Ok(AInst::MovImm(gr(v[0])?, imm)) }
            else { Ok(AInst::MovReg(gr(v[0])?, gr(v[1])?)) }
        }
        "add" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u32>() { Ok(AInst::AddImm(d, n, imm)) }
            else { Ok(AInst::AddRRR(d, n, gr(v[2])?)) }
        }
        "sub" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u32>() { Ok(AInst::SubImm(d, n, imm)) }
            else { Ok(AInst::SubRRR(d, n, gr(v[2])?)) }
        }
        "cmp" if v.len() == 2 => {
            let n = gr(v[0])?;
            if let Ok(imm) = v[1].parse::<u32>() { Ok(AInst::CmpImm(n, imm)) }
            else { Ok(AInst::CmpRR(n, gr(v[1])?)) }
        }
        "b" => Ok(AInst::B(v[0].to_string())),
        "bl" => Ok(AInst::Bl(v[0].to_string())),
        "br" => Ok(AInst::Br(gr(v[0])?)),
        "ret" => Ok(AInst::Ret),
        "beq" => Ok(AInst::Bcc(v[0].to_string(), 0)),
        "bne" => Ok(AInst::Bcc(v[0].to_string(), 1)),
        "blt" => Ok(AInst::Bcc(v[0].to_string(), 11)),
        "bgt" => Ok(AInst::Bcc(v[0].to_string(), 12)),
        "ble" => Ok(AInst::Bcc(v[0].to_string(), 13)),
        "bge" => Ok(AInst::Bcc(v[0].to_string(), 10)),
        "nop" => Ok(AInst::Nop),
        "hlt" => Ok(AInst::Hlt(v[0].parse().map_err(|_| "bad hlt")?)),
        "ldr" if v.len() == 2 => {
            let rd = gr(v[0])?; let mem = v[1];
            if mem.contains('[') {
                let inner = mem.trim_matches(|c| c == '[' || c == ']');
                let p2: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if p2.len() == 2 {
                    let off: i16 = p2[1].trim_start_matches('#').parse().map_err(|_| "bad off")?;
                    Ok(AInst::LdrOff(rd, gr(p2[0])?, off))
                } else { Err("bad mem".into()) }
            } else { Ok(AInst::LdrPc(rd, mem.to_string())) }
        }
        "str" if v.len() == 2 => {
            let rd = gr(v[0])?; let mem = v[1];
            if mem.contains('[') {
                let inner = mem.trim_matches(|c| c == '[' || c == ']');
                let p2: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                if p2.len() == 2 {
                    let off: i16 = p2[1].trim_start_matches('#').parse().map_err(|_| "bad off")?;
                    Ok(AInst::StrOff(rd, gr(p2[0])?, off))
                } else { Err("bad mem".into()) }
            } else { Ok(AInst::StrPc(rd, mem.to_string())) }
        }
        _ => Err(format!("unknown aarch64 '{}'", m)),
    }
}

fn ab_assemble(p: &Program) -> Result<Vec<u8>, String> {
    let mut is: Vec<AInst> = Vec::new();
    is.push(AInst::Label("_start".to_string()));
    is.push(AInst::MovImm(AReg::X0, 42));
    is.push(AInst::Bl(format!("__event_{}", p.entry_event)));
    is.push(AInst::Ret);
    struct CfFrame { kind: CfKind, endif_label: String, else_label: String, beqz_indices: Vec<usize>, has_else: bool }
    #[derive(PartialEq)] enum CfKind { If, While }
    let mut cf_stack: Vec<CfFrame> = Vec::new();
    let mut cf_counter: u32 = 0;
    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        is.push(AInst::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') { continue; }
            if t.ends_with(':') { is.push(AInst::Label(format!("{}.{}", b.name, t[..t.len()-1].trim()))); continue; }
            if let Some(x) = t.strip_prefix("emit ") { is.push(AInst::Bl(format!("__event_{}", x.trim()))); continue; }
            if let Some(x) = t.strip_prefix("call ") { is.push(AInst::Bl(format!("__proc_{}", x.trim()))); continue; }
            if t == "ret" { is.push(AInst::Ret); continue; }
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = match cond_str.trim().parse::<u64>() {
                    Ok(n) => { is.push(AInst::MovImm(AReg::X10, n)); AReg::X10 }
                    Err(_) => { let reg_str = cond_str.trim(); arp(reg_str).ok_or_else(|| format!("unknown register '{}' for if", reg_str))? }
                };
                let endif_lbl = format!("__cf_{}_endif", cf_counter);
                let else_lbl = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                is.push(AInst::CmpRR(reg, AReg::Xzr));
                let beqz_idx = is.len();
                is.push(AInst::Bcc(endif_lbl.clone(), 0));
                cf_stack.push(CfFrame { kind: CfKind::If, endif_label: endif_lbl, else_label: else_lbl, beqz_indices: vec![beqz_idx], has_else: false });
                continue;
            }
            if let Some(cond_str) = t.strip_prefix("elif ") {
                let frame = cf_stack.last_mut().ok_or("elif without if")?;
                if frame.has_else { return Err("elif after else".to_string()); }
                let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.beqz_indices.len());
                cf_counter += 1;
                let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                if let AInst::Bcc(ref mut label, _) = is[*prev] { *label = elif_lbl.clone(); }
                is.push(AInst::B(frame.endif_label.clone()));
                is.push(AInst::Label(elif_lbl));
                let reg = match cond_str.trim().parse::<u64>() {
                    Ok(n) => { is.push(AInst::MovImm(AReg::X10, n)); AReg::X10 }
                    Err(_) => { let reg_str = cond_str.trim(); arp(reg_str).ok_or_else(|| format!("unknown register '{}' for elif", reg_str))? }
                };
                is.push(AInst::CmpRR(reg, AReg::Xzr));
                let beqz_idx = is.len();
                is.push(AInst::Bcc(frame.endif_label.clone(), 0));
                frame.beqz_indices.push(beqz_idx);
                continue;
            }
            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".to_string()); }
                frame.has_else = true;
                let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                if let AInst::Bcc(ref mut label, _) = is[*prev] { *label = frame.else_label.clone(); }
                is.push(AInst::B(frame.endif_label.clone()));
                is.push(AInst::Label(frame.else_label.clone()));
                continue;
            }
            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                if frame.kind == CfKind::While { return Err("endif without matching if".to_string()); }
                is.push(AInst::Label(frame.endif_label.clone()));
                continue;
            }
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = match cond_str.trim().parse::<u64>() {
                    Ok(n) => { is.push(AInst::MovImm(AReg::X10, n)); AReg::X10 }
                    Err(_) => { let reg_str = cond_str.trim(); arp(reg_str).ok_or_else(|| format!("unknown register '{}' for while", reg_str))? }
                };
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                is.push(AInst::Label(start_lbl));
                is.push(AInst::CmpRR(reg, AReg::Xzr));
                let beqz_idx = is.len();
                is.push(AInst::Bcc(endwhile_lbl.clone(), 0));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), beqz_indices: vec![beqz_idx], has_else: false });
                continue;
            }
            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                if frame.kind != CfKind::While { return Err("endwhile without matching while".to_string()); }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                is.push(AInst::B(start_lbl));
                is.push(AInst::Label(frame.endif_label.clone()));
                continue;
            }
            match alower(t) { Ok(i) => is.push(i), Err(e) => return Err(format!("line '{}': {}", t, e)) }
        }
    }
    if !cf_stack.is_empty() {
        return Err("unclosed if/while block".to_string());
    }
    // Peephole optimise instruction stream
    crate::direct_peephole::peephole(&mut is,
        |i| matches!(i, AInst::Nop),
        |i| matches!(i, AInst::B(_)),
        |i| matches!(i, AInst::Ret),
        |i| matches!(i, AInst::Label(_)),
    );
    is.push(AInst::Label("__data".to_string()));
    for d in &p.data { match d {
        DataDecl::String { name, value } => {
            let mut b = crate::direct_arch::expand_str(value); b.push(0);
            is.push(AInst::Label(format!("__data_{}", name))); is.push(AInst::Bytes(b));
        }
        DataDecl::Scalar { name, width, value } => {
            is.push(AInst::Label(format!("__data_{}", name)));
            is.push(AInst::Bytes(match width {
                ScalarWidth::Byte => vec![*value as u8], ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(), ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
            }));
        }
        DataDecl::Buffer { name, size } => { is.push(AInst::Label(format!("__data_{}", name))); is.push(AInst::Bytes(vec![0u8; *size])); }
    }}
    let mut lm = BTreeMap::new(); let mut o: u32 = 0;
    for i in &is { match i { AInst::Label(n) => { lm.insert(n.clone(), o); } AInst::Bytes(b) => o += b.len() as u32, _ => o += 4 } }
    let mut bin = Vec::new();
    for i in &is { match i { AInst::Label(_) => {} AInst::Bytes(b) => bin.extend_from_slice(b), _ => bin.extend_from_slice(&aenc(i, bin.len() as u32, &lm)?) } }
    Ok(bin)
}
