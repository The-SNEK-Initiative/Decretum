use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct CheriBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectCheriBuilder;

impl DirectCheriBuilder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<CheriBuildOutput, String> {
        if p.target != "cheri" { return Err(format!("need cheri, got '{}'", p.target)); }
        let k = cr_assemble(p)?;
        std::fs::write(out, &k).map_err(|e| e.to_string())?;
        Ok(CheriBuildOutput { bin_path: out.to_path_buf(), bin_size: k.len() })
    }
}

#[derive(Clone, Copy)]
enum CReg { C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13,
    C14, C15, C16, C17, C18, C19, C20, C21, C22, C23, C24, C25, C26, C27, C28, C29, Ddc, Pcc }

fn crp(s: &str) -> Option<CReg> {
    Some(match s.to_lowercase().as_str() {
        "c0"=>CReg::C0,"c1"=>CReg::C1,"c2"=>CReg::C2,"c3"=>CReg::C3,
        "c4"=>CReg::C4,"c5"=>CReg::C5,"c6"=>CReg::C6,"c7"=>CReg::C7,
        "c8"=>CReg::C8,"c9"=>CReg::C9,"c10"=>CReg::C10,"c11"=>CReg::C11,
        "c12"=>CReg::C12,"c13"=>CReg::C13,"c14"=>CReg::C14,"c15"=>CReg::C15,
        "c16"=>CReg::C16,"c17"=>CReg::C17,"c18"=>CReg::C18,"c19"=>CReg::C19,
        "c20"=>CReg::C20,"c21"=>CReg::C21,"c22"=>CReg::C22,"c23"=>CReg::C23,
        "c24"=>CReg::C24,"c25"=>CReg::C25,"c26"=>CReg::C26,"c27"=>CReg::C27,
        "c28"=>CReg::C28,"c29"=>CReg::C29,"ddc"=>CReg::Ddc,"pcc"=>CReg::Pcc, _=>return None
    })
}

fn crn(r: CReg) -> u32 { r as u32 }
fn cw(u: u32) -> Vec<u8> { u.to_le_bytes().to_vec() }

#[derive(Clone)]
enum CInst {
    Label(String), Bytes(Vec<u8>), MovImm(CReg, u64), MovReg(CReg, CReg),
    AddRRR(CReg, CReg, CReg), SubRRR(CReg, CReg, CReg),
    AddImm(CReg, CReg, u32), SubImm(CReg, CReg, u32),
    B(String), Bl(String), Cbz(String, CReg), Ret, Nop,
    CSetAddr(CReg, CReg, u64), CSetBounds(CReg, CReg, u64), CAndPerm(CReg, CReg, u32),
    CLd(CReg, CReg, i16), CSt(CReg, CReg, i16), CCall(CReg, CReg), CRet(CReg),
    CFromPtr(CReg, CReg, u32, u32),
}

