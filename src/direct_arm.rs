use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

pub struct ArmCmBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectArmCmBuilder;

impl DirectArmCmBuilder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<ArmCmBuildOutput, String> {
        if p.target != "armcm" { return Err(format!("need armcm, got '{}'", p.target)); }
        let k = assemble(p)?;
        let mut bin = vec![0u8; 512];
        bin[0..4].copy_from_slice(&0x20020000u32.to_le_bytes());
        bin[4..8].copy_from_slice(&0x200u32.to_le_bytes());
        bin.extend_from_slice(&k);
        std::fs::write(out, &bin).map_err(|e| e.to_string())?;
        Ok(ArmCmBuildOutput { bin_path: out.to_path_buf(), bin_size: bin.len() })
    }
}

#[derive(Clone, Copy)]
enum Reg { R0,R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,Sp,Lr,Pc }

fn rp(s: &str) -> Option<Reg> {
    Some(match s.to_lowercase().as_str() {
        "r0"=>Reg::R0,"r1"=>Reg::R1,"r2"=>Reg::R2,"r3"=>Reg::R3,"r4"=>Reg::R4,
        "r5"=>Reg::R5,"r6"=>Reg::R6,"r7"=>Reg::R7,"r8"=>Reg::R8,"r9"=>Reg::R9,
        "r10"=>Reg::R10,"r11"=>Reg::R11,"r12"=>Reg::R12,"sp"=>Reg::Sp,"lr"=>Reg::Lr,"pc"=>Reg::Pc,
        _=>return None
    })
}

fn rn(r: Reg) -> u8 { r as u8 }
fn w16(u: u16) -> Vec<u8> { u.to_le_bytes().to_vec() }
fn w32(u: u32) -> Vec<u8> { let b = u.to_le_bytes(); vec![b[2], b[3], b[0], b[1]] }

#[derive(Clone)]
enum Inst {
    Label(String), Bytes(Vec<u8>), MovImm(Reg, u8), MovReg(Reg, Reg),
    AddRRR(Reg, Reg, Reg), AddImm(Reg, u8), SubRRR(Reg, Reg, Reg), SubImm(Reg, u8),
    LdrPc(Reg, u8), LdrSp(Reg, u8), StrSp(Reg, u8),
    Push(Vec<Reg>), Pop(Vec<Reg>), B(String), Bl(String), Bx(Reg),
    Beq(String), Bne(String), Blt(String), Bgt(String), Ble(String), Bge(String),
    CmpRR(Reg, Reg), CmpImm(Reg, u8), Nop, Wfi, Svc(u8),
}

fn bcc(l: &str, c: u8, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let t = *lm.get(l).ok_or("unknown")?;
    let r = t.wrapping_sub(off + 4) as i32;
    if r < -256 || r > 254 { return Err("bcc out of range".into()); }
    Ok(w16(0xD000 | ((c as u16) << 8) | (r as u16 & 0xFF)))
}

