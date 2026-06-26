// Lalalalala
// RISC-V compiler backend. Good luck. I know I needed it.
// I provided documentation on this to the best of my ability

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dcrt::{BlockKind, DataDecl, Program, ScalarWidth};

pub struct RiscvBuildOutput {
    pub bin_path: PathBuf,
    pub bin_size: usize,
}

pub struct DirectRiscvBuilder;

impl DirectRiscvBuilder {
    pub fn build_bin(program: &Program, out_path: &Path) -> Result<RiscvBuildOutput, String> {
        if program.target != "riscv" && program.target != "riscv64" && program.target != "riscv_cheri" {
            return Err(format!(
                "direct riscv backend requires target 'riscv' or 'riscv64', got '{}'",
                program.target
            ));
        }
        let kernel = DirectRiscvAssembler::assemble(program)?;
        std::fs::write(out_path, &kernel).map_err(|e| format!("failed to write output: {e}"))?;
        Ok(RiscvBuildOutput {
            bin_path: out_path.to_path_buf(),
            bin_size: kernel.len(),
        })
    }
}

// Registers
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reg {
    X0,  // zero
    X1,  // ra
    X2,  // sp
    X3,  // gp
    X4,  // tp
    X5,  // t0
    X6,  // t1
    X7,  // t2
    X8,  // s0/fp
    X9,  // s1
    X10, // a0
    X11, // a1
    X12, // a2
    X13, // a3
    X14, // a4
    X15, // a5
    X16, // a6
    X17, // a7
    X18, // s2
    X19, // s3
    X20, // s4
    X21, // s5
    X22, // s6
    X23, // s7
    X24, // s8
    X25, // s9
    X26, // s10
    X27, // s11
    X28, // t3
    X29, // t4
    X30, // t5
    X31, // t6
}

fn parse_reg(s: &str) -> Option<Reg> {
    Some(match s {
        "zero" | "x0" => Reg::X0,
        "ra" | "x1" => Reg::X1,
        "sp" | "x2" => Reg::X2,
        "gp" | "x3" => Reg::X3,
        "tp" | "x4" => Reg::X4,
        "t0" | "x5" => Reg::X5,
        "t1" | "x6" => Reg::X6,
        "t2" | "x7" => Reg::X7,
        "s0" | "fp" | "x8" => Reg::X8,
        "s1" | "x9" => Reg::X9,
        "a0" | "x10" => Reg::X10,
        "a1" | "x11" => Reg::X11,
        "a2" | "x12" => Reg::X12,
        "a3" | "x13" => Reg::X13,
        "a4" | "x14" => Reg::X14,
        "a5" | "x15" => Reg::X15,
        "a6" | "x16" => Reg::X16,
        "a7" | "x17" => Reg::X17,
        "s2" | "x18" => Reg::X18,
        "s3" | "x19" => Reg::X19,
        "s4" | "x20" => Reg::X20,
        "s5" | "x21" => Reg::X21,
        "s6" | "x22" => Reg::X22,
        "s7" | "x23" => Reg::X23,
        "s8" | "x24" => Reg::X24,
        "s9" | "x25" => Reg::X25,
        "s10" | "x26" => Reg::X26,
        "s11" | "x27" => Reg::X27,
        "t3" | "x28" => Reg::X28,
        "t4" | "x29" => Reg::X29,
        "t5" | "x30" => Reg::X30,
        "t6" | "x31" => Reg::X31,
        _ => return None,
    })
}

fn reg_num(r: Reg) -> u8 {
    r as u8
}

// 12-bit signed immediate values
#[derive(Debug, Clone)]
struct Imm12(i32);

fn parse_imm12(s: &str) -> Option<Imm12> {
    let val: i32 = s.parse().ok()?;
    if val < -2048 || val > 2047 {
        return None;
    }
    Some(Imm12(val))
}

fn parse_uimm20(s: &str) -> Option<u32> {
    let val: i32 = s.parse().ok()?;
    if val < 0 || val > 0xFFFFF {
        return None;
    }
    Some(val as u32)
}

fn parse_shift(s: &str) -> Option<u32> {
    let val: u32 = s.parse().ok()?;
    if val > 31 {
        return None;
    }
    Some(val)
}