fn cenc(i: &CInst, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    fn jmp(op: u32, off: u32, lm: &BTreeMap<String, u32>, l: &str) -> Result<Vec<u8>, String> {
        let t = *lm.get(l).ok_or("unknown")?;
        let r = t.wrapping_sub(off) as i32;
        Ok(cw(op | (((r as u32) >> 2) & 0x3FFFFFF)))
    }
    Ok(match i {
        CInst::Label(_) => vec![],
        CInst::Bytes(b) => b.clone(),
        CInst::MovImm(r, v) => {
            let d = crn(*r);
            if *v <= 0xFFFF { cw(0xD2800000 | d | ((*v as u32) << 5)) }
            else {
                let mut b = cw(0xD2800000 | d | ((*v as u32) << 5 & 0x1F));
                let w1 = ((*v >> 16) & 0xFFFF) as u32;
                if w1 != 0 { b.extend(cw(0xF2A00000 | d | (w1 << 5))); }
                let w2 = ((*v >> 32) & 0xFFFF) as u32;
                if w2 != 0 { b.extend(cw(0xF2C00000 | d | (w2 << 5))); }
                let w3 = ((*v >> 48) & 0xFFFF) as u32;
                if w3 != 0 { b.extend(cw(0xF2E00000 | d | (w3 << 5))); }
                return Ok(b);
            }
        }
        CInst::MovReg(d, s) => cw(0xAA000000 | (crn(*s) << 16) | crn(*d)),
        CInst::AddRRR(d, n, m) => cw(0x8B000000 | (crn(*m) << 16) | (crn(*n) << 5) | crn(*d)),
        CInst::SubRRR(d, n, m) => cw(0xCB000000 | (crn(*m) << 16) | (crn(*n) << 5) | crn(*d)),
        CInst::AddImm(d, n, i) => cw(0x91000000 | ((*i & 0xFFF) << 10) | (crn(*n) << 5) | crn(*d)),
        CInst::SubImm(d, n, i) => cw(0xD1000000 | ((*i & 0xFFF) << 10) | (crn(*n) << 5) | crn(*d)),
        CInst::B(l) => return jmp(0x14000000, off, lm, l),
        CInst::Bl(l) => return jmp(0x94000000, off, lm, l),
        CInst::Cbz(l, r) => {
            let tgt = *lm.get(l).ok_or("unknown")?;
            let rel = tgt.wrapping_sub(off) as i32;
            let imm19 = ((rel as u32) >> 2) & 0x7FFFF;
            cw(0xB4000000 | crn(*r) | (imm19 << 5))
        }
        CInst::Ret => cw(0xD65F03C0),
        CInst::Nop => cw(0xD503201F),
        CInst::CSetAddr(cd, _cb, addr) => {
            let mut b = cw(0xD503201F);
            b.extend_from_slice(&addr.to_le_bytes()[..8]);
            b.extend(cw(0xD503201F));
            return Ok(b);
        }
        CInst::CSetBounds(cd, _cb, sz) => {
            let mut b = cw(0xD503205F);
            b.extend_from_slice(&sz.to_le_bytes()[..8]);
            b.extend(cw(0xD503205F));
            return Ok(b);
        }
        CInst::CAndPerm(cd, _cb, perms) => {
            let mut b = cw(0xD503209F);
            b.extend_from_slice(&perms.to_le_bytes()[..4]);
            b.extend(cw(0xD503209F));
            return Ok(b);
        }
        CInst::CLd(cd, cb, _imm) => {
            cw(0xFC000000 | (crn(*cb) << 5) | crn(*cd))
        }
        CInst::CSt(cs, cb, _imm) => {
            cw(0xFC000000 | (crn(*cs) << 16) | (crn(*cb) << 5))
        }
        CInst::CCall(_cd, cp) => cw(0xD61F0000 | (1 << 10) | (crn(*cp) << 5)),
        CInst::CRet(cd) => cw(0xD61F0000 | (1 << 10) | (crn(*cd) << 5) | crn(*cd)),
        CInst::CFromPtr(cd, base, off, bounds) => {
            let mut bin = cenc(&CInst::AddImm(*cd, *base, *off), 0, lm)?;
            bin.extend(cenc(&CInst::CSetBounds(*cd, *cd, *bounds as u64), 0, lm)?);
            return Ok(bin);
        }
    })
}

fn clower(s: &str) -> Result<CInst, String> {
    let t = s.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(CInst::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let gr = |s: &str| crp(s).ok_or_else(|| format!("bad reg '{}'", s));
    match m {
        "mov" if v.len() == 2 => {
            if let Ok(imm) = v[1].parse::<u64>() { Ok(CInst::MovImm(gr(v[0])?, imm)) }
            else { Ok(CInst::MovReg(gr(v[0])?, gr(v[1])?)) }
        }
        "add" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u32>() { Ok(CInst::AddImm(d, n, imm)) }
            else { Ok(CInst::AddRRR(d, n, gr(v[2])?)) }
        }
        "sub" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u32>() { Ok(CInst::SubImm(d, n, imm)) }
            else { Ok(CInst::SubRRR(d, n, gr(v[2])?)) }
        }
        "b" => Ok(CInst::B(v[0].to_string())),
        "bl" => Ok(CInst::Bl(v[0].to_string())),
        "ret" => Ok(CInst::Ret),
        "nop" => Ok(CInst::Nop),
        "csetaddr" => Ok(CInst::CSetAddr(gr(v[0])?, gr(v[1])?, v[2].parse().map_err(|_| "bad addr")?)),
        "csetbounds" => Ok(CInst::CSetBounds(gr(v[0])?, gr(v[1])?, v[2].parse().map_err(|_| "bad sz")?)),
        "candperm" => Ok(CInst::CAndPerm(gr(v[0])?, gr(v[1])?, v[2].parse().map_err(|_| "bad perms")?)),
        _ => Err(format!("unknown cheri '{}'", m)),
    }
}

