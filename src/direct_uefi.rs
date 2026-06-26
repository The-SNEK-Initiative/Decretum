// Those who enter this place should abandon all hope.
// Jokes aside, a direct UEFI compiler, nothing more to add here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dcrt::{BlockKind, DataDecl, Program, ScalarWidth};

pub struct UefiBuildOutput {
    pub efi_path: PathBuf,
    pub efi_size: usize,
}

pub struct DirectUefiBuilder;

impl DirectUefiBuilder {
    pub fn build_efi(program: &Program, out_path: &Path) -> Result<UefiBuildOutput, String> {
        if program.target != "uefi" {
            return Err(format!(
                "direct uefi backend requires target 'uefi', got '{}'",
                program.target
            ));
        }
        let kernel = DirectUefiAssembler::assemble(program)?;
        let efi = build_pe32_plus(&kernel, program)?;
        std::fs::write(out_path, &efi).map_err(|e| format!("failed to write output: {e}"))?;
        Ok(UefiBuildOutput {
            efi_path: out_path.to_path_buf(),
            efi_size: efi.len(),
        })
    }
}

// x86-64 register definitions
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reg64 {
    Rax, Rcx, Rdx, Rbx, Rsp, Rbp, Rsi, Rdi,
    R8, R9, R10, R11, R12, R13, R14, R15,
}

fn parse_reg64(s: &str) -> Option<Reg64> {
    Some(match s.to_lowercase().as_str() {
        "rax" | "eax" | "ax" | "al" => Reg64::Rax,
        "rcx" | "ecx" | "cx" | "cl" => Reg64::Rcx,
        "rdx" | "edx" | "dx" | "dl" => Reg64::Rdx,
        "rbx" | "ebx" | "bx" | "bl" => Reg64::Rbx,
        "rsp" | "esp" | "sp" | "spl" => Reg64::Rsp,
        "rbp" | "ebp" | "bp" | "bpl" => Reg64::Rbp,
        "rsi" | "esi" | "si" | "sil" => Reg64::Rsi,
        "rdi" | "edi" | "di" | "dil" => Reg64::Rdi,
        "r8" | "r8d" | "r8w" | "r8b" => Reg64::R8,
        "r9" | "r9d" | "r9w" | "r9b" => Reg64::R9,
        "r10" | "r10d" | "r10w" | "r10b" => Reg64::R10,
        "r11" | "r11d" | "r11w" | "r11b" => Reg64::R11,
        "r12" | "r12d" | "r12w" | "r12b" => Reg64::R12,
        "r13" | "r13d" | "r13w" | "r13b" => Reg64::R13,
        "r14" | "r14d" | "r14w" | "r14b" => Reg64::R14,
        "r15" | "r15d" | "r15w" | "r15b" => Reg64::R15,
        _ => return None,
    })
}

fn reg64_num(r: Reg64) -> u8 {
    r as u8
}

fn rex_b(r: Reg64) -> u8 {
    if r as u8 >= 8 { 1 } else { 0 }
}
fn rex_x(_r: Reg64) -> u8 { 0 }
fn rex_r(r: Reg64) -> u8 {
    if r as u8 >= 8 { 1 } else { 0 }
}

fn rex_prefix(w: bool, r: u8, x: u8, b: u8) -> u8 {
    0x40 | ((w as u8) << 3) | (r << 2) | (x << 1) | b
}

fn modrm(mod_: u8, reg: u8, rm: u8) -> u8 {
    (mod_ << 6) | ((reg & 7) << 3) | (rm & 7)
}

fn rex_rb(w: bool, r: Reg64, b: Reg64) -> u8 {
    rex_prefix(w, reg64_num(r) >> 3, 0, reg64_num(b) >> 3)
}

// Instruction set
#[derive(Debug, Clone)]
enum Inst {
    Label(String),
    Bytes(Vec<u8>),
    // Data movement
    MovReg64Imm(Reg64, i64),
    MovReg64Reg64(Reg64, Reg64),
    MovReg32Imm(Reg64, u32),
    MovReg64Mem64(Reg64, String),
    MovMem64Reg64(String, Reg64),
    LeaReg64Mem(Reg64, String),
    AddReg64Reg64(Reg64, Reg64),
    AddReg64Imm(Reg64, i32),
    SubReg64Reg64(Reg64, Reg64),
    SubReg64Imm(Reg64, i32),
    XorReg64Reg64(Reg64, Reg64),
    AndReg64Reg64(Reg64, Reg64),
    OrReg64Reg64(Reg64, Reg64),
    IncReg64(Reg64),
    DecReg64(Reg64),
    MulReg64(Reg64),
    DivReg64(Reg64),
    NotReg64(Reg64),
    NegReg64(Reg64),
    Push(Reg64),
    Pop(Reg64),
    Jmp(String),
    Call(String),
    Ret,
    Je(String),  Jne(String),  Jl(String),  Jle(String),
    Jg(String),  Jge(String),  Jb(String),  Jbe(String),
    Ja(String),  Jae(String),
    CmpReg64Reg64(Reg64, Reg64),
    CmpReg64Imm(Reg64, i32),
    TestReg64Reg64(Reg64, Reg64),
    Nop,
    UefiOutputString,          // OutputString(SystemTable, L"text")
    UefiClearScreen,           // ClearScreen(SystemTable)
    UefiWaitForKey,            // WaitForKey
}

