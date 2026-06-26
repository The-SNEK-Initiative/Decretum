// IA-32 (i386) instruction encoder
// Shared between win32 (PE32) and elf32 (ELF32) backends

use std::collections::BTreeMap;
use crate::dcrt::{BlockKind, DataDecl, Program, ScalarWidth};

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum I386Reg { Eax, Ecx, Edx, Ebx, Esp, Ebp, Esi, Edi }

pub fn parse_i386_reg(s: &str) -> Option<I386Reg> {
    Some(match s.to_lowercase().as_str() {
        "eax"|"ax"|"al" => I386Reg::Eax, "ecx"|"cx"|"cl" => I386Reg::Ecx,
        "edx"|"dx"|"dl" => I386Reg::Edx, "ebx"|"bx"|"bl" => I386Reg::Ebx,
        "esp"|"sp"|"ah" => I386Reg::Esp, "ebp"|"bp"|"ch" => I386Reg::Ebp,
        "esi"|"si"|"dh" => I386Reg::Esi, "edi"|"di"|"bh" => I386Reg::Edi,
        _ => return None
    })
}

pub fn reg_num(r: I386Reg) -> u8 { r as u8 }
fn modrm(mod_:u8, reg:u8, rm:u8) -> u8 { (mod_<<6) | ((reg&7)<<3) | (rm&7) }

#[derive(Debug,Clone)]
pub enum Inst {
    Label(String), Bytes(Vec<u8>),
    MovRegImm(I386Reg, i32), MovRegReg(I386Reg, I386Reg),
    MovRegMem(I386Reg, String), MovMemReg(String, I386Reg),
    AddRegReg(I386Reg, I386Reg), SubRegReg(I386Reg, I386Reg),
    AddRegImm(I386Reg, i32), SubRegImm(I386Reg, i32),
    XorRegReg(I386Reg, I386Reg), AndRegReg(I386Reg, I386Reg), OrRegReg(I386Reg, I386Reg),
    IncReg(I386Reg), DecReg(I386Reg),
    MulReg(I386Reg), DivReg(I386Reg),
    Push(I386Reg), Pop(I386Reg),
    CmpRegReg(I386Reg, I386Reg), CmpRegImm(I386Reg, i32),
    Jmp(String), Call(String), Ret,
    Je(String), Jne(String), Jl(String), Jle(String), Jg(String), Jge(String),
    Nop, Int(u8),
}

fn encode_word(w: u16) -> Vec<u8> { w.to_le_bytes().to_vec() }