fn enc(i: &Inst, off: u32, lm: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    Ok(match i {
        Inst::Label(_) => vec![],
        Inst::Bytes(b) => b.clone(),
        Inst::MovImm(r, v) => {
            let d = rn(*r);
            if d <= 7 { w16(0x2000 | ((*v as u16) << 8) | ((d as u16) << 4)) }
            else { return Err("mov imm hi reg n/a".into()); }
        }
        Inst::MovReg(d, s) => { let dd = rn(*d); let ss = rn(*s); w16(0x4600 | ((ss as u16) << 3) | (dd as u16)) }
        Inst::AddRRR(d, n, m) => { let dd=rn(*d); let nn=rn(*n); let mm=rn(*m); w16(0x1800 | ((mm as u16) << 6) | ((nn as u16) << 3) | (dd as u16)) }
        Inst::AddImm(r, v) => { let d=rn(*r); w16(0x3000 | ((d as u16) << 8) | (*v as u16)) }
        Inst::SubRRR(d, n, m) => { let dd=rn(*d); let nn=rn(*n); let mm=rn(*m); w16(0x1A00 | ((mm as u16) << 6) | ((nn as u16) << 3) | (dd as u16)) }
        Inst::SubImm(r, v) => { let d=rn(*r); w16(0x3800 | ((d as u16) << 8) | (*v as u16)) }
        Inst::LdrPc(r, v) => { let d=rn(*r); w16(0x4800 | ((d as u16) << 8) | (*v as u16)) }
        Inst::LdrSp(r, v) => { let d=rn(*r); w16(0x9800 | ((d as u16) << 8) | (*v as u16)) }
        Inst::StrSp(r, v) => { let d=rn(*r); w16(0x9000 | ((d as u16) << 8) | (*v as u16)) }
        Inst::Push(rl) => { let mut m=0u16; for r in rl { let n=rn(*r); m|=1<<n; } w16(0xB400 | (if m&(1<<14)!=0{0x100}else{0}) | (m&0xFF)) }
        Inst::Pop(rl) => { let mut m=0u16; for r in rl { let n=rn(*r); m|=1<<n; } w16(0xBC00 | (if m&(1<<15)!=0{0x100}else{0}) | (m&0xFF)) }
        Inst::B(l) => { let t=*lm.get(l).ok_or("unknown")?; let r=(t.wrapping_sub(off+4)as i32); w16(0xE000 | (((r as u16)>>1)&0x7FF)) }
        Inst::Bl(l) => {
            let t = *lm.get(l).ok_or("unknown")?;
            let r = t.wrapping_sub(off + 4) as i32;
            let o = ((r as u32) >> 1) & 0x7FFFFF;
            let s = (o >> 22) & 1;
            let j1 = (((o >> 21) & 1) ^ s ^ 1) as u16;
            let j2 = (((o >> 20) & 1) ^ s ^ 1) as u16;
            let i10 = ((o >> 11) & 0x3FF) as u16;
            let i11 = (o & 0x7FF) as u16;
            let high = ((s as u16) << 10) | i10;
            let low = 0xF800 | (j1 << 13) | (j2 << 11) | i11;
            let mut bytes = high.to_le_bytes().to_vec();
            bytes.extend_from_slice(&low.to_le_bytes());
            bytes
        }
        Inst::Bx(r) => { let m=rn(*r); w16(0x4700 | ((m as u16) << 3)) }
        Inst::Beq(l) => return bcc(l, 0x0, off, lm),
        Inst::Bne(l) => return bcc(l, 0x1, off, lm),
        Inst::Blt(l) => return bcc(l, 0xB, off, lm),
        Inst::Bgt(l) => return bcc(l, 0xC, off, lm),
        Inst::Ble(l) => return bcc(l, 0xD, off, lm),
        Inst::Bge(l) => return bcc(l, 0xA, off, lm),
        Inst::CmpRR(n, m) => { let nn=rn(*n); let mm=rn(*m); w16(0x4280 | ((mm as u16) << 6) | ((nn as u16) << 3) | if nn>7||mm>7{0x40}else{0}) }
        Inst::CmpImm(n, v) => { let nn=rn(*n); w16(0x2800 | ((nn as u16) << 8) | (*v as u16)) }
        Inst::Nop => w16(0xBF00),
        Inst::Wfi => w16(0xBF30),
        Inst::Svc(v) => w16(0xDF00 | (*v as u16)),
    })
}