fn encode_x64_inst(inst: &Inst, offset: usize, label_map: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let off = offset as u32;
    match inst {
        Inst::Label(_) => Ok(vec![]),
        Inst::Bytes(bytes) => Ok(bytes.clone()),
        // mov r64, imm64: REX.W B8+rd io
        Inst::MovReg64Imm(rd, val) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let mut bytes = vec![rex, 0xB8 | (r & 7)];
            bytes.extend_from_slice(&val.to_le_bytes());
            Ok(bytes)
        }
        // mov r64, r64: REX.W 89 /r (or 8B /r for reg->reg)
        Inst::MovReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            // Using 89 /r: mov r/m64, r64
            Ok(vec![rex, 0x89, modrm_byte])
        }
        // mov r32, imm32: REX B8+rd id
        Inst::MovReg32Imm(rd, val) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(false, 0, 0, r >> 3);
            let mut bytes = vec![rex, 0xB8 | (r & 7)];
            bytes.extend_from_slice(&val.to_le_bytes());
            Ok(bytes)
        }
        // xor r64, r64: REX.W 33 /r
        Inst::XorReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            Ok(vec![rex, 0x33, modrm_byte])
        }
        // add r64, r64: REX.W 01 /r
        Inst::AddReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            Ok(vec![rex, 0x01, modrm_byte])
        }
        // add r64, imm32: REX.W 83/81 /0 id
        Inst::AddReg64Imm(rd, val) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 0, r & 7);
            if *val >= -128 && *val <= 127 {
                let mut bytes = vec![rex, 0x83, modrm_byte];
                bytes.push(*val as u8);
                Ok(bytes)
            } else {
                let mut bytes = vec![rex, 0x81, modrm_byte];
                bytes.extend_from_slice(&val.to_le_bytes());
                Ok(bytes)
            }
        }
        // sub r64, r64: REX.W 29 /r
        Inst::SubReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            Ok(vec![rex, 0x29, modrm_byte])
        }
        // sub r64, imm32
        Inst::SubReg64Imm(rd, val) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 5, r & 7); // 5 = sub opcode extension
            if *val >= -128 && *val <= 127 {
                let mut bytes = vec![rex, 0x83, modrm_byte];
                bytes.push(*val as u8);
                Ok(bytes)
            } else {
                let mut bytes = vec![rex, 0x81, modrm_byte];
                bytes.extend_from_slice(&val.to_le_bytes());
                Ok(bytes)
            }
        }
        // and r64, r64: REX.W 21 /r
        Inst::AndReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            Ok(vec![rex, 0x21, modrm_byte])
        }
        // or r64, r64: REX.W 09 /r
        Inst::OrReg64Reg64(dst, src) => {
            let rex = rex_prefix(true, reg64_num(*dst) >> 3, 0, reg64_num(*src) >> 3);
            let modrm_byte = modrm(3, reg64_num(*dst) & 7, reg64_num(*src) & 7);
            Ok(vec![rex, 0x09, modrm_byte])
        }
        // inc r64: REX.W FF /0
        Inst::IncReg64(rd) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 0, r & 7);
            Ok(vec![rex, 0xFF, modrm_byte])
        }
        // dec r64: REX.W FF /1
        Inst::DecReg64(rd) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 1, r & 7);
            Ok(vec![rex, 0xFF, modrm_byte])
        }
        // mul r64: REX.W F7 /4 (unsigned mul RDX:RAX = RAX * r/m64)
        Inst::MulReg64(rm) => {
            let r = reg64_num(*rm);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 4, r & 7);
            Ok(vec![rex, 0xF7, modrm_byte])
        }
        // div r64: REX.W F7 /6 (unsigned div RDX:RAX / r/m64)
        Inst::DivReg64(rm) => {
            let r = reg64_num(*rm);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 6, r & 7);
            Ok(vec![rex, 0xF7, modrm_byte])
        }
        // not r64: REX.W F7 /2
        Inst::NotReg64(rd) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 2, r & 7);
            Ok(vec![rex, 0xF7, modrm_byte])
        }
        // neg r64: REX.W F7 /3
        Inst::NegReg64(rd) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 3, r & 7);
            Ok(vec![rex, 0xF7, modrm_byte])
        }
        // push r64: 50+rd (with REX for extended regs)
        Inst::Push(rd) => {
            let r = reg64_num(*rd);
            let mut bytes = Vec::new();
            if r >= 8 {
                bytes.push(0x41); // REX.B
            }
            bytes.push(0x50 | (r & 7));
            Ok(bytes)
        }
        // pop r64: 58+rd
        Inst::Pop(rd) => {
            let r = reg64_num(*rd);
            let mut bytes = Vec::new();
            if r >= 8 {
                bytes.push(0x41); // REX.B
            }
            bytes.push(0x58 | (r & 7));
            Ok(bytes)
        }
        Inst::Nop => Ok(vec![0x90]),
        Inst::Ret => Ok(vec![0xC3]),
        // cmp r64, r64: REX.W 39 /r
        Inst::CmpReg64Reg64(a, b) => {
            let rex = rex_prefix(true, reg64_num(*a) >> 3, 0, reg64_num(*b) >> 3);
            let modrm_byte = modrm(3, reg64_num(*a) & 7, reg64_num(*b) & 7);
            Ok(vec![rex, 0x39, modrm_byte])
        }
        // cmp r64, imm32: REX.W 83/81 /7 id
        Inst::CmpReg64Imm(rd, val) => {
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, 0, 0, r >> 3);
            let modrm_byte = modrm(3, 7, r & 7);
            if *val >= -128 && *val <= 127 {
                Ok(vec![rex, 0x83, modrm_byte, *val as u8])
            } else {
                let mut bytes = vec![rex, 0x81, modrm_byte];
                bytes.extend_from_slice(&val.to_le_bytes());
                Ok(bytes)
            }
        }
        // test r64, r64: REX.W 85 /r
        Inst::TestReg64Reg64(a, b) => {
            let rex = rex_prefix(true, reg64_num(*a) >> 3, 0, reg64_num(*b) >> 3);
            let modrm_byte = modrm(3, reg64_num(*a) & 7, reg64_num(*b) & 7);
            Ok(vec![rex, 0x85, modrm_byte])
        }
        // RIP-relative jmp/call: E8 (call) or E9 (jmp) rel32
        Inst::Jmp(label) | Inst::Call(label) => {
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = (target as i64).wrapping_sub(off as i64 + 5) as i32;
            let is_call = matches!(inst, Inst::Call(_));
            let mut bytes = vec![if is_call { 0xE8 } else { 0xE9 }];
            bytes.extend_from_slice(&rel.to_le_bytes());
            Ok(bytes)
        }
        // Conditional jumps: 0F 8x rel32
        Inst::Je(l) => jcc_rel32(0x84, l, off, label_map),
        Inst::Jne(l) => jcc_rel32(0x85, l, off, label_map),
        Inst::Jl(l) => jcc_rel32(0x8C, l, off, label_map),
        Inst::Jle(l) => jcc_rel32(0x8E, l, off, label_map),
        Inst::Jg(l) => jcc_rel32(0x8F, l, off, label_map),
        Inst::Jge(l) => jcc_rel32(0x8D, l, off, label_map),
        Inst::Jb(l) => jcc_rel32(0x82, l, off, label_map),
        Inst::Jbe(l) => jcc_rel32(0x86, l, off, label_map),
        Inst::Ja(l) => jcc_rel32(0x87, l, off, label_map),
        Inst::Jae(l) => jcc_rel32(0x83, l, off, label_map),
        // lea r64, [rip + label]: REX.W 8D /r rel32
        Inst::LeaReg64Mem(rd, label) => {
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = (target as i64).wrapping_sub(off as i64 + 7) as i32;
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, r >> 3, 0, 0);
            let modrm_byte = modrm(0, r & 7, 5); // 5 = RIP-relative
            let mut bytes = vec![rex, 0x8D, modrm_byte];
            bytes.extend_from_slice(&rel.to_le_bytes());
            Ok(bytes)
        }
        // mov rax, [label]: REX.W 8B /r (RIP-relative)
        Inst::MovReg64Mem64(rd, label) => {
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = (target as i64).wrapping_sub(off as i64 + 7) as i32;
            let r = reg64_num(*rd);
            let rex = rex_prefix(true, r >> 3, 0, 0);
            let modrm_byte = modrm(0, r & 7, 5);
            let mut bytes = vec![rex, 0x8B, modrm_byte];
            bytes.extend_from_slice(&rel.to_le_bytes());
            Ok(bytes)
        }
        // mov [label], rax: REX.W 89 /r (RIP-relative)
        Inst::MovMem64Reg64(label, src) => {
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = (target as i64).wrapping_sub(off as i64 + 7) as i32;
            let r = reg64_num(*src);
            let rex = rex_prefix(true, r >> 3, 0, 0);
            let modrm_byte = modrm(0, r & 7, 5);
            let mut bytes = vec![rex, 0x89, modrm_byte];
            bytes.extend_from_slice(&rel.to_le_bytes());
            Ok(bytes)
        }
        // UEFI builtins - expanded during lowering
        _ => Err(format!("instruction cannot encode directly")),
    }
}

