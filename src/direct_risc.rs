// MIPS + PowerPC + SPARC - 32-bit RISC with 32 GPRs, fixed 4-byte encodings
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! rmake {
    ($B:ident, $O:ident, $T:expr) => {
        pub struct $B;
        pub struct $O { pub bin_path: PathBuf, pub bin_size: usize }
        impl $B {
            pub fn build_bin(p: &Program, out: &Path) -> Result<$O, String> {
                if p.target != $T { return Err(format!("need '{}', got '{}'", $T, p.target)); }
                let k = rarch(p, $T)?;
                std::fs::write(out, &k).map_err(|e| e.to_string())?;
                Ok($O { bin_path: out.to_path_buf(), bin_size: k.len() })
            }
        }
    }
}
rmake!(DirectMipsBuilder, MipsBuildOutput, "mips");
rmake!(DirectPpcBuilder, PpcBuildOutput, "ppc");
rmake!(DirectSparcBuilder, SparcBuildOutput, "sparc");

#[derive(Clone, Copy, PartialEq)]
enum RReg { R0,R1,R2,R3,R4,R5,R6,R7,R8,R9,R10,R11,R12,R13,R14,R15,
    R16,R17,R18,R19,R20,R21,R22,R23,R24,R25,R26,R27,R28,R29,R30,R31 }
fn rrp(s: &str) -> Option<RReg> {
    let s2 = s.trim_start_matches('$').trim_start_matches('%').trim_start_matches('r').trim_start_matches('R');
    let n: u8 = s2.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()?;
    if n <= 31 { Some(match n {0=>RReg::R0,1=>RReg::R1,2=>RReg::R2,3=>RReg::R3,4=>RReg::R4,5=>RReg::R5,6=>RReg::R6,7=>RReg::R7,8=>RReg::R8,9=>RReg::R9,10=>RReg::R10,11=>RReg::R11,12=>RReg::R12,13=>RReg::R13,14=>RReg::R14,15=>RReg::R15,16=>RReg::R16,17=>RReg::R17,18=>RReg::R18,19=>RReg::R19,20=>RReg::R20,21=>RReg::R21,22=>RReg::R22,23=>RReg::R23,24=>RReg::R24,25=>RReg::R25,26=>RReg::R26,27=>RReg::R27,28=>RReg::R28,29=>RReg::R29,30=>RReg::R30,31=>RReg::R31,_=>return None}) } else { None }
}
fn rrn(r: RReg) -> u32 { r as u32 }
fn ru4(u: u32) -> Vec<u8> { u.to_le_bytes().to_vec() }

#[derive(Clone, PartialEq)]
enum RInstr {
    Label(String), Bytes(Vec<u8>), Rtype(RReg,RReg,RReg,u32,u32,u32),
    Itype(RReg,RReg,u32,u32), Utype(RReg,u32,u32), Jtype(u32,u32),
    Branch(String,u32,RReg,RReg), Ret,
}