fn lower(s: &str) -> Result<Inst, String> {
    let t = s.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(Inst::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let gr = |s: &str| rp(s).ok_or_else(|| format!("bad reg '{}'", s));
    match m {
        "mov" if v.len() == 2 => {
            if let Ok(imm) = v[1].parse::<u8>() { Ok(Inst::MovImm(gr(v[0])?, imm)) }
            else { Ok(Inst::MovReg(gr(v[0])?, gr(v[1])?)) }
        }
        "add" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u8>() { Ok(Inst::AddImm(d, imm)) }
            else { Ok(Inst::AddRRR(d, n, gr(v[2])?)) }
        }
        "sub" if v.len() == 3 => {
            let d = gr(v[0])?; let n = gr(v[1])?;
            if let Ok(imm) = v[2].parse::<u8>() { Ok(Inst::SubImm(d, imm)) }
            else { Ok(Inst::SubRRR(d, n, gr(v[2])?)) }
        }
        "cmp" if v.len() == 2 => {
            let n = gr(v[0])?;
            if let Ok(imm) = v[1].parse::<u8>() { Ok(Inst::CmpImm(n, imm)) }
            else { Ok(Inst::CmpRR(n, gr(v[1])?)) }
        }
        "push" => { let inner = r.trim().trim_matches(|c| c=='{'||c=='}'||c==' ');
            let regs: Result<Vec<Reg>,_> = inner.split(',').map(|s| gr(s.trim())).collect(); Ok(Inst::Push(regs?)) }
        "pop" => { let inner = r.trim().trim_matches(|c| c=='{'||c=='}'||c==' ');
            let regs: Result<Vec<Reg>,_> = inner.split(',').map(|s| gr(s.trim())).collect(); Ok(Inst::Pop(regs?)) }
        "b" => Ok(Inst::B(v[0].to_string())),
        "bl" => Ok(Inst::Bl(v[0].to_string())),
        "bx" => Ok(Inst::Bx(gr(v[0])?)),
        "beq" => Ok(Inst::Beq(v[0].to_string())),
        "bne" => Ok(Inst::Bne(v[0].to_string())),
        "blt" => Ok(Inst::Blt(v[0].to_string())),
        "bgt" => Ok(Inst::Bgt(v[0].to_string())),
        "ble" => Ok(Inst::Ble(v[0].to_string())),
        "bge" => Ok(Inst::Bge(v[0].to_string())),
        "nop" => Ok(Inst::Nop),
        "wfi" => Ok(Inst::Wfi),
        "svc" => Ok(Inst::Svc(v[0].parse().map_err(|_| "bad svc")?)),
        _ => Err(format!("unknown arm '{}'", m)),
    }
}

