// Alpha + PA-RISC - 64-bit RISC with unique encodings
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::dcrt::*;

macro_rules! emake {
    ($B:ident, $O:ident, $T:expr) => {
        pub struct $B;
        pub struct $O { pub bin_path: PathBuf, pub bin_size: usize }
        impl $B {
            pub fn build_bin(p: &Program, out: &Path) -> Result<$O, String> {
                if p.target != $T { return Err(format!("need '{}', got '{}'", $T, p.target)); }
                let k = earch(p, $T)?;
                std::fs::write(out, &k).map_err(|e| e.to_string())?;
                Ok($O { bin_path: out.to_path_buf(), bin_size: k.len() })
            }
        }
    }
}
emake!(DirectAlphaBuilder, AlphaBuildOutput, "alpha");
emake!(DirectPariscBuilder, PariscBuildOutput, "parisc");

#[derive(Clone, Copy)]
enum EReg { V0,V1,V2,V3,V4,V5,V6,V7,V8,V9,V10,V11,V12,V13,V14,V15,
    V16,V17,V18,V19,V20,V21,V22,V23,V24,V25,V26,V27,V28,V29,V30,V31 }
fn erp(s: &str) -> Option<EReg> {
    let s2 = s.trim_start_matches('$').trim_start_matches('%').to_lowercase();
    let n = s2.trim_start_matches('r').trim_start_matches('v').parse::<u8>().ok()?;
    if n <= 31 { Some(match n {0=>EReg::V0,1=>EReg::V1,2=>EReg::V2,3=>EReg::V3,4=>EReg::V4,5=>EReg::V5,6=>EReg::V6,7=>EReg::V7,8=>EReg::V8,9=>EReg::V9,10=>EReg::V10,11=>EReg::V11,12=>EReg::V12,13=>EReg::V13,14=>EReg::V14,15=>EReg::V15,16=>EReg::V16,17=>EReg::V17,18=>EReg::V18,19=>EReg::V19,20=>EReg::V20,21=>EReg::V21,22=>EReg::V22,23=>EReg::V23,24=>EReg::V24,25=>EReg::V25,26=>EReg::V26,27=>EReg::V27,28=>EReg::V28,29=>EReg::V29,30=>EReg::V30,31=>EReg::V31,_=>return None}) } else { None }
}
fn ern(r: EReg) -> u32 { r as u32 }
fn eu4(u: u32) -> Vec<u8> { u.to_le_bytes().to_vec() }

#[derive(Clone)]
enum EInst {
    Label(String), Bytes(Vec<u8>), Etype(EReg,EReg,EReg,u32,u32),
    Eimm(EReg,EReg,u32,u32), Eret, Branch(String,EReg,u32),
}

fn eenc(i: &EInst, arch: &str, off: u32, lm: &BTreeMap<String,u32>) -> Result<Vec<u8>,String> {
    Ok(match i {
        EInst::Label(_) => vec![],
        EInst::Bytes(b) => b.clone(),
        EInst::Etype(rd,rs,rt,op,funct) => {
            let d=ern(*rd); let s=ern(*rs); let t=ern(*rt);
            match arch {
                "alpha" => eu4(op|(s<<21)|(t<<16)|(d<<0)|funct),
                "parisc" => eu4(op|(d<<21)|(s<<16)|(t<<1)|funct),
                _ => vec![0;4],
            }
        }
        EInst::Eimm(rt,rs,imm,op) => {
            let s=ern(*rs); let t=ern(*rt);
            match arch {
                "alpha" => eu4(op|(s<<21)|(t<<16)|(*imm&0xFFFF)),
                "parisc" => eu4(op|(t<<21)|(s<<16)|(*imm&0x3FFF)),
                _ => vec![0;4],
            }
        }
        EInst::Branch(l,reg,op_base) => {
            let tgt = *lm.get(l).ok_or("bad label")?;
            let rel = (tgt as i32).wrapping_sub(off as i32 + 4);
            let disp = (rel >> 2) as u32 & 0x1FFFFF;
            match arch {
                "alpha" => eu4(op_base | (ern(*reg) << 21) | disp),
                "parisc" => eu4(op_base | (disp << 3)),
                _ => vec![0;4],
            }
        }
        EInst::Eret => match arch {
            "alpha" => eu4(0x6BFAC000),
            "parisc" => eu4(0xE840C002),
            _ => vec![0;4],
        },
    })
}