// Instruction set
#[derive(Debug, Clone)]
enum Inst {
    // R-type
    Add(Reg, Reg, Reg),
    Sub(Reg, Reg, Reg),
    Sll(Reg, Reg, Reg),
    Slt(Reg, Reg, Reg),
    Sltu(Reg, Reg, Reg),
    Xor(Reg, Reg, Reg),
    Srl(Reg, Reg, Reg),
    Sra(Reg, Reg, Reg),
    Or(Reg, Reg, Reg),
    And(Reg, Reg, Reg),
    // R-type with funct7=1 (RV32M)
    Mul(Reg, Reg, Reg),
    Div(Reg, Reg, Reg),
    Rem(Reg, Reg, Reg),
    // I-type
    Addi(Reg, Reg, Imm12),
    Slti(Reg, Reg, Imm12),
    Sltiu(Reg, Reg, Imm12),
    Xori(Reg, Reg, Imm12),
    Ori(Reg, Reg, Imm12),
    Andi(Reg, Reg, Imm12),
    Slli(Reg, Reg, u32),
    Srli(Reg, Reg, u32),
    Srai(Reg, Reg, u32),
    // Load (I-type)
    Lb(Reg, Reg, Imm12),
    Lh(Reg, Reg, Imm12),
    Lw(Reg, Reg, Imm12),
    Lbu(Reg, Reg, Imm12),
    Lhu(Reg, Reg, Imm12),
    // Store (S-type)
    Sb(Reg, Reg, Imm12),
    Sh(Reg, Reg, Imm12),
    Sw(Reg, Reg, Imm12),
    // U-type
    Lui(Reg, u32),
    Auipc(Reg, u32),
    // Branch (B-type)
    Beq(Reg, Reg, String),
    Bne(Reg, Reg, String),
    Blt(Reg, Reg, String),
    Bge(Reg, Reg, String),
    Bltu(Reg, Reg, String),
    Bgeu(Reg, Reg, String),
    // Jump (J-type / I-type)
    Jal(Reg, String),
    Jalr(Reg, Reg, Imm12),
    // Special
    Ecall,
    Ebreak,
    Nop,
    // Pseudo-instructions
    Li(Reg, i64),
    Mv(Reg, Reg),
    Neg(Reg, Reg),
    Not(Reg, Reg),
    Seqz(Reg, Reg),
    Snez(Reg, Reg),
    // Labels and raw data bytes
    Label(String),
    Bytes(Vec<u8>),
}