pub fn encode_inst(inst: &Inst, offset: usize, labels: &BTreeMap<String,u32>) -> Result<Vec<u8>,String> {
    let off = offset as u32;
    match inst {
        Inst::Label(_)=>Ok(vec![]), Inst::Bytes(b)=>Ok(b.clone()),
        Inst::MovRegImm(rd,val)=>{
            let r=reg_num(*rd);
            Ok(vec![0xB8|r, *val as u8, (*val>>8)as u8, (*val>>16)as u8, (*val>>24)as u8])
        }
        Inst::MovRegReg(d,s)=>{
            Ok(vec![0x89, modrm(3,reg_num(*d),reg_num(*s))])
        }
        Inst::MovRegMem(rd,label)=>{
            let t=*labels.get(label).ok_or(format!("unknown '{label}'"))?;
            let rel=(t as i64).wrapping_sub(off as i64+6) as i32;
            let mut b=vec![0x8B, modrm(0,reg_num(*rd),5)];
            b.extend_from_slice(&rel.to_le_bytes());
            Ok(b)
        }
        Inst::MovMemReg(label,src)=>{
            let t=*labels.get(label).ok_or(format!("unknown '{label}'"))?;
            let rel=(t as i64).wrapping_sub(off as i64+6) as i32;
            let mut b=vec![0x89, modrm(0,reg_num(*src),5)];
            b.extend_from_slice(&rel.to_le_bytes());
            Ok(b)
        }
        Inst::AddRegReg(d,s)=>Ok(vec![0x01, modrm(3,reg_num(*d),reg_num(*s))]),
        Inst::SubRegReg(d,s)=>Ok(vec![0x29, modrm(3,reg_num(*d),reg_num(*s))]),
        Inst::AddRegImm(r,v)=>{
            if *v>=-128&&*v<=127{Ok(vec![0x83, modrm(3,0,reg_num(*r)), *v as u8])}
            else{let mut b=vec![0x81, modrm(3,0,reg_num(*r))];b.extend_from_slice(&v.to_le_bytes());Ok(b)}
        }
        Inst::SubRegImm(r,v)=>{
            if *v>=-128&&*v<=127{Ok(vec![0x83, modrm(3,5,reg_num(*r)), *v as u8])}
            else{let mut b=vec![0x81, modrm(3,5,reg_num(*r))];b.extend_from_slice(&v.to_le_bytes());Ok(b)}
        }
        Inst::XorRegReg(d,s)=>Ok(vec![0x31, modrm(3,reg_num(*d),reg_num(*s))]),
        Inst::AndRegReg(d,s)=>Ok(vec![0x21, modrm(3,reg_num(*d),reg_num(*s))]),
        Inst::OrRegReg(d,s)=>Ok(vec![0x09, modrm(3,reg_num(*d),reg_num(*s))]),
        Inst::IncReg(r)=>Ok(vec![0x40|reg_num(*r)]),
        Inst::DecReg(r)=>Ok(vec![0x48|reg_num(*r)]),
        Inst::MulReg(r)=>Ok(vec![0xF7, modrm(3,4,reg_num(*r))]),
        Inst::DivReg(r)=>Ok(vec![0xF7, modrm(3,6,reg_num(*r))]),
        Inst::Push(r)=>Ok(vec![0x50|reg_num(*r)]),
        Inst::Pop(r)=>Ok(vec![0x58|reg_num(*r)]),
        Inst::CmpRegReg(a,b)=>Ok(vec![0x39, modrm(3,reg_num(*a),reg_num(*b))]),
        Inst::CmpRegImm(r,v)=>{
            if *v>=-128&&*v<=127{Ok(vec![0x83, modrm(3,7,reg_num(*r)), *v as u8])}
            else{let mut b=vec![0x81, modrm(3,7,reg_num(*r))];b.extend_from_slice(&v.to_le_bytes());Ok(b)}
        }
        Inst::Jmp(l)=>jmp_rel32(0xE9, l, off, labels),
        Inst::Call(l)=>jmp_rel32(0xE8, l, off, labels),
        Inst::Ret=>Ok(vec![0xC3]),
        Inst::Je(l)=>jcc_rel32(0x84, l, off, labels),
        Inst::Jne(l)=>jcc_rel32(0x85, l, off, labels),
        Inst::Jl(l)=>jcc_rel32(0x8C, l, off, labels),
        Inst::Jle(l)=>jcc_rel32(0x8E, l, off, labels),
        Inst::Jg(l)=>jcc_rel32(0x8F, l, off, labels),
        Inst::Jge(l)=>jcc_rel32(0x8D, l, off, labels),
        Inst::Nop=>Ok(vec![0x90]),
        Inst::Int(n)=>Ok(vec![0xCD, *n]),
    }
}

fn jmp_rel32(op:u8, label:&str, off:u32, labels:&BTreeMap<String,u32>)->Result<Vec<u8>,String>{
    let t=*labels.get(label).ok_or(format!("unknown '{label}'"))?;
    let rel=(t as i64).wrapping_sub(off as i64+5) as i32;
    let mut b=vec![op];b.extend_from_slice(&rel.to_le_bytes());Ok(b)
}

fn jcc_rel32(op2:u8, label:&str, off:u32, labels:&BTreeMap<String,u32>)->Result<Vec<u8>,String>{
    let t=*labels.get(label).ok_or(format!("unknown '{label}'"))?;
    let rel=(t as i64).wrapping_sub(off as i64+6) as i32;
    let mut b=vec![0x0F, op2];b.extend_from_slice(&rel.to_le_bytes());Ok(b)
}