fn eparse(t: &str, arch: &str) -> Result<EInst, String> {
    let t = t.trim();
    if t.is_empty() || t.starts_with(';') { return Err("".into()); }
    if t.ends_with(':') { return Ok(EInst::Label(t[..t.len()-1].to_string())); }
    let p: Vec<&str> = t.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
    if p.is_empty() { return Err("".into()); }
    let m = p[0]; let r = p[1..].join(" ");
    let v: Vec<&str> = r.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();
    let gr = |s:&str| erp(s).ok_or_else(||format!("bad reg '{s}'"));
    match m {
        "nop" => Ok(EInst::Eimm(EReg::V0,EReg::V0,0,if arch=="alpha"{0x11<<26}else{0x0A<<26})),
        "ret" => Ok(EInst::Eret),
        "li" if v.len()==2 => {
            let rd=gr(v[0])?; let imm:u32=v[1].parse().map_err(|_|"bad imm")?;
            let op=if arch=="alpha"{0x11<<26}else{0x34<<26};
            Ok(EInst::Eimm(rd,EReg::V0,imm,op))
        }
        "add"|"sub" if v.len()==3 => {
            let rd=gr(v[0])?;let rs=gr(v[1])?;let rt=gr(v[2])?;
            let (op,funct)=match (m,arch){
                ("add","alpha")=>(0x10<<26,0x00),("sub","alpha")=>(0x10<<26,0x28),
                ("add","parisc")=>(0x08<<26,0x028),("sub","parisc")=>(0x08<<26,0x228),
                _=>return Err("not impl".into())
            };
            Ok(EInst::Etype(rd,rs,rt,op,funct))
        }
        _ => Err(format!("unknown '{m}' for {arch}"))
    }
}