fn encode_r_type(funct7: u8, rs2: u8, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    ((funct7 as u32) << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | ((funct3 as u32) << 12)
        | ((rd as u32) << 7)
        | (opcode as u32)
}

fn encode_i_type(imm: u32, rs1: u8, funct3: u8, rd: u8, opcode: u8) -> u32 {
    ((imm & 0xFFF) << 20) | ((rs1 as u32) << 15) | ((funct3 as u32) << 12)
        | ((rd as u32) << 7)
        | (opcode as u32)
}

fn encode_s_type(imm: u32, rs2: u8, rs1: u8, funct3: u8, opcode: u8) -> u32 {
    ((imm & 0xFE0) << 20)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | ((funct3 as u32) << 12)
        | ((imm & 0x1F) << 7)
        | (opcode as u32)
}

fn encode_b_type(imm: u32, rs2: u8, rs1: u8, funct3: u8, opcode: u8) -> u32 {
    let b11 = (imm >> 11) & 1;
    let b10_5 = (imm >> 5) & 0x3F;
    let b4_1 = (imm >> 1) & 0xF;
    let b12 = (imm >> 12) & 1;
    ((b12 as u32) << 31)
        | ((b10_5 as u32) << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | ((funct3 as u32) << 12)
        | ((b4_1 as u32) << 8)
        | ((b11 as u32) << 7)
        | (opcode as u32)
}

fn encode_u_type(imm: u32, rd: u8, opcode: u8) -> u32 {
    ((imm & 0xFFFFF) << 12) | ((rd as u32) << 7) | (opcode as u32)
}

fn encode_j_type(imm: u32, rd: u8, opcode: u8) -> u32 {
    let b20 = (imm >> 20) & 1;
    let b10_1 = (imm >> 1) & 0x3FF;
    let b11 = (imm >> 11) & 1;
    let b19_12 = (imm >> 12) & 0xFF;
    ((b20 as u32) << 31)
        | ((b19_12 as u32) << 12)
        | ((b11 as u32) << 20)
        | ((b10_1 as u32) << 21)
        | ((rd as u32) << 7)
        | (opcode as u32)
}

fn encode_inst(inst: &Inst, offset: usize, label_map: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let off = offset as u32;
    let word = match inst {
        // R-type (opcode=0x33)
        Inst::Add(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x0, reg_num(*rd), 0x33),
        Inst::Sub(rd, rs1, rs2) => encode_r_type(0x20, reg_num(*rs2), reg_num(*rs1), 0x0, reg_num(*rd), 0x33),
        Inst::Sll(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x1, reg_num(*rd), 0x33),
        Inst::Slt(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x2, reg_num(*rd), 0x33),
        Inst::Sltu(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x3, reg_num(*rd), 0x33),
        Inst::Xor(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x4, reg_num(*rd), 0x33),
        Inst::Srl(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x5, reg_num(*rd), 0x33),
        Inst::Sra(rd, rs1, rs2) => encode_r_type(0x20, reg_num(*rs2), reg_num(*rs1), 0x5, reg_num(*rd), 0x33),
        Inst::Or(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x6, reg_num(*rd), 0x33),
        Inst::And(rd, rs1, rs2) => encode_r_type(0x00, reg_num(*rs2), reg_num(*rs1), 0x7, reg_num(*rd), 0x33),
        // RV32M extension (funct7=0x01)
        Inst::Mul(rd, rs1, rs2) => encode_r_type(0x01, reg_num(*rs2), reg_num(*rs1), 0x0, reg_num(*rd), 0x33),
        Inst::Div(rd, rs1, rs2) => encode_r_type(0x01, reg_num(*rs2), reg_num(*rs1), 0x4, reg_num(*rd), 0x33),
        Inst::Rem(rd, rs1, rs2) => encode_r_type(0x01, reg_num(*rs2), reg_num(*rs1), 0x6, reg_num(*rd), 0x33),
        // I-type arithmetic (opcode=0x13)
        Inst::Addi(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x0, reg_num(*rd), 0x13),
        Inst::Slti(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x2, reg_num(*rd), 0x13),
        Inst::Sltiu(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x3, reg_num(*rd), 0x13),
        Inst::Xori(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x4, reg_num(*rd), 0x13),
        Inst::Ori(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x6, reg_num(*rd), 0x13),
        Inst::Andi(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x7, reg_num(*rd), 0x13),
        Inst::Slli(rd, rs1, shamt) => encode_i_type(*shamt, reg_num(*rs1), 0x1, reg_num(*rd), 0x13),
        Inst::Srli(rd, rs1, shamt) => encode_i_type(*shamt, reg_num(*rs1), 0x5, reg_num(*rd), 0x13),
        Inst::Srai(rd, rs1, shamt) => encode_i_type(*shamt | 0x400, reg_num(*rs1), 0x5, reg_num(*rd), 0x13),
        // Load (opcode=0x03)
        Inst::Lb(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x0, reg_num(*rd), 0x03),
        Inst::Lh(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x1, reg_num(*rd), 0x03),
        Inst::Lw(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x2, reg_num(*rd), 0x03),
        Inst::Lbu(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x4, reg_num(*rd), 0x03),
        Inst::Lhu(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x5, reg_num(*rd), 0x03),
        // Store (opcode=0x23)
        Inst::Sb(rs2, rs1, imm) => encode_s_type(imm.0 as u32, reg_num(*rs2), reg_num(*rs1), 0x0, 0x23),
        Inst::Sh(rs2, rs1, imm) => encode_s_type(imm.0 as u32, reg_num(*rs2), reg_num(*rs1), 0x1, 0x23),
        Inst::Sw(rs2, rs1, imm) => encode_s_type(imm.0 as u32, reg_num(*rs2), reg_num(*rs1), 0x2, 0x23),
        // U-type
        Inst::Lui(rd, imm) => encode_u_type(*imm, reg_num(*rd), 0x37),
        Inst::Auipc(rd, imm) => encode_u_type(*imm, reg_num(*rd), 0x17),
        // Branch (opcode=0x63)
        Inst::Beq(_r1, _r2, _) | Inst::Bne(_r1, _r2, _) | Inst::Blt(_r1, _r2, _)
        | Inst::Bge(_r1, _r2, _) | Inst::Bltu(_r1, _r2, _) | Inst::Bgeu(_r1, _r2, _) => {
            let (rs1, rs2, label, funct3) = match inst {
                Inst::Beq(rs1, rs2, l) => (rs1, rs2, l, 0x0),
                Inst::Bne(rs1, rs2, l) => (rs1, rs2, l, 0x1),
                Inst::Blt(rs1, rs2, l) => (rs1, rs2, l, 0x4),
                Inst::Bge(rs1, rs2, l) => (rs1, rs2, l, 0x5),
                Inst::Bltu(rs1, rs2, l) => (rs1, rs2, l, 0x6),
                Inst::Bgeu(rs1, rs2, l) => (rs1, rs2, l, 0x7),
                _ => unreachable!(),
            };
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = if target >= off { target - off } else { 0 }; // forward only for simplicity
            let rel = rel as i32; // offset might be negative
            if rel < -4096 || rel > 4095 {
                return Err(format!("branch offset out of range for '{label}': {rel}"));
            }
            encode_b_type(rel as u32 & 0x1FFF, reg_num(*rs2), reg_num(*rs1), funct3, 0x63)
        }
        // Jump (opcode=0x6F)
        Inst::Jal(rd, label) => {
            let target = *label_map.get(label).ok_or_else(|| format!("unknown label '{label}'"))?;
            let rel = if target >= off { target - off } else { 0 };
            let rel = rel as i32;
            if rel < -1048576 || rel > 1048575 {
                return Err(format!("jal offset out of range for '{label}': {rel}"));
            }
            encode_j_type(rel as u32 & 0x1FFFFF, reg_num(*rd), 0x6F)
        }
        // JALR (opcode=0x67)
        Inst::Jalr(rd, rs1, imm) => encode_i_type(imm.0 as u32, reg_num(*rs1), 0x0, reg_num(*rd), 0x67),
        // Special
        Inst::Ecall => encode_i_type(0, 0, 0, 0, 0x73),
        Inst::Ebreak => encode_i_type(1, 0, 0, 0, 0x73),
        Inst::Nop => encode_i_type(0, 0, 0, 0, 0x13), // addi x0, x0, 0
        // Pseudo: li rd, imm -> expands to lui+addi for 32-bit immediates
        Inst::Li(rd, val) => {
            if *val >= -2048 && *val <= 2047 {
                encode_i_type((*val & 0xFFF) as u32, 0, 0, reg_num(*rd), 0x13) // addi rd, x0, imm
            } else {
                let _upper = ((*val >> 12) + if (*val & 0x800) != 0 { 1 } else { 0 }) as u32 & 0xFFFFF;
                let _lower = (*val & 0xFFF) as u32;
                // Generate two instructions: lui rd, upper; addi rd, rd, lower
                panic!("Li should not reach encoding directly; expand in lower_line");
            }
        }
        Inst::Mv(rd, rs) => encode_r_type(0x00, reg_num(*rs), 0, 0, reg_num(*rd), 0x33), // add rd, x0, rs
        Inst::Neg(rd, rs) => encode_r_type(0x20, 0, reg_num(*rs), 0x0, reg_num(*rd), 0x33), // sub rd, x0, rs
        Inst::Not(rd, rs) => encode_i_type(0xFFF, reg_num(*rs), 0x4, reg_num(*rd), 0x13), // xori rd, rs, -1
        Inst::Seqz(rd, rs) => encode_i_type(1, reg_num(*rs), 0x3, reg_num(*rd), 0x13), // sltiu rd, rs, 1
        Inst::Snez(rd, rs) => encode_r_type(0x00, 0, reg_num(*rs), 0x3, reg_num(*rd), 0x33), // sltu rd, x0, rs
        _ => return Err(format!("instruction cannot be encoded directly")),
    };
    Ok(word.to_le_bytes().to_vec())
}

fn write_string_null(s: &str) -> Vec<u8> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

fn lower_line(line: &str, _data_labels: &BTreeMap<String, Vec<u8>>) -> Result<Inst, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("empty line".to_string());
    }

    if trimmed.ends_with(':') {
        return Ok(Inst::Label(trimmed[..trimmed.len()-1].to_string()));
    }

    if trimmed.starts_with(';') {
        return Err("comment line".to_string());
    }

    let parts: Vec<&str> = trimmed.splitn(4, |c: char| c == ' ' || c == '\t')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return Err("empty instruction line".to_string());
    }

    let mnemonic = parts[0];
    let args_str = if parts.len() > 1 { parts[1..].join(" ") } else { String::new() };

    match mnemonic {
        "add" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Add(rd, rs1, rs2))
        }
        "sub" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Sub(rd, rs1, rs2))
        }
        "sll" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Sll(rd, rs1, rs2))
        }
        "slt" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Slt(rd, rs1, rs2))
        }
        "sltu" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Sltu(rd, rs1, rs2))
        }
        "xor" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Xor(rd, rs1, rs2))
        }
        "srl" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Srl(rd, rs1, rs2))
        }
        "sra" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Sra(rd, rs1, rs2))
        }
        "or" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Or(rd, rs1, rs2))
        }
        "and" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::And(rd, rs1, rs2))
        }
        "mul" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Mul(rd, rs1, rs2))
        }
        "div" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Div(rd, rs1, rs2))
        }
        "rem" => {
            let (rd, rs1, rs2) = parse_three_reg(&args_str)?;
            Ok(Inst::Rem(rd, rs1, rs2))
        }
        "addi" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Addi(rd, rs1, imm))
        }
        "slti" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Slti(rd, rs1, imm))
        }
        "sltiu" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Sltiu(rd, rs1, imm))
        }
        "xori" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Xori(rd, rs1, imm))
        }
        "ori" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Ori(rd, rs1, imm))
        }
        "andi" => {
            let (rd, rs1, imm) = parse_two_reg_imm(&args_str)?;
            Ok(Inst::Andi(rd, rs1, imm))
        }
        "slli" => {
            let (rd, rs1, shamt) = parse_two_reg_shift(&args_str)?;
            Ok(Inst::Slli(rd, rs1, shamt))
        }
        "srli" => {
            let (rd, rs1, shamt) = parse_two_reg_shift(&args_str)?;
            Ok(Inst::Srli(rd, rs1, shamt))
        }
        "srai" => {
            let (rd, rs1, shamt) = parse_two_reg_shift(&args_str)?;
            Ok(Inst::Srai(rd, rs1, shamt))
        }
        "lui" => {
            let (rd, imm) = parse_reg_imm20(&args_str)?;
            Ok(Inst::Lui(rd, imm))
        }
        "auipc" => {
            let (rd, imm) = parse_reg_imm20(&args_str)?;
            Ok(Inst::Auipc(rd, imm))
        }
        "lb" => parse_load_store("lb", &args_str, |rd, rs1, imm| Inst::Lb(rd, rs1, imm)),
        "lh" => parse_load_store("lh", &args_str, |rd, rs1, imm| Inst::Lh(rd, rs1, imm)),
        "lw" => parse_load_store("lw", &args_str, |rd, rs1, imm| Inst::Lw(rd, rs1, imm)),
        "lbu" => parse_load_store("lbu", &args_str, |rd, rs1, imm| Inst::Lbu(rd, rs1, imm)),
        "lhu" => parse_load_store("lhu", &args_str, |rd, rs1, imm| Inst::Lhu(rd, rs1, imm)),
        "sb" => parse_store("sb", &args_str, |rs2, rs1, imm| Inst::Sb(rs2, rs1, imm)),
        "sh" => parse_store("sh", &args_str, |rs2, rs1, imm| Inst::Sh(rs2, rs1, imm)),
        "sw" => parse_store("sw", &args_str, |rs2, rs1, imm| Inst::Sw(rs2, rs1, imm)),
        "beq" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Beq(rs1, rs2, label))
        }
        "bne" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Bne(rs1, rs2, label))
        }
        "blt" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Blt(rs1, rs2, label))
        }
        "bge" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Bge(rs1, rs2, label))
        }
        "bltu" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Bltu(rs1, rs2, label))
        }
        "bgeu" => {
            let (rs1, rs2, label) = parse_two_reg_label(&args_str)?;
            Ok(Inst::Bgeu(rs1, rs2, label))
        }
        "beqz" => {
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 2 {
                let rs = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                Ok(Inst::Beq(rs, Reg::X0, args[1].to_string()))
            } else {
                Err("beqz: expected reg, label".to_string())
            }
        }
        "bnez" => {
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 2 {
                let rs = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                Ok(Inst::Bne(rs, Reg::X0, args[1].to_string()))
            } else {
                Err("bnez: expected reg, label".to_string())
            }
        }
        "jal" => {
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 1 {
                Ok(Inst::Jal(Reg::X1, args[0].to_string())) // default: jal ra, label
            } else if args.len() == 2 {
                let rd = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                Ok(Inst::Jal(rd, args[1].to_string()))
            } else {
                Err(format!("bad jal syntax: {args_str}"))
            }
        }
        "jalr" => {
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 3 {
                let rd = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                let rs1 = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                let imm = parse_imm12(args[2]).ok_or_else(|| format!("bad immediate '{}'", args[2]))?;
                Ok(Inst::Jalr(rd, rs1, imm))
            } else {
                Err(format!("bad jalr syntax: {args_str}"))
            }
        }
        "ecall" => Ok(Inst::Ecall),
        "ebreak" => Ok(Inst::Ebreak),
        "nop" => Ok(Inst::Nop),
        "li" | "mv" | "neg" | "not" | "seqz" | "snez" => {
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if mnemonic == "li" && args.len() == 2 {
                let rd = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                let val: i64 = args[1].parse().map_err(|_| format!("bad immediate '{}'", args[1]))?;
                Ok(Inst::Li(rd, val))
            } else if (mnemonic == "mv" || mnemonic == "neg" || mnemonic == "not" || mnemonic == "seqz" || mnemonic == "snez") && args.len() == 2 {
                let rd = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
                let rs = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
                match mnemonic {
                    "mv" => Ok(Inst::Mv(rd, rs)),
                    "neg" => Ok(Inst::Neg(rd, rs)),
                    "not" => Ok(Inst::Not(rd, rs)),
                    "seqz" => Ok(Inst::Seqz(rd, rs)),
                    "snez" => Ok(Inst::Snez(rd, rs)),
                    _ => unreachable!(),
                }
            } else {
                Err(format!("bad {mnemonic} syntax: {args_str}"))
            }
        }
        "j" => {
            // Unconditional jump: j label -> jal x0, label
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 1 {
                Ok(Inst::Jal(Reg::X0, args[0].to_string()))
            } else {
                Err(format!("bad j syntax: {args_str}"))
            }
        }
        "ret" => {
            // ret -> jalr x0, x1, 0
            Ok(Inst::Jalr(Reg::X0, Reg::X1, Imm12(0)))
        }
        "call" => {
            // call label -> jal ra, label (but needs pseudo-instruction expansion)
            let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            if args.len() == 1 {
                Ok(Inst::Jal(Reg::X1, args[0].to_string()))
            } else {
                Err(format!("bad call syntax: {args_str}"))
            }
        }
        _ => Err(format!("unknown riscv instruction '{mnemonic}'")),
    }
}