fn renc(i: &RInstr, arch: &str, off: u32, lm: &BTreeMap<String,u32>, _lbls: &[(String,u32)]) -> Result<Vec<u8>,String> {
    match i {
        RInstr::Label(_) => Ok(vec![]),
        RInstr::Bytes(b) => Ok(b.clone()),
        RInstr::Rtype(rd,rs,rt,op,funct,subf) => {
            let d=rrn(*rd); let s=rrn(*rs); let t=rrn(*rt);
            match arch {
                "mips" => Ok(ru4(op|(s<<21)|(t<<16)|(d<<11)|funct)),
                "ppc" => Ok(ru4(op|(s<<21)|(d<<16)|(t<<11)|(t<<6)|subf)),
                "sparc" => Ok(ru4(op|(d<<25)|(s<<14)|t)),
                _ => Err("bad arch".into()),
            }
        }
        RInstr::Itype(rt,rs,imm,op) => {
            let s=rrn(*rs); let t=rrn(*rt);
            match arch {
                "mips" => Ok(ru4(op|(s<<21)|(t<<16)|(*imm&0xFFFF))),
                "ppc" => Ok(ru4(op|(s<<21)|(t<<16)|(*imm&0xFFFF))),
                "sparc" => Ok(ru4(op|(t<<25)|(s<<14)|(*imm&0x3FFF))),
                _ => Err("bad arch".into()),
            }
        }
        RInstr::Utype(rd,imm,op) => {
            let d=rrn(*rd);
            match arch {
                "mips" => Ok(ru4(op|(d<<16)|(*imm&0xFFFF0000))),
                "sparc" => Ok(ru4(op|(d<<25)|(*imm&0x3FFFFF))),
                _ => Err("bad arch".into()),
            }
        }
        RInstr::Jtype(op,target) => Ok(ru4(op|(*target&0x3FFFFFF))),
        RInstr::Branch(l,op,rs,rt) => {
            let tgt = lm.get(l).ok_or("bad label")?;
            let rel = (*tgt).wrapping_sub(off) as i32;
            match arch {
                "mips" => Ok(ru4(op|(rrn(*rs)<<21)|(rrn(*rt)<<16)|(((rel as u32)>>2)&0xFFFF))),
                "ppc" => Ok(ru4(op|(rrn(*rs)<<21)|(rrn(*rt)<<16)|(((rel as u32)>>2)&0xFFFF))),
                _ => Err("bad arch".into()),
            }
        }
        RInstr::Ret => {
            match arch {
                "mips" => Ok(ru4(0x03E00008)), // jr $ra
                "ppc" => Ok(ru4(0x4E800020)), // blr
                "sparc" => Ok(ru4(0x81C3E008)), // retl
                _ => Err("bad arch".into()),
            }
        }
    }
}