fn earch(p: &Program, arch: &str) -> Result<Vec<u8>, String> {
    let mut items: Vec<EInst> = Vec::new();
    items.push(EInst::Eimm(EReg::V1,EReg::V0,0,0x11<<26));
    items.push(EInst::Label(format!("__event_{}", p.entry_event)));

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

    let (beq_op, br_op) = match arch {
        "alpha" => (0x39000000u32, 0x30000000u32),
        "parisc" => (0x90000000u32, 0xE8000000u32),
        _ => (0, 0),
    };

    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        items.push(EInst::Label(format!("{}{}", pr, b.name)));
        for l in &b.lines {
            let t = l.trim();
            if t.is_empty()||t.starts_with(';'){continue;}
            if t.ends_with(':'){items.push(EInst::Label(format!("{}.{}",b.name,t[..t.len()-1].trim())));continue;}
            if let Some(x)=t.strip_prefix("emit "){items.push(EInst::Eimm(EReg::V0,EReg::V0,0,0));items.push(EInst::Label(format!("__event_{}",x.trim())));continue;}
            if let Some(x)=t.strip_prefix("call "){items.push(EInst::Eimm(EReg::V0,EReg::V0,0,0));items.push(EInst::Label(format!("__proc_{}",x.trim())));continue;}
            if t=="ret"{items.push(EInst::Eret);continue;}

            // if <reg>
            if let Some(cond_str) = t.strip_prefix("if ") {
                let reg = erp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endif_label = format!("__cf_{}_endif", cf_counter);
                let else_label = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                let br_idx = items.len();
                items.push(EInst::Branch(endif_label.clone(), reg, beq_op));
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
                if let EInst::Branch(ref mut l, ..) = items[prev] { *l = elif_lbl.clone(); }
                items.push(EInst::Branch(frame.endif_label.clone(), EReg::V0, br_op));
                items.push(EInst::Label(elif_lbl));
                let reg = erp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let br_idx = items.len();
                items.push(EInst::Branch(frame.endif_label.clone(), reg, beq_op));
                frame.br_indices.push(br_idx);
                continue;
            }
            if t == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".into()); }
                frame.has_else = true;
                let prev = *frame.br_indices.last().ok_or("internal")?;
                if let EInst::Branch(ref mut l, ..) = items[prev] { *l = frame.else_label.clone(); }
                items.push(EInst::Branch(frame.endif_label.clone(), EReg::V0, br_op));
                items.push(EInst::Label(frame.else_label.clone()));
                continue;
            }
            if t == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
                items.push(EInst::Label(frame.endif_label.clone()));
                continue;
            }
            // while <reg>
            if let Some(cond_str) = t.strip_prefix("while ") {
                let reg = erp(cond_str.trim()).ok_or_else(|| format!("unknown reg '{}'", cond_str.trim()))?;
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                items.push(EInst::Label(start_lbl));
                let br_idx = items.len();
                items.push(EInst::Branch(endwhile_lbl.clone(), reg, beq_op));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_indices: vec![br_idx], has_else: false });
                continue;
            }
            if t == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                items.push(EInst::Branch(start_lbl, EReg::V0, br_op));
                items.push(EInst::Label(frame.endif_label.clone()));
                continue;
            }

            match eparse(t,arch){Ok(i)=>items.push(i),Err(_)=>return Err(format!("line '{t}'"))}
        }
    }

    if !cf_stack.is_empty() { return Err("unclosed if/while block".into()); }
    // Peephole - NOP compression + dead code elimination
    {
        let nop_op = if arch == "alpha" { 0x11u32 << 26 } else { 0x0Au32 << 26 };
        let is_nop = |i: &EInst| matches!(i, EInst::Eimm(EReg::V0, EReg::V0, 0, op) if *op == nop_op);
        let is_term = |i: &EInst| matches!(i, EInst::Eret);

        // NOP compression
        let mut i = 0;
        while i + 1 < items.len() {
            if is_nop(&items[i]) && is_nop(&items[i+1]) {
                items.remove(i + 1);
            } else {
                i += 1;
            }
        }

        // Dead code elimination after unconditional terminator
        let mut i = 0;
        while i < items.len() {
            if is_term(&items[i]) {
                let mut j = i + 1;
                while j < items.len() && !matches!(items[j], EInst::Label(_)) {
                    j += 1;
                }
                if j > i + 1 {
                    items.drain(i+1..j);
                }
            }
            i += 1;
        }
    }
    items.push(EInst::Label("__data".to_string()));
    for d in &p.data { match d {
        DataDecl::String{name,value}=>{let mut b=crate::direct_arch::expand_str(value);b.push(0);items.push(EInst::Label(format!("__data_{name}")));items.push(EInst::Bytes(b));}
        DataDecl::Scalar{name,width,value}=>{items.push(EInst::Label(format!("__data_{name}")));items.push(EInst::Bytes(match width{ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_le_bytes().to_vec(),ScalarWidth::Dword=>(*value as u32).to_le_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_le_bytes().to_vec(),}));}
        DataDecl::Buffer{name,size}=>{items.push(EInst::Label(format!("__data_{name}")));items.push(EInst::Bytes(vec![0u8;*size]));}
    }}
    let mut lm=BTreeMap::new();let mut o:u32=0;
    for i in &items{match i{EInst::Label(n)=>{lm.insert(n.clone(),o);},EInst::Bytes(b)=>o+=b.len()as u32,_=>o+=4}}
    let mut bin=Vec::new();
    for i in &items{match i{EInst::Label(_)=>{}EInst::Bytes(b)=>bin.extend_from_slice(b),_=>bin.extend_from_slice(&eenc(i,arch,bin.len()as u32,&lm)?)}}
    Ok(bin)
}