fn assemble(p: &Program) -> Result<Vec<u8>, String> {
    let mut items: Vec<Inst> = Vec::new();
    items.push(Inst::Label("_start".to_string()));
    items.push(Inst::Bl(format!("__event_{}", p.entry_event)));
    items.push(Inst::B("_halt".to_string()));
    items.push(Inst::Label("_halt".to_string()));
    items.push(Inst::B("_halt".to_string()));
    struct CfFrame { kind: CfKind, endif_label: String, else_label: String, beqz_indices: Vec<usize>, has_else: bool }
    #[derive(PartialEq)] enum CfKind { If, While }
    let mut cf_stack: Vec<CfFrame> = Vec::new();
    let mut cf_counter: u32 = 0;
    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        items.push(Inst::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') { continue; }
            if t.ends_with(':') { items.push(Inst::Label(format!("{}.{}", b.name, t[..t.len()-1].trim()))); continue; }
            if let Some(x) = t.strip_prefix("emit ") { items.push(Inst::Bl(format!("__event_{}", x.trim()))); continue; }
            if let Some(x) = t.strip_prefix("call ") { items.push(Inst::Bl(format!("__proc_{}", x.trim()))); continue; }
            if t == "ret" { items.push(Inst::Pop(vec![Reg::Pc])); continue; }
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = match cond_str.trim().parse::<u8>() {
                    Ok(n) => { items.push(Inst::MovImm(Reg::R0, n)); Reg::R0 }
                    Err(_) => {
                        let reg_str = cond_str.trim();
                        let r = rp(reg_str).ok_or_else(|| format!("unknown register '{}' for if", reg_str))?;
                        if rn(r) > 7 { items.push(Inst::MovReg(Reg::R0, r)); Reg::R0 } else { r }
                    }
                };
                let endif_lbl = format!("__cf_{}_endif", cf_counter);
                let else_lbl = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                items.push(Inst::CmpImm(reg, 0));
                let beqz_idx = items.len();
                items.push(Inst::Beq(endif_lbl.clone()));
                cf_stack.push(CfFrame { kind: CfKind::If, endif_label: endif_lbl, else_label: else_lbl, beqz_indices: vec![beqz_idx], has_else: false });
                continue;
            }
            if let Some(cond_str) = t.strip_prefix("elif ") {
                let frame = cf_stack.last_mut().ok_or("elif without if")?;
                if frame.has_else { return Err("elif after else".to_string()); }
                let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.beqz_indices.len());
                cf_counter += 1;
                let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                if let Inst::Beq(ref mut label) = items[*prev] { *label = elif_lbl.clone(); }
                items.push(Inst::B(frame.endif_label.clone()));
                items.push(Inst::Label(elif_lbl));
                let reg = match cond_str.trim().parse::<u8>() {
                    Ok(n) => { items.push(Inst::MovImm(Reg::R0, n)); Reg::R0 }
                    Err(_) => {
                        let reg_str = cond_str.trim();
                        let r = rp(reg_str).ok_or_else(|| format!("unknown register '{}' for elif", reg_str))?;
                        if rn(r) > 7 { items.push(Inst::MovReg(Reg::R0, r)); Reg::R0 } else { r }
                    }
                };
                items.push(Inst::CmpImm(reg, 0));
                let beqz_idx = items.len();
                items.push(Inst::Beq(frame.endif_label.clone()));
                frame.beqz_indices.push(beqz_idx);
                continue;
            }
            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".to_string()); }
                frame.has_else = true;
                let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                if let Inst::Beq(ref mut label) = items[*prev] { *label = frame.else_label.clone(); }
                items.push(Inst::B(frame.endif_label.clone()));
                items.push(Inst::Label(frame.else_label.clone()));
                continue;
            }
            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                if frame.kind == CfKind::While { return Err("endif without matching if".to_string()); }
                items.push(Inst::Label(frame.endif_label.clone()));
                continue;
            }
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = match cond_str.trim().parse::<u8>() {
                    Ok(n) => { items.push(Inst::MovImm(Reg::R0, n)); Reg::R0 }
                    Err(_) => {
                        let reg_str = cond_str.trim();
                        let r = rp(reg_str).ok_or_else(|| format!("unknown register '{}' for while", reg_str))?;
                        if rn(r) > 7 { items.push(Inst::MovReg(Reg::R0, r)); Reg::R0 } else { r }
                    }
                };
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                items.push(Inst::Label(start_lbl));
                items.push(Inst::CmpImm(reg, 0));
                let beqz_idx = items.len();
                items.push(Inst::Beq(endwhile_lbl.clone()));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), beqz_indices: vec![beqz_idx], has_else: false });
                continue;
            }
            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                if frame.kind != CfKind::While { return Err("endwhile without matching while".to_string()); }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                items.push(Inst::B(start_lbl));
                items.push(Inst::Label(frame.endif_label.clone()));
                continue;
            }
            match lower(t) { Ok(i) => items.push(i), Err(_) => {} }
        }
    }
    if !cf_stack.is_empty() {
        return Err("unclosed if/while block".to_string());
    }
    // Peephole
    crate::direct_peephole::peephole(&mut items,
        |i| matches!(i, Inst::Nop),
        |i| matches!(i, Inst::B(_)|Inst::Bl(_)),
        |i| matches!(i, Inst::Bx(Reg::Lr)|Inst::Pop(_)),
        |i| matches!(i, Inst::Label(_)),
    );
    items.push(Inst::Label("__data".to_string()));
    for d in &p.data { match d {
        DataDecl::String{name,value} => { let mut b = crate::direct_arch::expand_str(value); b.push(0);
            items.push(Inst::Label(format!("__data_{}", name))); items.push(Inst::Bytes(b)); }
        DataDecl::Scalar{name,width,value} => { items.push(Inst::Label(format!("__data_{}", name)));
            items.push(Inst::Bytes(match width {
                ScalarWidth::Byte => vec![*value as u8], ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(), ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
            })); }
        DataDecl::Buffer{name,size} => { items.push(Inst::Label(format!("__data_{}", name))); items.push(Inst::Bytes(vec![0u8; *size])); }
    }}
    let mut lm = BTreeMap::new(); let mut o: u32 = 0;
    for i in &items { match i {
        Inst::Label(n) => { lm.insert(n.clone(), o); }
        Inst::Bytes(b) => o += b.len() as u32,
        _ => o += if matches!(i, Inst::Bl(_)) { 4 } else { 2 },
    }}
    let mut bin = Vec::new();
    for i in &items { match i {
        Inst::Label(_) => {},
        Inst::Bytes(b) => bin.extend_from_slice(b),
        _ => bin.extend_from_slice(&enc(i, bin.len() as u32, &lm)?),
    }}
    Ok(bin)
}