fn parse_three_reg(s: &str) -> Result<(Reg, Reg, Reg), String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 3 {
        return Err(format!("expected 3 register arguments, got {}", args.len()));
    }
    let r1 = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let r2 = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
    let r3 = parse_reg(args[2]).ok_or_else(|| format!("unknown register '{}'", args[2]))?;
    Ok((r1, r2, r3))
}

fn parse_two_reg_imm(s: &str) -> Result<(Reg, Reg, Imm12), String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 3 {
        return Err(format!("expected 2 registers + immediate, got {}", args.len()));
    }
    let r1 = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let r2 = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
    let imm = parse_imm12(args[2]).ok_or_else(|| format!("bad immediate '{}'", args[2]))?;
    Ok((r1, r2, imm))
}

fn parse_two_reg_shift(s: &str) -> Result<(Reg, Reg, u32), String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 3 {
        return Err(format!("expected 2 registers + shift amount, got {}", args.len()));
    }
    let r1 = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let r2 = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
    let shamt = parse_shift(args[2]).ok_or_else(|| format!("bad shift amount '{}'", args[2]))?;
    Ok((r1, r2, shamt))
}

fn parse_reg_imm20(s: &str) -> Result<(Reg, u32), String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 2 {
        return Err(format!("expected register + immediate, got {}", args.len()));
    }
    let r = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let imm = parse_uimm20(args[1]).ok_or_else(|| format!("bad 20-bit immediate '{}'", args[1]))?;
    Ok((r, imm))
}