pub fn lower(s:&str)->Result<Inst,String>{
    let t=s.trim();if t.is_empty()||t.starts_with(';'){return Err("".into())}
    if t.ends_with(':'){return Ok(Inst::Label(t[..t.len()-1].to_string()))}
    let p:Vec<&str>=t.splitn(4,|c:char|c==' '||c=='\t').filter(|s|!s.is_empty()).collect();
    if p.is_empty(){return Err("".into())}
    let m=p[0];let r=p[1..].join(" ");
    let v:Vec<&str>=r.split(',').map(|s|s.trim()).filter(|s|!s.is_empty()).collect();
    let rp=|s| parse_i386_reg(s).ok_or_else(|| format!("bad reg '{s}'"));
    match m {
        "mov" if v.len()==2=>{
            if let Ok(imm)=v[1].parse::<i32>(){return Ok(Inst::MovRegImm(rp(v[0])?,imm))}
            if v[1].starts_with('[')||v[0].starts_with('['){
                if v[0].starts_with('['){let label=v[0].trim_matches(|c|c=='['||c==']');return Ok(Inst::MovMemReg(label.to_string(),rp(v[1])?))}
                let label=v[1].trim_matches(|c|c=='['||c==']');return Ok(Inst::MovRegMem(rp(v[0])?,label.to_string()))
            }
            Ok(Inst::MovRegReg(rp(v[0])?,rp(v[1])?))
        }
        "add"|"sub"|"xor"|"and"|"or" if v.len()==2=>{
            let d=rp(v[0])?;
            if let Ok(imm)=v[1].parse::<i32>(){Ok(match m{"add"=>Inst::AddRegImm(d,imm),"sub"=>Inst::SubRegImm(d,imm),_=>return Err("no imm".into())})}
            else{let s=rp(v[1])?;Ok(match m{"add"=>Inst::AddRegReg(d,s),"sub"=>Inst::SubRegReg(d,s),"xor"=>Inst::XorRegReg(d,s),"and"=>Inst::AndRegReg(d,s),"or"=>Inst::OrRegReg(d,s),_=>unreachable!()})}
        }
        "inc"=>Ok(Inst::IncReg(rp(v[0])?)),"dec"=>Ok(Inst::DecReg(rp(v[0])?)),
        "mul"=>Ok(Inst::MulReg(rp(v[0])?)),"div"=>Ok(Inst::DivReg(rp(v[0])?)),
        "push"=>Ok(Inst::Push(rp(v[0])?)),"pop"=>Ok(Inst::Pop(rp(v[0])?)),
        "cmp" if v.len()==2=>{
            let a=rp(v[0])?;
            if let Ok(imm)=v[1].parse::<i32>(){Ok(Inst::CmpRegImm(a,imm))}else{Ok(Inst::CmpRegReg(a,rp(v[1])?))}
        }
        "jmp"=>Ok(Inst::Jmp(v[0].to_string())),"call"=>Ok(Inst::Call(v[0].to_string())),
        "ret"=>Ok(Inst::Ret),
        "je"|"jz"=>Ok(Inst::Je(v[0].to_string())),"jne"|"jnz"=>Ok(Inst::Jne(v[0].to_string())),
        "jl"|"jnge"=>Ok(Inst::Jl(v[0].to_string())),"jle"|"jng"=>Ok(Inst::Jle(v[0].to_string())),
        "jg"|"jnle"=>Ok(Inst::Jg(v[0].to_string())),"jge"|"jnl"=>Ok(Inst::Jge(v[0].to_string())),
        "nop"=>Ok(Inst::Nop),"int"=>Ok(Inst::Int(v[0].parse().map_err(|_|"bad int")?)),

        _=>Err(format!("unknown i386 '{m}'"))
    }
}

pub fn expand_str(s:&str)->Vec<u8>{
    let mut b=Vec::new();let c:Vec<char>=s.chars().collect();let mut i=0;
    while i<c.len(){if c[i]=='\\'&&i+1<c.len(){match c[i+1]{'n'=>b.push(b'\n'),'r'=>b.push(b'\r'),'t'=>b.push(b'\t'),'0'=>b.push(0),'\\'=>b.push(b'\\'),'"'=>b.push(b'"'),o=>{b.push(b'\\');b.push(o as u8)}}i+=2}else{b.push(c[i]as u8);i+=1}}b.push(0);b
}

pub struct I386Assembler;