fn jcc_rel32(opcode2: u8, label: &str, offset: u32, label_map: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
    let rel = (target as i64).wrapping_sub(offset as i64 + 6) as i32;
    let mut bytes = vec![0x0F, opcode2];
    bytes.extend_from_slice(&rel.to_le_bytes());
    Ok(bytes)
}

fn lower_x64_line(line: &str) -> Result<Inst, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() { return Err("empty".to_string()); }
    if trimmed.starts_with(';') { return Err("comment".to_string()); }
    if trimmed.ends_with(':') {
        return Ok(Inst::Label(trimmed[..trimmed.len()-1].to_string()));
    }

    let parts: Vec<&str> = trimmed.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() { return Err("empty".to_string()); }
    let mnemonic = parts[0];
    let rest = parts[1..].join(" ");

    match mnemonic {
        "mov" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 {
                return Err(format!("bad mov syntax: {rest}"));
            }
            // Try label reference first
            if args[1].starts_with('[') || args[1].contains("__data") || args[1].contains("__event") || args[1].contains("__proc") {
                // mov reg, [label]
                let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                let label = args[1].trim_matches(|c| c == '[' || c == ']' || c == ' ');
                return Ok(Inst::MovReg64Mem64(rd, label.to_string()));
            }
            if args[0].starts_with('[') || args[0].contains("__data") {
                // mov [label], reg
                let src = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                let label = args[0].trim_matches(|c| c == '[' || c == ']' || c == ' ');
                return Ok(Inst::MovMem64Reg64(label.to_string(), src));
            }
            // Try immediate
            if let Ok(imm) = args[1].parse::<i64>() {
                let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                if imm >= 0 && imm <= 0xFFFFFFFF {
                    return Ok(Inst::MovReg32Imm(rd, imm as u32));
                }
                return Ok(Inst::MovReg64Imm(rd, imm));
            }
            let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            let rs = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
            Ok(Inst::MovReg64Reg64(rd, rs))
        }
        "add" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 { return Err(format!("bad add syntax: {rest}")); }
            let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            if let Ok(imm) = args[1].parse::<i32>() {
                Ok(Inst::AddReg64Imm(rd, imm))
            } else {
                let rs = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                Ok(Inst::AddReg64Reg64(rd, rs))
            }
        }
        "sub" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 { return Err(format!("bad sub syntax: {rest}")); }
            let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            if let Ok(imm) = args[1].parse::<i32>() {
                Ok(Inst::SubReg64Imm(rd, imm))
            } else {
                let rs = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                Ok(Inst::SubReg64Reg64(rd, rs))
            }
        }
        "xor" => parse_binop(&rest, |a, b| Inst::XorReg64Reg64(a, b)),
        "and" => parse_binop(&rest, |a, b| Inst::AndReg64Reg64(a, b)),
        "or" => parse_binop(&rest, |a, b| Inst::OrReg64Reg64(a, b)),
        "inc" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::IncReg64(rd))
        }
        "dec" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::DecReg64(rd))
        }
        "mul" => {
            let rs = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::MulReg64(rs))
        }
        "div" => {
            let rs = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::DivReg64(rs))
        }
        "not" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::NotReg64(rd))
        }
        "neg" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::NegReg64(rd))
        }
        "cmp" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 { return Err(format!("bad cmp syntax: {rest}")); }
            let ra = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            if let Ok(imm) = args[1].parse::<i32>() {
                Ok(Inst::CmpReg64Imm(ra, imm))
            } else {
                let rb = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                Ok(Inst::CmpReg64Reg64(ra, rb))
            }
        }
        "push" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::Push(rd))
        }
        "pop" => {
            let rd = parse_reg64(rest.trim()).ok_or_else(|| format!("unknown register '{}'", rest.trim()))?;
            Ok(Inst::Pop(rd))
        }
        "jmp" => Ok(Inst::Jmp(rest.trim().to_string())),
        "call" => {
            // Check if its a label or register
            if let Some(_) = parse_reg64(rest.trim()) {
                // call reg - not directly supported, use for UEFI func ptrs
                return Err("call reg not supported, use call <label>".to_string());
            }
            Ok(Inst::Call(rest.trim().to_string()))
        }
        "ret" => Ok(Inst::Ret),
        "je" | "jz" => Ok(Inst::Je(rest.trim().to_string())),
        "jne" | "jnz" => Ok(Inst::Jne(rest.trim().to_string())),
        "jl" | "jnge" => Ok(Inst::Jl(rest.trim().to_string())),
        "jle" | "jng" => Ok(Inst::Jle(rest.trim().to_string())),
        "jg" | "jnle" => Ok(Inst::Jg(rest.trim().to_string())),
        "jge" | "jnl" => Ok(Inst::Jge(rest.trim().to_string())),
        "jb" | "jc" | "jnae" => Ok(Inst::Jb(rest.trim().to_string())),
        "jbe" | "jna" => Ok(Inst::Jbe(rest.trim().to_string())),
        "ja" | "jnbe" => Ok(Inst::Ja(rest.trim().to_string())),
        "jae" | "jnc" | "jnb" => Ok(Inst::Jae(rest.trim().to_string())),
        "lea" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 { return Err(format!("bad lea syntax: {rest}")); }
            let rd = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            let label = args[1].trim_matches(|c| c == '[' || c == ']' || c == ' ');
            Ok(Inst::LeaReg64Mem(rd, label.to_string()))
        }
        "test" => {
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() != 2 { return Err(format!("bad test syntax: {rest}")); }
            let ra = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
            let rb = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
            Ok(Inst::TestReg64Reg64(ra, rb))
        }
        "nop" => Ok(Inst::Nop),
        // UEFI builtins
        "uefi_output_string" => Ok(Inst::UefiOutputString),
        "uefi_clear_screen" => Ok(Inst::UefiClearScreen),
        "uefi_wait_key" => Ok(Inst::UefiWaitForKey),
        _ => Err(format!("unknown x86-64 instruction '{mnemonic}'")),
    }
}