fn parse_two_reg_label(s: &str) -> Result<(Reg, Reg, String), String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 3 {
        return Err(format!("expected 2 registers + label, got {}", args.len()));
    }
    let r1 = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    let r2 = parse_reg(args[1]).ok_or_else(|| format!("unknown register '{}'", args[1]))?;
    Ok((r1, r2, args[2].to_string()))
}

fn parse_load_store(mnemonic: &str, s: &str, f: impl FnOnce(Reg, Reg, Imm12) -> Inst) -> Result<Inst, String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 2 {
        return Err(format!("bad {mnemonic} syntax: {s}"));
    }
    let rd = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    // Parse "offset(rs1)"
    let mem = args[1];
    if let Some(rest) = mem.strip_suffix(')') {
        if let Some(idx) = rest.find('(') {
            let offset_str = &rest[..idx];
            let reg_str = &rest[idx+1..];
            let imm = if offset_str.is_empty() || offset_str == "0" {
                Imm12(0)
            } else {
                parse_imm12(offset_str).ok_or_else(|| format!("bad offset '{}'", offset_str))?
            };
            let rs1 = parse_reg(reg_str).ok_or_else(|| format!("unknown register '{}'", reg_str))?;
            return Ok(f(rd, rs1, imm));
        }
    }
    Err(format!("bad memory operand '{}'", args[1]))
}