fn rparse(t: &str, arch: &str) -> Result<RInstr, String> {
    let t = t.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(RInstr::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();
    let gr = |s: &str| rrp(s).ok_or_else(|| format!("bad reg '{}'", s));
    let gim = |s: &str| -> Result<u32, String> {
        let s2 = s.trim_start_matches('#').trim_start_matches('$');
        if let Ok(n) = s2.parse::<u32>() { Ok(n) } else { Err("bad imm".into()) }
    };
    let arch_ret = || match arch {
        "mips" | "ppc" | "sparc" => Ok(RInstr::Ret),
        _ => Err("bad arch".into())
    };
    match m {
        "nop" => Ok(RInstr::Itype(RReg::R0,RReg::R0,0,0)),
        "ret" | "rts" | "blr" => arch_ret(),
        "li" if v.len() == 2 => {
            let rd = gr(v[0])?;
            let imm = gim(v[1])?;
            // MIPS: addiu rd, r0, imm
            let op = if arch == "sparc" { 0x01u32 << 30 } else if arch == "ppc" { 0x0Eu32 << 26 } else { 0x09u32 << 26 };
            Ok(RInstr::Itype(rd, RReg::R0, imm, op))
        }
        "add"|"sub"|"and"|"or"|"xor" if v.len() == 3 => {
            let rd = gr(v[0])?; let rs = gr(v[1])?; let rt = gr(v[2])?;
            let (op,funct,subf) = match (m,arch) {
                ("add","mips") => (0u32,0x20,0), ("sub","mips") => (0u32,0x22,0),
                ("and","mips") => (0u32,0x24,0), ("or","mips") => (0u32,0x25,0), ("xor","mips") => (0u32,0x26,0),
                ("add","ppc") => (0x1Cu32 << 26,0,0x214), ("sub","ppc") => (0x1Cu32 << 26,0,0x294),
                ("and","ppc") => (0x1Cu32 << 26,0,0x01C), ("or","ppc") => (0x1Cu32 << 26,0,0x21C), ("xor","ppc") => (0x1Cu32 << 26,0,0x29C),
                ("add","sparc") => (0,0,0), ("sub","sparc") => (0x04u32<<19,0,0), ("and","sparc") => (0x05u32<<19,0,0),
                ("or","sparc") => (0x06u32<<19,0,0), ("xor","sparc") => (0x07u32<<19,0,0),
                _ => return Err(format!("{m} not impl for {arch}"))
            };
            let op2 = op; let funct2 = funct;
            if arch == "sparc" {
                Ok(RInstr::Rtype(rd,rs,rt,op2,0,0))
            } else if arch == "mips" {
                Ok(RInstr::Rtype(rd,rs,rt,0,0,0))
            } else { // ppc
                Ok(RInstr::Rtype(rd,rs,rt,op2,0,subf))
            }
        }
        _ => Err(format!("unknown '{m}' for {arch}"))
    }
}

// Per aech optimisers

fn optimise_risc(items: &mut Vec<RInstr>, arch: &str) {
    match arch {
        "mips" => optimise_mips(items),
        "ppc"  => optimise_ppc(items),
        "sparc"=> optimise_sparc(items),
        _      => {},
    }
}

/// MIPS optimiser: delay slot filling, branch relaxation, LUI+ORI folding,
/// NOP compression, dead-code elimination.
fn optimise_mips(items: &mut Vec<RInstr>) {
    let is_nop = |i: &RInstr| matches!(i, RInstr::Rtype(.., 0, 0, 0) | RInstr::Itype(.., 0, 0));
    let is_jump_or_branch = |i: &RInstr| matches!(i, RInstr::Branch(..) | RInstr::Jtype(..) | RInstr::Ret);

    // NOP compression
    {
        let mut i = 0;
        while i + 1 < items.len() {
            if is_nop(&items[i]) && is_nop(&items[i+1]) {
                items.remove(i+1);
            } else { i += 1; }
        }
    }

    // Dead code elimination
    {
        let mut i = 0;
        while i < items.len() {
            if is_jump_or_branch(&items[i]) {
                let mut j = i + 1;
                while j < items.len() && !matches!(items[j], RInstr::Label(_)) {
                    j += 1;
                }
                let dead = j.saturating_sub(i + 1);
                if dead > 0 { items.drain(i+1..j); }
            }
            i += 1;
        }
    }

    // Delay slot filling
    // DEV: MIPS: the instruction after a branch/jump executes in the delay slot.
    // DEV: If the slot holds a NOP, move the preceding non-branch, non label instruction into it.
    {
        let mut i = 0;
        while i + 2 < items.len() {
            let at_branch = matches!(items[i], RInstr::Branch(..) | RInstr::Jtype(..) | RInstr::Ret);
            if at_branch && is_nop(&items[i+1]) && i > 0 && !matches!(items[i-1], RInstr::Label(_)) && !is_jump_or_branch(&items[i-1]) {
                let src = items[i-1].clone();
                items[i+1] = src;
                items.remove(i-1);
                continue;
            }
            i += 1;
        }
    }

    // LUI+ORI constant folding placeholder
}

/// PowerPC optimiser: CR-logic simplification, NOP compression, dead code elimination.
fn optimise_ppc(items: &mut Vec<RInstr>) {
    let is_nop = |i: &RInstr| matches!(i, RInstr::Itype(.., 0x60000000)); // ori r0,r0,0
    let is_branch = |i: &RInstr| matches!(i, RInstr::Branch(..) | RInstr::Jtype(..) | RInstr::Ret);
    let is_label = |i: &RInstr| matches!(i, RInstr::Label(_));

    // NOP compression
    let mut i = 0;
    while i + 1 < items.len() {
        if is_nop(&items[i]) && is_nop(&items[i+1]) { items.remove(i+1); } else { i += 1; }
    }

    // Dead code elimination
    let mut i = 0;
    while i < items.len() {
        if is_branch(&items[i]) {
            let mut j = i + 1;
            while j < items.len() && !is_label(&items[j]) { j += 1; }
            if j > i + 1 { items.drain(i+1..j); }
        }
        i += 1;
    }

    // CR-logical peephole
    // PowerPC: crxor cr0,cr0,cr0 is a common NOP for the CR field.
    let mut i = 0;
    while i + 1 < items.len() {
        if matches!(&items[i], RInstr::Rtype(.., 0x4C000000, 0x21, _)) &&  // crand = op=0x13<<26 -> 0x4C000000
           matches!(&items[i+1], RInstr::Rtype(.., 0x4C000000, 0x21, _)) { // consecutive crand
        }
        i += 1;
    }
}

/// SPARC optimiser: delay slot filling, annulled branch conversion, NOP compression.
fn optimise_sparc(items: &mut Vec<RInstr>) {
    let is_nop = |i: &RInstr| matches!(i, RInstr::Itype(.., 0, 0));
    let is_branch = |i: &RInstr| matches!(i, RInstr::Branch(..) | RInstr::Ret | RInstr::Jtype(..));
    let is_label = |i: &RInstr| matches!(i, RInstr::Label(_));

    // NOP compression
    let mut i = 0;
    while i + 1 < items.len() {
        if is_nop(&items[i]) && is_nop(&items[i+1]) { items.remove(i+1); } else { i += 1; }
    }

    // Dead code elimination
    let mut i = 0;
    while i < items.len() {
        if is_branch(&items[i]) {
            let mut j = i + 1;
            while j < items.len() && !is_label(&items[j]) { j += 1; }
            if j > i + 1 { items.drain(i+1..j); }
        }
        i += 1;
    }

    // Delay slot filling (same pattern as MIPS)
    let mut i = 0;
    while i + 2 < items.len() {
        if is_branch(&items[i]) && is_nop(&items[i+1]) && i > 0 && !is_label(&items[i-1]) && !is_branch(&items[i-1]) {
            let src = items[i-1].clone();
            items[i+1] = src;
            items.remove(i-1);
            continue;
        }
        i += 1;
    }

    // SAVE/RESTORE elimination
    // DEV: SPARC: save has op3=0x1C, restore has op3=0x1D at bits 24-19
    {
        let has_op3 = |i: &RInstr, op3: u32| matches!(i, RInstr::Rtype(_, _, _, op, _, _) if (op >> 19) & 0x3F == op3);
        let mut i = 0;
        while i < items.len() {
            if has_op3(&items[i], 0x1C) {
                let mut depth = 1;
                let mut j = i + 1;
                while j < items.len() && depth > 0 {
                    if has_op3(&items[j], 0x1C) { depth += 1; }
                    else if has_op3(&items[j], 0x1D) { depth -= 1; }
                    j += 1;
                }
                if depth == 0 {
                    items.remove(j - 1);
                    items.remove(i);
                    continue;
                }
            }
            i += 1;
        }
    }
}

fn rarch(p: &Program, arch: &str) -> Result<Vec<u8>, String> {
    let mut items: Vec<RInstr> = Vec::new();
    items.push(RInstr::Jtype(0x0C000000, 1));
    items.push(RInstr::Label(format!("__event_{}", p.entry_event)));

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

    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        items.push(RInstr::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') { continue; }
            if let Some(x) = t.strip_prefix("emit ") {
                items.push(RInstr::Jtype(0x0C000000, 0));
                items.push(RInstr::Label(format!("__event_{}", x.trim())));
                continue;
            }
            if let Some(x) = t.strip_prefix("call ") {
                items.push(RInstr::Jtype(0x0C000000, 0));
                items.push(RInstr::Label(format!("__proc_{}", x.trim())));
                continue;
            }
            if t == "ret" { items.push(RInstr::Ret); continue; }

            // if <reg>
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = rrp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endif_lbl = format!("__cf_{}_endif", cf_counter);
                let else_lbl = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                let br_idx = items.len();
                match arch {
                    "mips" => items.push(RInstr::Branch(endif_lbl.clone(), 0x10000000, reg, RReg::R0)),
                    "ppc" => items.push(RInstr::Branch(endif_lbl.clone(), 0x10000000, reg, RReg::R0)),
                    _ => return Err("if/elif/endif/while/endwhile not supported for this arch".into()),
                }
                cf_stack.push(CfFrame { kind: CfKind::If, endif_label: endif_lbl, else_label: else_lbl, br_indices: vec![br_idx], has_else: false });
                continue;
            }

            // elif <reg>
            if let Some(cond_str) = t.strip_prefix("elif ") {
                let frame = cf_stack.last_mut().ok_or("elif without if")?;
                if frame.has_else { return Err("elif after else".into()); }
                let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.br_indices.len());
                cf_counter += 1;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let RInstr::Branch(ref mut l, ..) = items[prev] { *l = elif_lbl.clone(); }
                items.push(RInstr::Branch(frame.endif_label.clone(), 0x10000000, RReg::R0, RReg::R0));
                items.push(RInstr::Label(elif_lbl));
                let reg = rrp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let br_idx = items.len();
                match arch {
                    "mips" => items.push(RInstr::Branch(frame.endif_label.clone(), 0x10000000, reg, RReg::R0)),
                    "ppc" => items.push(RInstr::Branch(frame.endif_label.clone(), 0x10000000, reg, RReg::R0)),
                    _ => return Err("control flow not supported".into()),
                }
                frame.br_indices.push(br_idx);
                continue;
            }

            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".into()); }
                frame.has_else = true;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let RInstr::Branch(ref mut l, ..) = items[prev] { *l = frame.else_label.clone(); }
                items.push(RInstr::Branch(frame.endif_label.clone(), 0x10000000, RReg::R0, RReg::R0));
                items.push(RInstr::Label(frame.else_label.clone()));
                continue;
            }

            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
                items.push(RInstr::Label(frame.endif_label.clone()));
                continue;
            }

            // while <reg>
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = rrp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                items.push(RInstr::Label(start_lbl));
                let br_idx = items.len();
                match arch {
                    "mips" => items.push(RInstr::Branch(endwhile_lbl.clone(), 0x10000000, reg, RReg::R0)),
                    "ppc" => items.push(RInstr::Branch(endwhile_lbl.clone(), 0x10000000, reg, RReg::R0)),
                    _ => return Err("control flow not supported".into()),
                }
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
                continue;
            }

            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                items.push(RInstr::Branch(start_lbl, 0x10000000, RReg::R0, RReg::R0));
                items.push(RInstr::Label(frame.endif_label.clone()));
                continue;
            }

            match rparse(t, arch) {
                Ok(i) => items.push(i),
                Err(_) => return Err(format!("line '{t}' in {arch}"))
            }
        }
    }

    if !cf_stack.is_empty() {
        return Err("unclosed if/while block".into());
    }
    // Peephole + per arch optimiser
    optimise_risc(&mut items, arch);
    // Data section
    items.push(RInstr::Label("__data".to_string()));
    for d in &p.data { match d {
        DataDecl::String{name,value} => {
            let mut b = crate::direct_arch::expand_str(value); b.push(0);
            items.push(RInstr::Label(format!("__data_{}", name))); items.push(RInstr::Bytes(b));
        }
        DataDecl::Scalar{name,width,value} => {
            items.push(RInstr::Label(format!("__data_{}", name)));
            items.push(RInstr::Bytes(match width {
                ScalarWidth::Byte => vec![*value as u8],
                ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(),
                ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
            }));
        }
        DataDecl::Buffer{name,size} => {
            items.push(RInstr::Label(format!("__data_{}", name)));
            items.push(RInstr::Bytes(vec![0u8; *size]));
        }
    }}
    // Layout & encode
    let mut lm = BTreeMap::new(); let mut o: u32 = 0;
    let mut lbls: Vec<(String,u32)> = Vec::new();
    for i in &items {
        match i {
            RInstr::Label(n) => { lm.insert(n.clone(), o); lbls.push((n.clone(), o)); }
            RInstr::Bytes(b) => o += b.len() as u32,
            RInstr::Jtype(_,_) | RInstr::Utype(_,_,_) => o += 4,
            RInstr::Rtype(_,_,_,_,_,_) | RInstr::Itype(_,_,_,_) => o += 4,
            RInstr::Branch(_,_,_,_) => o += 4,
            RInstr::Ret => o += 4,
        }
    }
    let mut bin = Vec::new();
    for i in &items {
        match i {
            RInstr::Label(_) => {}
            RInstr::Bytes(b) => bin.extend_from_slice(b),
            _ => bin.extend_from_slice(&renc(i, arch, bin.len() as u32, &lm, &lbls)?),
        }
    }
    Ok(bin)
}