fn parse_binop<F>(rest: &str, f: F) -> Result<Inst, String>
where F: FnOnce(Reg64, Reg64) -> Inst {
    let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 2 { return Err(format!("expected 2 register arguments, got {}", args.len())); }
    let r1 = parse_reg64(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let r2 = parse_reg64(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
    Ok(f(r1, r2))
}

fn expand_string(s: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => bytes.push(b'\n'),
                'r' => bytes.push(b'\r'),
                't' => bytes.push(b'\t'),
                '0' => bytes.push(0),
                '\\' => bytes.push(b'\\'),
                '"' => bytes.push(b'"'),
                other => { bytes.push(b'\\'); bytes.push(other as u8); }
            }
            i += 2;
        } else {
            bytes.push(chars[i] as u8);
            i += 1;
        }
    }
    bytes.push(0);
    bytes
}

// PE32+ image builder
fn build_pe32_plus(kernel: &[u8], _program: &Program) -> Result<Vec<u8>, String> {
    // PE32+ header layout:
    // [64B DOS header] [PE sig] [20B COFF header] [112B Optional header] [40B Section table] = 236B headers
    // Then .text section with kernel code

    let dos_size = 64u32;
    let pe_sig_offset = dos_size;
    let coff_offset = pe_sig_offset + 4;
    let opt_offset = coff_offset + 20;
    let sect_offset = opt_offset + 112;
    let header_size = sect_offset + 40;
    let file_align = 0x200u32;
    let sect_align = 0x1000u32;

    // Align kernel data to file alignment
    let code_size = ((kernel.len() as u32 + file_align - 1) / file_align) * file_align;

    let size_of_image = sect_align + ((kernel.len() as u32 + sect_align - 1) / sect_align) * sect_align;

    let mut efi = Vec::new();

    // DOS header
    efi.push(0x4D); efi.push(0x5A); // "MZ"
    // Fill to offset 0x3C
    while efi.len() < 0x3C {
        efi.push(0);
    }
    // PE signature offset
    efi.extend_from_slice(&pe_sig_offset.to_le_bytes());
    while efi.len() < pe_sig_offset as usize {
        efi.push(0);
    }

    // PE signature
    efi.push(b'P'); efi.push(b'E'); efi.push(0); efi.push(0);

    // COFF header (20 bytes)
    let machine: u16 = 0x8664; // IMAGE_FILE_MACHINE_AMD64
    let num_sections: u16 = 1;
    let time_stamp: u32 = 0;
    let ptr_to_symtab: u32 = 0;
    let num_syms: u32 = 0;
    let opt_header_size: u16 = 112; // PE32+ optional header size
    let characteristics: u16 = 0x0002; // IMAGE_FILE_EXECUTABLE_IMAGE

    efi.extend_from_slice(&machine.to_le_bytes());
    efi.extend_from_slice(&num_sections.to_le_bytes());
    efi.extend_from_slice(&time_stamp.to_le_bytes());
    efi.extend_from_slice(&ptr_to_symtab.to_le_bytes());
    efi.extend_from_slice(&num_syms.to_le_bytes());
    efi.extend_from_slice(&opt_header_size.to_le_bytes());
    efi.extend_from_slice(&characteristics.to_le_bytes());

    // Optional header PE32+ (112 bytes)
    let magic: u16 = 0x020B; // PE32+
    let major_linker: u8 = 1;
    let minor_linker: u8 = 0;
    let code_size_val = code_size;
    let init_data_size: u32 = 0;
    let uninit_data_size: u32 = 0;
    let entry_rva: u32 = sect_align + 0x10; // .text section base + small offset for entry stub
    let code_base: u32 = sect_align; // .text section RVA
    let image_base: u64 = 0; // relocatable
    let section_align_val: u32 = sect_align;
    let file_align_val: u32 = file_align;
    let major_os: u16 = 6;
    let minor_os: u16 = 0;
    let major_image: u16 = 0;
    let minor_image: u16 = 0;
    let major_subsys: u16 = 6;
    let minor_subsys: u16 = 0;
    let win32_version: u32 = 0;
    let size_of_image_val = size_of_image;
    let size_of_headers: u32 = header_size;
    let check_sum: u32 = 0;
    let subsystem: u16 = 10; // IMAGE_SUBSYSTEM_EFI_APPLICATION
    let dll_characteristics: u16 = 0;
    let size_of_stack_reserve: u64 = 0;
    let size_of_stack_commit: u64 = 0;
    let size_of_heap_reserve: u64 = 0;
    let size_of_heap_commit: u64 = 0;
    let loader_flags: u32 = 0;
    let num_rva_sizes: u32 = 16;
    let export_rva: u64 = 0;
    let export_size: u32 = 0;
    let _import_rva: u64 = 0;
    let _import_size: u32 = 0;
    let _resource_rva: u64 = (sect_align + code_size) as u64;
    let _resource_size: u32 = 0;

    efi.extend_from_slice(&magic.to_le_bytes());
    efi.extend_from_slice(&major_linker.to_le_bytes());
    efi.extend_from_slice(&minor_linker.to_le_bytes());
    efi.extend_from_slice(&code_size_val.to_le_bytes());
    efi.extend_from_slice(&init_data_size.to_le_bytes());
    efi.extend_from_slice(&uninit_data_size.to_le_bytes());
    efi.extend_from_slice(&entry_rva.to_le_bytes());
    efi.extend_from_slice(&code_base.to_le_bytes());
    efi.extend_from_slice(&image_base.to_le_bytes());
    efi.extend_from_slice(&section_align_val.to_le_bytes());
    efi.extend_from_slice(&file_align_val.to_le_bytes());
    efi.extend_from_slice(&major_os.to_le_bytes());
    efi.extend_from_slice(&minor_os.to_le_bytes());
    efi.extend_from_slice(&major_image.to_le_bytes());
    efi.extend_from_slice(&minor_image.to_le_bytes());
    efi.extend_from_slice(&major_subsys.to_le_bytes());
    efi.extend_from_slice(&minor_subsys.to_le_bytes());
    efi.extend_from_slice(&win32_version.to_le_bytes());
    efi.extend_from_slice(&size_of_image_val.to_le_bytes());
    efi.extend_from_slice(&size_of_headers.to_le_bytes());
    efi.extend_from_slice(&check_sum.to_le_bytes());
    efi.extend_from_slice(&subsystem.to_le_bytes());
    efi.extend_from_slice(&dll_characteristics.to_le_bytes());
    efi.extend_from_slice(&size_of_stack_reserve.to_le_bytes());
    efi.extend_from_slice(&size_of_stack_commit.to_le_bytes());
    efi.extend_from_slice(&size_of_heap_reserve.to_le_bytes());
    efi.extend_from_slice(&size_of_heap_commit.to_le_bytes());
    efi.extend_from_slice(&loader_flags.to_le_bytes());
    efi.extend_from_slice(&num_rva_sizes.to_le_bytes());

    // Each data directory entry is 8 bytes: u32 RVA followed by u32 size.
    for _ in 0..16 {
        efi.extend_from_slice(&0u32.to_le_bytes());
        efi.extend_from_slice(&0u32.to_le_bytes());
    }

    // Section table
    let section_name = [b'.', b't', b'e', b'x', b't', 0, 0, 0];
    let virt_size = ((kernel.len() as u32 + sect_align - 1) / sect_align) * sect_align;
    let virt_addr = sect_align;
    let raw_size = code_size;
    let raw_addr = header_size;
    let reloc_addr: u32 = 0;
    let line_nums: u32 = 0;
    let num_relocs: u16 = 0;
    let num_line_nums: u16 = 0;
    let section_flags: u32 = 0x60000020; // IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ

    efi.extend_from_slice(&section_name);
    efi.extend_from_slice(&virt_size.to_le_bytes());
    efi.extend_from_slice(&virt_addr.to_le_bytes());
    efi.extend_from_slice(&raw_size.to_le_bytes());
    efi.extend_from_slice(&raw_addr.to_le_bytes());
    efi.extend_from_slice(&reloc_addr.to_le_bytes());
    efi.extend_from_slice(&line_nums.to_le_bytes());
    efi.extend_from_slice(&num_relocs.to_le_bytes());
    efi.extend_from_slice(&num_line_nums.to_le_bytes());
    efi.extend_from_slice(&section_flags.to_le_bytes());

    // Pad to header size
    while efi.len() < header_size as usize {
        efi.push(0);
    }

    // .text section content
    efi.extend_from_slice(kernel);

    // Pad to file alignment
    while efi.len() < header_size as usize + code_size as usize {
        efi.push(0);
    }

    Ok(efi)
}