impl I386Assembler {
    pub fn assemble(program:&Program)->Result<Vec<u8>,String>{
        let mut items:Vec<Inst>=Vec::new();
        let entry=format!("__event_{}",program.entry_event);
        items.push(Inst::Label("_start".to_string()));
        items.push(Inst::MovRegImm(I386Reg::Eax,42));
        items.push(Inst::Call(entry));
        items.push(Inst::MovRegImm(I386Reg::Eax,0));
        items.push(Inst::Ret);

        // Control flow construct stack (if/else/endif/while/endwhile)
        struct CfFrame {
            kind: CfKind,
            endif_label: String,
            else_label: String,
            beqz_indices: Vec<usize>,
            has_else: bool,
        }
        #[derive(PartialEq)]
        enum CfKind { If, While }
        let mut cf_stack: Vec<CfFrame> = Vec::new();
        let mut cf_counter: u32 = 0;

        for block in &program.blocks {
            let p=match block.kind{BlockKind::Event=>"__event_",BlockKind::Proc=>"__proc_"};
            items.push(Inst::Label(format!("{}{}",p,block.name)));
            for line in &block.lines {
                let t=line.trim();if t.is_empty()||t.starts_with(';'){continue}
                if t.ends_with(':'){items.push(Inst::Label(format!("{}.{}",block.name,t[..t.len()-1].trim())));continue}
                if let Some(target)=t.strip_prefix("emit "){items.push(Inst::Call(format!("__event_{}",target.trim())));continue}
                if let Some(target)=t.strip_prefix("call "){items.push(Inst::Call(format!("__proc_{}",target.trim())));continue}
                if t=="ret"{items.push(Inst::Ret);continue}

                // if <reg>
                if let Some(cond_str) = t.strip_prefix("if ") {
                    let reg = match cond_str.trim().parse::<i32>() {
                        Ok(n) => {
                            items.push(Inst::MovRegImm(I386Reg::Eax, n));
                            I386Reg::Eax
                        }
                        Err(_) => {
                            parse_i386_reg(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for if", cond_str.trim()))?
                        }
                    };
                    let endif_lbl = format!("__cf_{}_endif", cf_counter);
                    let else_lbl = format!("__cf_{}_else", cf_counter);
                    cf_counter += 1;
                    items.push(Inst::CmpRegImm(reg, 0));
                    let beqz_idx = items.len();
                    items.push(Inst::Je(endif_lbl.clone()));
                    cf_stack.push(CfFrame {
                        kind: CfKind::If,
                        endif_label: endif_lbl,
                        else_label: else_lbl,
                        beqz_indices: vec![beqz_idx],
                        has_else: false,
                    });
                    continue;
                }

                // elif <reg>
                if let Some(cond_str) = t.strip_prefix("elif ") {
                    let frame = cf_stack.last_mut().ok_or("elif without if")?;
                    if frame.has_else {
                        return Err("elif after else".to_string());
                    }
                    let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.beqz_indices.len());
                    cf_counter += 1;
                    let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                    if let Inst::Je(ref mut label) = items[*prev] {
                        *label = elif_lbl.clone();
                    }
                    items.push(Inst::Jmp(frame.endif_label.clone()));
                    items.push(Inst::Label(elif_lbl));
                    let reg = match cond_str.trim().parse::<i32>() {
                        Ok(n) => {
                            items.push(Inst::MovRegImm(I386Reg::Eax, n));
                            I386Reg::Eax
                        }
                        Err(_) => {
                            parse_i386_reg(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for elif", cond_str.trim()))?
                        }
                    };
                    items.push(Inst::CmpRegImm(reg, 0));
                    let beqz_idx = items.len();
                    items.push(Inst::Je(frame.endif_label.clone()));
                    frame.beqz_indices.push(beqz_idx);
                    continue;
                }

                // else
                if t == "else" {
                    let frame = cf_stack.last_mut().ok_or("else without if")?;
                    if frame.has_else {
                        return Err("duplicate else".to_string());
                    }
                    frame.has_else = true;
                    let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                    if let Inst::Je(ref mut label) = items[*prev] {
                        *label = frame.else_label.clone();
                    }
                    items.push(Inst::Jmp(frame.endif_label.clone()));
                    items.push(Inst::Label(frame.else_label.clone()));
                    continue;
                }

                // endif
                if t == "endif" {
                    let frame = cf_stack.pop().ok_or("endif without if/while")?;
                    if frame.kind == CfKind::While {
                        return Err("endif without matching if".to_string());
                    }
                    items.push(Inst::Label(frame.endif_label.clone()));
                    continue;
                }

                // while <reg>
                if let Some(cond_str) = t.strip_prefix("while ") {
                    let reg = match cond_str.trim().parse::<i32>() {
                        Ok(n) => {
                            items.push(Inst::MovRegImm(I386Reg::Eax, n));
                            I386Reg::Eax
                        }
                        Err(_) => {
                            parse_i386_reg(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for while", cond_str.trim()))?
                        }
                    };
                    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                    let start_lbl = format!("__cf_{}_start", cf_counter);
                    cf_counter += 1;
                    items.push(Inst::Label(start_lbl));
                    items.push(Inst::CmpRegImm(reg, 0));
                    let beqz_idx = items.len();
                    items.push(Inst::Je(endwhile_lbl.clone()));
                    cf_stack.push(CfFrame {
                        kind: CfKind::While,
                        endif_label: endwhile_lbl,
                        else_label: String::new(),
                        beqz_indices: vec![beqz_idx],
                        has_else: false,
                    });
                    continue;
                }

                // endwhile
                if t == "endwhile" {
                    let frame = cf_stack.pop().ok_or("endwhile without while")?;
                    if frame.kind != CfKind::While {
                        return Err("endwhile without matching while".to_string());
                    }
                    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                    items.push(Inst::Jmp(start_lbl));
                    items.push(Inst::Label(frame.endif_label.clone()));
                    continue;
                }

                match lower(t){Ok(i)=>items.push(i),Err(e)=>return Err(format!("line '{t}': {e}"))}
            }
        }

        if !cf_stack.is_empty() {
            return Err("unclosed if/while block".to_string());
        }

        // Peephole
        {
            let mut i = 0;
            while i < items.len() {
                let is_nop = |x: &Inst| matches!(x, Inst::Nop);
                let is_term = |x: &Inst| matches!(x, Inst::Jmp(_)|Inst::Ret|Inst::Call(_));
                let is_label = |x: &Inst| matches!(x, Inst::Label(_));
                if i + 1 < items.len() && is_nop(&items[i]) && is_nop(&items[i+1]) { items.remove(i+1); continue; }
                if is_term(&items[i]) {
                    let mut j = i + 1;
                    while j < items.len() && !is_label(&items[j]) { j += 1; }
                    if j > i + 1 { items.drain(i+1..j); }
                }
                i += 1;
            }
        }
        items.push(Inst::Label("__data".to_string()));
        for d in &program.data{match d{
            DataDecl::String{name,value}=>{items.push(Inst::Label(format!("__data_{name}")));items.push(Inst::Bytes(expand_str(value)));}
            DataDecl::Scalar{name,width,value}=>{items.push(Inst::Label(format!("__data_{name}")));items.push(Inst::Bytes(match width{
                ScalarWidth::Byte=>vec![*value as u8],ScalarWidth::Word=>(*value as u16).to_le_bytes().to_vec(),
                ScalarWidth::Dword=>(*value as u32).to_le_bytes().to_vec(),ScalarWidth::Qword=>(*value as u64).to_le_bytes().to_vec(),}));}
            DataDecl::Buffer{name,size}=>{items.push(Inst::Label(format!("__data_{name}")));items.push(Inst::Bytes(vec![0u8;*size]));}
        }}
        let labels=layout(&items)?;encode_items(&items,&labels,0)
    }
}

fn layout(items:&[Inst])->Result<BTreeMap<String,u32>,String>{
    let mut m=BTreeMap::new();let mut o:u32=0;
    for item in items{match item{
        Inst::Label(n)=>{if m.insert(n.clone(),o).is_some(){return Err(format!("dup '{n}'"))}}
        Inst::Bytes(b)=>o+=b.len()as u32,
        _=>o+=5,
    }}Ok(m)
}

fn encode_items(items:&[Inst],labels:&BTreeMap<String,u32>,base:u32)->Result<Vec<u8>,String>{
    let mut bin=Vec::new();
    for item in items{if let Inst::Label(_)=item{}else if let Inst::Bytes(b)=item{bin.extend_from_slice(b);}else{bin.extend_from_slice(&encode_inst(item,bin.len()as usize,labels)?);}}
    Ok(bin)
}