fn parse_store(mnemonic: &str, s: &str, f: impl FnOnce(Reg, Reg, Imm12) -> Inst) -> Result<Inst, String> {
    let args: Vec<&str> = s.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if args.len() != 2 {
        return Err(format!("bad {mnemonic} syntax: {s}"));
    }
    let rs2 = parse_reg(args[0]).ok_or_else(|| format!("unknown register '{}'", args[0]))?;
    // Parse "offset(rs1)"
    let mem = args[1];
    if let Some(rest) = mem.strip_suffix(')') {
        if let Some(idx) = rest.find('(') {
            let offset_str = &rest[..idx];
            let reg_str = &rest[idx+1..];
            let imm = if offset_str.is_empty() || offset_str == "0" {
                Imm12(0)
            } else {
                parse_imm12(offset_str).ok_or_else(|| format!("bad offset '{}'", offset_str))?
            };
            let rs1 = parse_reg(reg_str).ok_or_else(|| format!("unknown register '{}'", reg_str))?;
            return Ok(f(rs2, rs1, imm));
        }
    }
    Err(format!("bad memory operand '{}'", args[1]))
}

// Assembler pipeline
struct DirectRiscvAssembler;

impl DirectRiscvAssembler {
    fn assemble(program: &Program) -> Result<Vec<u8>, String> {
        let mut items: Vec<Inst> = Vec::new();

        // Emit data declarations
        let data_labels: BTreeMap<String, Vec<u8>> = BTreeMap::new();

        // _start entry point
        let entry_name = format!("__event_{}", program.entry_event);
        items.push(Inst::Label("_start".to_string()));
        // Initialize stack pointer
        items.push(Inst::Li(Reg::X2, 0x8000_1000)); // sp = top of RAM
        // Call entry event
        items.push(Inst::Jal(Reg::X1, entry_name.clone()));
        // Halt: try OpenSBI SBI SRST shutdown, then sifive_test, then ecall, then loop
        items.push(Inst::Label("_halt".to_string()));
        // SBI SRST extension (OpenSBI): a6=0 shutdown, a7=SRST EID
        items.push(Inst::Li(Reg::X10, 0));             // a0 = shutdown type
        items.push(Inst::Li(Reg::X11, 0));             // a1 = reset reason
        items.push(Inst::Li(Reg::X16, 0));             // a6 = function ID (shutdown)
        items.push(Inst::Li(Reg::X17, 0x53525354));    // a7 = SRST extension ID
        items.push(Inst::Ecall);
        // SiFive test device (bare-metal QEMU virt at 0x100000)
        items.push(Inst::Li(Reg::X10, 0x5555));        // a0 = magic shutdown value
        items.push(Inst::Li(Reg::X11, 0x100000));       // a1 = sifive_test base address
        items.push(Inst::Sw(Reg::X10, Reg::X11, Imm12(0))); // sw a0, 0(a1)
        items.push(Inst::Ecall);
        items.push(Inst::Jal(Reg::X0, "_halt".to_string()));

        // Emit blocks
        // Control flow construct stack (if/else/endif/while/endwhile)
        struct CfFrame {
            kind: CfKind,
            endif_label: String,
            else_label: String,
            beqz_indices: Vec<usize>,  // indices of all beqz instructions (if + each elif)
            has_else: bool,
        }
        #[derive(PartialEq)]
        enum CfKind { If, While }
        let mut cf_stack: Vec<CfFrame> = Vec::new();
        let mut cf_counter: u32 = 0;

        for block in &program.blocks {
            let label = format!("__{}", match block.kind {
                BlockKind::Event => "event",
                BlockKind::Proc => "proc",
            });
            let block_label = format!("{label}_{}", block.name);
            items.push(Inst::Label(block_label));

            for line in &block.lines {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with(';') {
                    continue;
                }
                if trimmed.ends_with(':') {
                    items.push(Inst::Label(format!("{}.{}", block.name, trimmed[..trimmed.len()-1].trim())));
                    continue;
                }
                if let Some(target) = trimmed.strip_prefix("emit ") {
                    let event_name = target.trim().to_string();
                    items.push(Inst::Jal(Reg::X1, format!("__event_{}", event_name)));
                    continue;
                }
                if let Some(target) = trimmed.strip_prefix("call ") {
                    let proc_name = target.trim().to_string();
                    items.push(Inst::Jal(Reg::X1, format!("__proc_{}", proc_name)));
                    continue;
                }
                if trimmed == "ret" {
                    items.push(Inst::Jalr(Reg::X0, Reg::X1, Imm12(0)));
                    continue;
                }

                // if <reg>
                if let Some(cond_str) = trimmed.strip_prefix("if ") {
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Inst::Li(Reg::X10, n));
                            Reg::X10
                        }
                        Err(_) => {
                            let reg_str = cond_str.trim();
                            parse_reg(reg_str).ok_or_else(|| format!("unknown register '{}' for if", reg_str))?
                        }
                    };
                    let endif_lbl = format!("__cf_{}_endif", cf_counter);
                    let else_lbl = format!("__cf_{}_else", cf_counter);
                    cf_counter += 1;
                    let beqz_idx = items.len();
                    items.push(Inst::Beq(reg, Reg::X0, endif_lbl.clone()));
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
                    // Patch the previous beqz to jump to this elif label
                    let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                    if let Inst::Beq(_, _, ref mut label) = items[*prev] {
                        *label = elif_lbl.clone();
                    }
                    // jmp past the elif chain to endif
                    items.push(Inst::Jal(Reg::X0, frame.endif_label.clone()));
                    // Emit the elif label
                    items.push(Inst::Label(elif_lbl));
                    // Load/parse condition for this elif
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Inst::Li(Reg::X10, n));
                            Reg::X10
                        }
                        Err(_) => {
                            let reg_str = cond_str.trim();
                            parse_reg(reg_str).ok_or_else(|| format!("unknown register '{}' for elif", reg_str))?
                        }
                    };
                    let beqz_idx = items.len();
                    items.push(Inst::Beq(reg, Reg::X0, frame.endif_label.clone()));
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
                    if let Inst::Beq(_, _, ref mut label) = items[*prev] {
                        *label = frame.else_label.clone();
                    }
                    items.push(Inst::Jal(Reg::X0, frame.endif_label.clone()));
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
                            items.push(Inst::Li(Reg::X10, n));
                            Reg::X10
                        }
                        Err(_) => {
                            let reg_str = cond_str.trim();
                            parse_reg(reg_str).ok_or_else(|| format!("unknown register '{}' for while", reg_str))?
                        }
                    };
                    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                    let start_lbl = format!("__cf_{}_start", cf_counter);
                    cf_counter += 1;
                    items.push(Inst::Label(start_lbl));
                    let beqz_idx = items.len();
                    items.push(Inst::Beq(reg, Reg::X0, endwhile_lbl.clone()));
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
                    items.push(Inst::Jal(Reg::X0, start_lbl));
                    items.push(Inst::Label(frame.endif_label.clone()));
                    continue;
                }

                match lower_line(trimmed, &data_labels) {
                    Ok(inst) => items.push(inst),
                    Err(e) => return Err(format!("line '{}': {e}", trimmed)),
                }
            }
        }

        if !cf_stack.is_empty() {
            return Err(format!("unclosed if/while block"));
        }
        // (first assembly loop above; second loop removed - single pass)

        // Peephole: NOP compression + dead-code elimination
        crate::direct_peephole::peephole(&mut items,
            |i| matches!(i, Inst::Nop),
            |i| matches!(i, Inst::Jal(..)|Inst::Jalr(..)|Inst::Beq(..)|Inst::Bne(..)|Inst::Blt(..)|Inst::Bge(..)|Inst::Bltu(..)|Inst::Bgeu(..)),
            |i| matches!(i, Inst::Ecall|Inst::Ebreak),
            |i| matches!(i, Inst::Label(_)),
        );

        // Emit data
        items.push(Inst::Label("__data_start".to_string()));
        for decl in &program.data {
            match decl {
                DataDecl::String { name, value } => {
                    items.push(Inst::Label(format!("__data_{}", name)));
                    // Expand escape sequences
                    let expanded = expand_string(value);
                    items.push(Inst::Bytes(expanded));
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

        // Expand pseudo instructions (Li with >12-bit immediates)
        let expanded = expand_pseudo(&items)?;
        // Layout pass - resolve labels
        let label_map = layout_labels(&expanded)?;
        // Encode pass
        let binary = encode_items(&expanded, &label_map)?;

        Ok(binary)
    }
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
                other => {
                    bytes.push(b'\\');
                    bytes.push(other as u8);
                }
            }
            i += 2;
        } else {
            bytes.push(chars[i] as u8);
            i += 1;
        }
    }
    bytes.push(0); // null terminate
    bytes
}