// Assembler pipeline
pub struct DirectUefiAssembler;

impl DirectUefiAssembler {
    pub fn assemble(program: &Program) -> Result<Vec<u8>, String> {
        let mut items: Vec<Inst> = Vec::new();
        let entry_name = format!("__event_{}", program.entry_event);

        // UEFI entry point
        // RCX = ImageHandle, RDX = SystemTable
        items.push(Inst::Label("__entry".to_string()));
        // Save SystemTable pointer in a global
        // Use a movabs-style approach: put SystemTable ptr in a known data label
        items.push(Inst::MovReg64Reg64(Reg64::R8, Reg64::Rdx)); // R8 = SystemTable
        // Store SystemTable to global
        items.push(Inst::MovMem64Reg64("__uefi_system_table".to_string(), Reg64::R8));
        // Call entry event
        items.push(Inst::Call(entry_name));
        // Return 0 (success)
        items.push(Inst::XorReg64Reg64(Reg64::Rax, Reg64::Rax));
        items.push(Inst::Ret);

        // UEFI runtime builtins
        items.push(Inst::Label("__uefi_builtins".to_string()));

        items.push(Inst::Label("__builtin_putc".to_string()));
        items.push(Inst::Push(Reg64::Rcx));
        items.push(Inst::Push(Reg64::Rdx));
        items.push(Inst::Push(Reg64::R8));
        items.push(Inst::Push(Reg64::R9));
        items.push(Inst::MovReg64Mem64(Reg64::Rcx, "__uefi_system_table".to_string()));
        items.push(Inst::MovReg64Mem64(Reg64::Rdx, "__builtin_putc_wide".to_string()));
        //TODO: Desimplify
        items.push(Inst::Pop(Reg64::R9));
        items.push(Inst::Pop(Reg64::R8));
        items.push(Inst::Pop(Reg64::Rdx));
        items.push(Inst::Pop(Reg64::Rcx));
        items.push(Inst::Ret);

        // __builtin_clear_screen
        items.push(Inst::Label("__builtin_clear_screen".to_string()));
        items.push(Inst::Ret);

        // __builtin_wait_key
        items.push(Inst::Label("__builtin_wait_key".to_string()));
        items.push(Inst::Ret);

        // Emit blocks
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
            let prefix = match block.kind {
                BlockKind::Event => "__event_",
                BlockKind::Proc => "__proc_",
            };
            items.push(Inst::Label(format!("{}{}", prefix, block.name)));

            for line in &block.lines {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with(';') {
                    continue;
                }
                if trimmed.ends_with(':') {
                    items.push(Inst::Label(format!("{}.{}", block.name, trimmed[..trimmed.len()-1].trim())));
                    continue;
                }
                // Handle emit
                if let Some(target) = trimmed.strip_prefix("emit ") {
                    items.push(Inst::Call(format!("__event_{}", target.trim())));
                    continue;
                }
                // Handle call
                if let Some(target) = trimmed.strip_prefix("call ") {
                    items.push(Inst::Call(format!("__proc_{}", target.trim())));
                    continue;
                }
                // Handle ret
                if trimmed == "ret" {
                    items.push(Inst::Ret);
                    continue;
                }

                // if <reg>
                if let Some(cond_str) = trimmed.strip_prefix("if ") {
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Inst::MovReg64Imm(Reg64::Rax, n));
                            Reg64::Rax
                        }
                        Err(_) => {
                            parse_reg64(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for if", cond_str.trim()))?
                        }
                    };
                    let endif_lbl = format!("__cf_{}_endif", cf_counter);
                    let else_lbl = format!("__cf_{}_else", cf_counter);
                    cf_counter += 1;
                    items.push(Inst::CmpReg64Imm(reg, 0));
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
                if let Some(cond_str) = trimmed.strip_prefix("elif ") {
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
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Inst::MovReg64Imm(Reg64::Rax, n));
                            Reg64::Rax
                        }
                        Err(_) => {
                            parse_reg64(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for elif", cond_str.trim()))?
                        }
                    };
                    items.push(Inst::CmpReg64Imm(reg, 0));
                    let beqz_idx = items.len();
                    items.push(Inst::Je(frame.endif_label.clone()));
                    frame.beqz_indices.push(beqz_idx);
                    continue;
                }

                // else
                if trimmed == "else" {
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
                if trimmed == "endif" {
                    let frame = cf_stack.pop().ok_or("endif without if/while")?;
                    if frame.kind == CfKind::While {
                        return Err("endif without matching if".to_string());
                    }
                    items.push(Inst::Label(frame.endif_label.clone()));
                    continue;
                }

                // while <reg>
                if let Some(cond_str) = trimmed.strip_prefix("while ") {
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Inst::MovReg64Imm(Reg64::Rax, n));
                            Reg64::Rax
                        }
                        Err(_) => {
                            parse_reg64(cond_str.trim()).ok_or_else(|| format!("unknown register '{}' for while", cond_str.trim()))?
                        }
                    };
                    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                    let start_lbl = format!("__cf_{}_start", cf_counter);
                    cf_counter += 1;
                    items.push(Inst::Label(start_lbl));
                    items.push(Inst::CmpReg64Imm(reg, 0));
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
                if trimmed == "endwhile" {
                    let frame = cf_stack.pop().ok_or("endwhile without while")?;
                    if frame.kind != CfKind::While {
                        return Err("endwhile without matching while".to_string());
                    }
                    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                    items.push(Inst::Jmp(start_lbl));
                    items.push(Inst::Label(frame.endif_label.clone()));
                    continue;
                }

                match lower_x64_line(trimmed) {
                    Ok(inst) => items.push(inst),
                    Err(e) => return Err(format!("line '{}': {e}", trimmed)),
                }
            }
        }

        if !cf_stack.is_empty() {
            return Err("unclosed if/while block".to_string());
        }

        // Peephole: NOP compression + dead-code elimination
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

        // Emit data
        items.push(Inst::Label("__data_start".to_string()));

        // UEFI system table pointer storage
        items.push(Inst::Label("__uefi_system_table".to_string()));
        items.push(Inst::Bytes(vec![0u8; 8]));

        // UEFI builtin wide char buffer
        items.push(Inst::Label("__builtin_putc_wide".to_string()));
        items.push(Inst::Bytes(vec![0u8; 4])); // wide char + null

        for decl in &program.data {
            match decl {
                DataDecl::String { name, value } => {
                    items.push(Inst::Label(format!("__data_{}", name)));
                    items.push(Inst::Bytes(expand_string(value)));
                }
                DataDecl::Scalar { name, width, value } => {
                    items.push(Inst::Label(format!("__data_{}", name)));
                    let bytes = match width {
                        ScalarWidth::Byte => vec![*value as u8],
                        ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                        ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(),
                        ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
                    };
                    items.push(Inst::Bytes(bytes));
                }
                DataDecl::Buffer { name, size } => {
                    items.push(Inst::Label(format!("__data_{}", name)));
                    items.push(Inst::Bytes(vec![0u8; *size]));
                }
            }
        }

        // Layout pass
        let label_map = layout_x64_labels(&items)?;
        // Encode pass
        let binary = encode_x64_items(&items, &label_map)?;

        Ok(binary)
    }
}