fn cr_assemble(p: &Program) -> Result<Vec<u8>, String> {
    let mut is: Vec<CInst> = Vec::new();

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

    is.push(CInst::Label("_start".to_string()));
    is.push(CInst::MovImm(CReg::C0, 0xFFFFFFFFFFFFFFFF));
    is.push(CInst::Bl(format!("__event_{}", p.entry_event)));
    is.push(CInst::Ret);
    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        is.push(CInst::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') { continue; }
            if t.ends_with(':') { is.push(CInst::Label(format!("{}.{}", b.name, t[..t.len()-1].trim()))); continue; }
            if let Some(x) = t.strip_prefix("emit ") { is.push(CInst::Bl(format!("__event_{}", x.trim()))); continue; }
            if let Some(x) = t.strip_prefix("call ") { is.push(CInst::Bl(format!("__proc_{}", x.trim()))); continue; }
            if t == "ret" { is.push(CInst::Ret); continue; }

            // if <reg>
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = crp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endif_label = format!("__cf_{}_endif", cf_counter);
                let else_label = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                let br_idx = is.len();
                is.push(CInst::Cbz(endif_label.clone(), reg));
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
                if let CInst::Cbz(ref mut l, _) = is[prev] { *l = elif_lbl.clone(); }
                is.push(CInst::B(frame.endif_label.clone()));
                is.push(CInst::Label(elif_lbl));
                let reg = crp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let br_idx = is.len();
                is.push(CInst::Cbz(frame.endif_label.clone(), reg));
                frame.br_indices.push(br_idx);
                continue;
            }
            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".into()); }
                frame.has_else = true;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let CInst::Cbz(ref mut l, _) = is[prev] { *l = frame.else_label.clone(); }
                is.push(CInst::B(frame.endif_label.clone()));
                is.push(CInst::Label(frame.else_label.clone()));
                continue;
            }
            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
                is.push(CInst::Label(frame.endif_label.clone()));
                continue;
            }
            // while <reg>
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = crp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                is.push(CInst::Label(start_lbl));
                let br_idx = is.len();
                is.push(CInst::Cbz(endwhile_lbl.clone(), reg));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
                continue;
            }
            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                is.push(CInst::B(start_lbl));
                is.push(CInst::Label(frame.endif_label.clone()));
                continue;
            }

            match clower(t) { Ok(i) => is.push(i), Err(e) => return Err(format!("line '{}': {}", t, e)) }
        }
    }

    if !cf_stack.is_empty() { return Err("unclosed if/while block".into()); }
    // Peephole
    crate::direct_peephole::peephole(&mut is,
        |i| matches!(i, CInst::Nop),
        |i| matches!(i, CInst::B(_)|CInst::Bl(_)),
        |i| matches!(i, CInst::Ret),
        |i| matches!(i, CInst::Label(_)),
    );
    is.push(CInst::Label("__data".to_string()));
    for d in &p.data { match d {
        DataDecl::String{name,value} => { let mut b = crate::direct_arch::expand_str(value); b.push(0);
            is.push(CInst::Label(format!("__data_{}", name))); is.push(CInst::Bytes(b)); }
        DataDecl::Scalar{name,width,value} => { is.push(CInst::Label(format!("__data_{}", name)));
            is.push(CInst::Bytes(match width {
                ScalarWidth::Byte => vec![*value as u8], ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(), ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
            })); }
        DataDecl::Buffer{name,size} => { is.push(CInst::Label(format!("__data_{}", name))); is.push(CInst::Bytes(vec![0u8; *size])); }
    }}
    let mut lm = BTreeMap::new(); let mut o: u32 = 0;
    for i in &is { match i { CInst::Label(n) => { lm.insert(n.clone(), o); } CInst::Bytes(b) => o += b.len() as u32, _ => o += 4 } }
    let mut bin = Vec::new();
    for i in &is { match i { CInst::Label(_) => {} CInst::Bytes(b) => bin.extend_from_slice(b), _ => bin.extend_from_slice(&cenc(i, bin.len() as u32, &lm)?) } }
    Ok(bin)
}