fn expand_pseudo(items: &[Inst]) -> Result<Vec<Inst>, String> {
    let mut result = Vec::new();
    for item in items {
        match item {
            Inst::Li(rd, val) => {
                if *val >= -2048 && *val <= 2047 {
                    result.push(Inst::Addi(*rd, Reg::X0, Imm12(*val as i32)));
                } else {
                    // lui rd, upper; addi rd, rd, lower
                    let lower = (*val & 0xFFF) as i32;
                    let upper_val = if lower >= 2048 { (*val >> 12) + 1 } else { *val >> 12 };
                    let upper = upper_val as u32 & 0xFFFFF;
                    let lower_adj = if lower >= 2048 { lower - 4096 } else { lower };
                    result.push(Inst::Lui(*rd, upper));
                    if lower_adj != 0 {
                        result.push(Inst::Addi(*rd, *rd, Imm12(lower_adj)));
                    }
                }
            }
            _ => result.push(item.clone()),
        }
    }
    Ok(result)
}

fn layout_labels(items: &[Inst]) -> Result<BTreeMap<String, u32>, String> {
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
                // Align to 4 bytes for RV32I
                let align = (4 - (offset % 4)) % 4;
                offset += align;
                offset += bytes.len() as u32;
            }
            _ => {
                offset += 4; // all instructions are 4 bytes
            }
        }
    }
    Ok(labels)
}

fn encode_items(items: &[Inst], label_map: &BTreeMap<String, u32>) -> Result<Vec<u8>, String> {
    let mut binary = Vec::new();
    for item in items {
        match item {
            Inst::Label(_) => {}
            Inst::Bytes(bytes) => {
                // Align to 4 bytes
                let align = (4 - (binary.len() % 4)) % 4;
                for _ in 0..align {
                    binary.push(0);
                }
                binary.extend_from_slice(bytes);
            }
            _ => {
                let bytes = encode_inst(item, binary.len(), label_map)?;
                binary.extend_from_slice(&bytes);
            }
        }
    }
    Ok(binary)
}