fn layout_x64_labels(items: &[Inst]) -> Result<BTreeMap<String, u32>, String> {
    let mut labels = BTreeMap::new();
    let mut offset: u32 = 0;
    for item in items {
        match item {
            Inst::Label(name) => {
                if labels.insert(name.clone(), offset).is_some() {
                    return Err(format!("duplicate label '{name}'"));
                }
            }
            Inst::Bytes(bytes) => {
                offset += bytes.len() as u32;
            }
            _ => {
                // Fixed size for common instructions
                offset += match item {
                    Inst::MovReg64Imm(_, _) => 10,  // REX.W B8+rd i64
                    Inst::MovReg64Reg64(_, _) => 3,  // REX.W 89 /r
                    Inst::MovReg32Imm(_, _) => 6,    // REX B8+rd i32
                    Inst::MovReg64Mem64(_, _) | Inst::MovMem64Reg64(_, _) => 7, // REX.W 8B/89 ModRM rel32
                    Inst::LeaReg64Mem(_, _) => 7,    // REX.W 8D ModRM rel32
                    Inst::AddReg64Imm(_, v) | Inst::SubReg64Imm(_, v) => {
                        if *v >= -128 && *v <= 127 { 4 } else { 7 }
                    }
                    Inst::AddReg64Reg64(..) | Inst::SubReg64Reg64(_, _) => 3,
                    Inst::XorReg64Reg64(_, _) | Inst::AndReg64Reg64(_, _) | Inst::OrReg64Reg64(_, _) => 3,
                    Inst::IncReg64(_) | Inst::DecReg64(_) => 3,
                    Inst::MulReg64(_) | Inst::DivReg64(_) => 3,
                    Inst::NotReg64(_) | Inst::NegReg64(_) => 3,
                    Inst::Push(_) | Inst::Pop(_) => 1 + if reg64_num(match item {
                        Inst::Push(r) | Inst::Pop(r) => *r,
                        _ => unreachable!(),
                    }) >= 8 { 1 } else { 0 },
                    Inst::CmpReg64Reg64(_, _) | Inst::TestReg64Reg64(_, _) => 3,
                    Inst::CmpReg64Imm(_, v) => if *v >= -128 && *v <= 127 { 4 } else { 7 },
                    Inst::Jmp(_) | Inst::Call(_) => 5, // E8/E9 rel32
                    Inst::Je(_) | Inst::Jne(_) | Inst::Jl(_) | Inst::Jle(_) |
                    Inst::Jg(_) | Inst::Jge(_) | Inst::Jb(_) | Inst::Jbe(_) |
                    Inst::Ja(_) | Inst::Jae(_) => 6, // 0F 8x rel32
                    Inst::Ret => 1,
                    Inst::Nop => 1,
                    Inst::UefiOutputString | Inst::UefiClearScreen | Inst::UefiWaitForKey => 1, // placeholder
                    _ => 1,
                };
            }
        }
    }
    Ok(labels)
}

fn encode_x64_items(items: &[Inst], label_map: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let mut binary = Vec::new();
    for item in items {
        match item {
            Inst::Label(_) => {}
            Inst::Bytes(bytes) => {
                binary.extend_from_slice(bytes);
            }
            _ => {
                let bytes = encode_x64_inst(item, binary.len(), label_map)?;
                binary.extend_from_slice(&bytes);
            }
        }
    }
    Ok(binary)
}
