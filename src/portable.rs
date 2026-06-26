// No. Just no.
// One of the 3 first original targets that existed in Decretum, used to be named win64.
// Its a amalgamate of things that should not be here, and that are here, and I might one day clean this up.
// Also needs to be commented better throughout

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dcrt::{
    Block, BlockKind, DataDecl, Program, is_ident, parse_integer, parse_quoted_string,
};

const DCB_MAGIC: &[u8; 4] = b"DCB2";
const DCB_VERSION: u16 = 2;

#[derive(Debug, Clone)]
pub struct BytecodeBuildOutput {
    pub bytecode_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PortablePeOutput {
    pub bytecode_path: PathBuf,
    pub pe_path: PathBuf,
    pub project_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BytecodeEvent {
    pub name: String,
}

pub struct PortableBuilder;

impl PortableBuilder {
    pub fn compile_to_bytes(program: &Program) -> Result<Vec<u8>, String> {
        let bytecode = compile_program(program)?;
        encode_program(&bytecode)
    }

    pub fn build_bytecode(
        program: &Program,
        out_path: &Path,
    ) -> Result<BytecodeBuildOutput, String> {
        let bytes = Self::compile_to_bytes(program)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        std::fs::write(out_path, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
        Ok(BytecodeBuildOutput {
            bytecode_path: out_path.to_path_buf(),
        })
    }

    pub fn build_pe(program: &Program, out_path: &Path) -> Result<PortablePeOutput, String> {
        let bytes = Self::compile_to_bytes(program)?;
        let bytecode_path = out_path.with_extension("dcb");
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        std::fs::write(&bytecode_path, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", bytecode_path.display()))?;

        let current_exe =
            std::env::current_exe().map_err(|e| format!("failed to get current exe: {e}"))?;

        let mut out_exe = out_path.to_path_buf();
        let ext = std::env::consts::EXE_EXTENSION;
        if !ext.is_empty() {
            out_exe.set_extension(ext);
        }

        let host_exe = if program_uses_compile_decretum(program) {
            resolve_full_runtime_exe(&current_exe)?
        } else {
            resolve_runtime_stub_exe(&current_exe)?
        };
        let mut exe_bytes =
            std::fs::read(&host_exe).map_err(|e| format!("failed to read host exe: {e}"))?;

        // Strip any previously appended payload by checking tail magic
        let magic = dcrt_embed_magic();
        if exe_bytes.len() >= 20 && exe_bytes[exe_bytes.len() - 16..] == magic {
            let len_off = exe_bytes.len() - 20;
            let bc_len = u32::from_le_bytes([
                exe_bytes[len_off],
                exe_bytes[len_off + 1],
                exe_bytes[len_off + 2],
                exe_bytes[len_off + 3],
            ]) as usize;
            // Truncate to remove [bytecode][len][magic]
            exe_bytes.truncate(len_off.saturating_sub(bc_len));
        }

        // Append: [bytecode bytes] [u32 bytecode length LE] [16-byte magic]
        exe_bytes.extend_from_slice(&bytes);
        exe_bytes.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        exe_bytes.extend_from_slice(&magic);

        std::fs::write(&out_exe, &exe_bytes).map_err(|e| format!("failed to write pe: {e}"))?;

        Ok(PortablePeOutput {
            bytecode_path,
            pe_path: out_exe,
            project_dir: out_path.parent().unwrap().to_path_buf(),
        })
    }
}

#[derive(Debug, Clone)]
struct BytecodeProgram {
    entry_event: String,
    int_data: BTreeMap<String, i64>,
    string_data: BTreeMap<String, String>,
    blocks: Vec<BytecodeBlock>,
}

#[derive(Debug, Clone)]
struct BytecodeBlock {
    kind: BlockKind,
    name: String,
    ops: Vec<Op>,
}

#[derive(Debug, Clone)]
enum Op {
    Mov(Dest, Operand),
    Add(Dest, Operand),
    Sub(Dest, Operand),
    Mul(Dest, Operand),
    Div(Dest, Operand),
    Mod(Dest, Operand),
    And(Dest, Operand),
    Or(Dest, Operand),
    Xor(Dest, Operand),
    Shl(Dest, Operand),
    Shr(Dest, Operand),
    Not(Dest),
    Inc(Dest),
    Dec(Dest),
    Cmp(Operand, Operand),
    Jump(u32),
    JumpIf(Cond, u32),
    Emit(String),
    CallProc(String),
    CallLabel(u32),
    Ret,
    PrintText(String),
    PrintData(String),
    PrintValue(Operand),
    PrintLn,
    WaitInput,
    Exit(i32),
    SleepMs(u64),
    InputInt(String),
    InputStr(String),
    Nop,

    Random(Dest),
    // High-level operations (opcodes 39-50)
    StrCat(String, String, String),     // dst, src1, src2
    StrFind(String, String, Dest),      // str, substr, result_dest
    Abs(Dest, Operand),                 // dest, src
    Min(Dest, Operand, Operand),        // dest, a, b
    Max(Dest, Operand, Operand),        // dest, a, b
    TimeMs(Dest),                       // dest
    Pow(Dest, Operand, Operand),        // dest, base, exp
    IntToStr(String, Operand),          // str_var, value
    StrToInt(String, Dest),             // str_var, result_dest
    Clamp(Dest, Operand, Operand, Operand), // dest, val, lo, hi
    RandomRange(Dest, Operand, Operand),    // dest, lo, hi
    Assert(Operand, String),            // cond, msg_str
    ReadFile(String, String),
    WriteFile(String, String),
    GetChar(String, Operand, Dest),
    SetChar(String, Operand, Operand),
    StrLen(String, Dest),
    StrAlloc(String, Operand),
    CompileDecretum(String, String, String),
}

fn program_uses_compile_decretum(program: &Program) -> bool {
    program
        .blocks
        .iter()
        .flat_map(|b| b.lines.iter())
        .map(|line| line.trim_start())
        .any(|line| line.starts_with("compile_decretum "))
}

fn resolve_full_runtime_exe(current_exe: &Path) -> Result<PathBuf, String> {
    if let Ok(path_raw) = std::env::var("DECRETUM_COMPILER_RUNTIME_EXE") {
        let path = PathBuf::from(path_raw);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(parent) = current_exe.parent() {
        let sibling = parent.join(format!("decretum.{}", std::env::consts::EXE_EXTENSION));
        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_release = manifest_dir
        .join("target")
        .join("release")
        .join(format!("decretum.{}", std::env::consts::EXE_EXTENSION));
    if target_release.is_file() {
        return Ok(target_release);
    }
    let target_debug = manifest_dir
        .join("target")
        .join("debug")
        .join(format!("decretum.{}", std::env::consts::EXE_EXTENSION));
    if target_debug.is_file() {
        return Ok(target_debug);
    }

    // If no dedicated compiler runtime is available, current exe is still valid.
    Ok(current_exe.to_path_buf())
}

fn resolve_runtime_stub_exe(current_exe: &Path) -> Result<PathBuf, String> {
    if let Ok(path_raw) = std::env::var("DECRETUM_RUNTIME_EXE") {
        let path = PathBuf::from(path_raw);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(parent) = current_exe.parent() {
        let sibling = parent.join(format!("dcrt_rt.{}", std::env::consts::EXE_EXTENSION));
        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_release = manifest_dir
        .join("target")
        .join("release")
        .join(format!("dcrt_rt.{}", std::env::consts::EXE_EXTENSION));
    if target_release.is_file() {
        return Ok(target_release);
    }

    let target_debug = manifest_dir
        .join("target")
        .join("debug")
        .join(format!("dcrt_rt.{}", std::env::consts::EXE_EXTENSION));
    if target_debug.is_file() {
        return Ok(target_debug);
    }

    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--bin",
            "dcrt_rt",
            "--no-default-features",
        ])
        .current_dir(&manifest_dir)
        .status()
        .map_err(|e| format!("failed to build runtime stub: {e}"))?;
    if !status.success() {
        return Err(format!(
            "failed to build runtime stub (cargo status {})",
            status
        ));
    }
    if target_release.is_file() {
        Ok(target_release)
    } else {
        Err(format!(
            "runtime stub build finished but file was not found: {}",
            target_release.display()
        ))
    }
}

#[derive(Debug, Clone)]
enum Dest {
    Reg(u8),
    Mem(String),
}

#[derive(Debug, Clone)]
enum Operand {
    Imm(i64),
    Reg(u8),
    Mem(String),
}

#[derive(Debug, Clone, Copy)]
enum Cond {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Below,
    BelowEq,
    Above,
    AboveEq,
}

#[derive(Debug, Clone)]
struct RawOp {
    op: RawOpKind,
}

#[derive(Debug, Clone)]
enum RawOpKind {
    Final(Op),
    Jump(String),
    JumpIf(Cond, String),
    CallLabel(String),
}

fn compile_program(program: &Program) -> Result<BytecodeProgram, String> {
    if program.target == "bios16" || program.target == "uefi"
        || program.target == "armcm" || program.target == "riscv"
        || program.target == "x86_64" || program.target == "riscv64"
        || program.target == "aarch64" || program.target == "macho"
        || program.target == "elf64" || program.target == "cheri"
        || program.target == "riscv_cheri" || program.target == "win32"
        || program.target == "elf32"
        || program.target == "mips" || program.target == "ppc" || program.target == "sparc"
        || program.target == "alpha" || program.target == "parisc" || program.target == "openrisc"
        || program.target == "nios2" || program.target == "microblaze"
        || program.target == "6502" || program.target == "z80" || program.target == "6809"
        || program.target == "pic" || program.target == "avr"
        || program.target == "sh2" || program.target == "sh4" || program.target == "m68k"
        || program.target == "ternary"
        || program.target == "quantum8" || program.target == "quantum64"
        || program.target == "ia64" || program.target == "vliw"
        || program.target == "s360" || program.target == "zarch" || program.target == "univac" || program.target == "cdc6600"
        || program.target == "pdp8" || program.target == "pdp11" || program.target == "vax" || program.target == "hp3000"
        || program.target == "i4004" || program.target == "i8008" || program.target == "i8080" || program.target == "i8086"
        || program.target == "m6800" || program.target == "mos6501"
        || program.target == "tms320" || program.target == "blackfin" || program.target == "sharc"
        || program.target == "c166" || program.target == "xc800" || program.target == "rl78" || program.target == "rx"
        || program.target == "h8" || program.target == "msp430"
        || program.target == "v20" || program.target == "nec78k" || program.target == "m16c" || program.target == "r8c" || program.target == "fr"
        || program.target == "mico32" || program.target == "picoblaze" || program.target == "mmix" || program.target == "dlx" || program.target == "lc3"
        || program.target == "huc6280" || program.target == "v810" || program.target == "arm7tdmi" || program.target == "arm9"
        || program.target == "ppc740" || program.target == "ppc970"
        || program.target == "mil1750a" || program.target == "jovial" || program.target == "ural" || program.target == "besm"
        || program.target == "elbrus" || program.target == "mir" || program.target == "harvard" || program.target == "mill"
    {
        return Err(format!(
            "{} must be built with the direct machine-code backend (compile-{})",
            program.target,
            match program.target.as_str() {
                "uefi" => "uefi",
                "armcm" => "armcm",
                "riscv" => "riscv",
                "x86_64" => "x86-64",
                "riscv64" => "riscv64",
                "aarch64" => "aarch64",
                "macho" => "macho",
                "elf64" => "elf64",
                "cheri" => "cheri",
                "riscv_cheri" => "riscv-cheri",
                "win32" => "win32",
                "elf32" => "elf32",
                _ => "bootimg",
            }
        ));
    }
    if program.target != "portable" && program.target != "win64" {
        return Err(format!(
            "portable backend supports target portable/win64, got '{}'",
            program.target
        ));
    }

    let mut int_data = BTreeMap::new();
    let mut string_data = BTreeMap::new();
    for decl in &program.data {
        match decl {
            DataDecl::String { name, value } => {
                string_data.insert(name.clone(), value.clone());
            }
            DataDecl::Scalar { name, value, .. } => {
                int_data.insert(name.clone(), *value);
            }
            DataDecl::Buffer { name, .. } => {
                int_data.insert(name.clone(), 0);
            }
        }
    }

    let mut blocks = Vec::new();
    for block in &program.blocks {
        blocks.push(compile_block(block)?);
    }

    if !blocks
        .iter()
        .any(|b| b.kind == BlockKind::Event && b.name == program.entry_event)
    {
        return Err(format!(
            "entry event '{}' was not compiled",
            program.entry_event
        ));
    }

    Ok(BytecodeProgram {
        entry_event: program.entry_event.clone(),
        int_data,
        string_data,
        blocks,
    })
}

fn compile_block(block: &Block) -> Result<BytecodeBlock, String> {
    let mut labels = BTreeMap::<String, usize>::new();
    let mut raw_ops = Vec::<RawOp>::new();
    let scope = format!("__block_{}", block.name);

    // Control flow construct stack
    struct CfFrame {
        kind: CfKind,
        endif_label: String,
        else_label: String,
        je_indices: Vec<usize>,  // indices of all je instructions (if + each elif), patched in reverse order
        has_else: bool,
    }
    #[derive(PartialEq)]
    enum CfKind { If, While }
    let mut cf_stack: Vec<CfFrame> = Vec::new();
    let mut cf_counter: u32 = 0;

    for line in &block.lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(label_raw) = trimmed.strip_suffix(':') {
            let label = qualify_label(label_raw.trim(), &scope);
            if labels.insert(label.clone(), raw_ops.len()).is_some() {
                return Err(format!(
                    "duplicate label '{}' in block '{}'",
                    label, block.name
                ));
            }
            continue;
        }

        if let Some(cond_str) = trimmed.strip_prefix("if ") {
            let cond = parse_operand(cond_str.trim())?;
            let endif_lbl = format!("__cf_{}_endif", cf_counter);
            let else_lbl = format!("__cf_{}_else", cf_counter);
            cf_counter += 1;
            raw_ops.push(RawOp { op: RawOpKind::Final(Op::Cmp(cond.clone(), Operand::Imm(0))) });
            let je_idx = raw_ops.len();
            raw_ops.push(RawOp { op: RawOpKind::JumpIf(Cond::Eq, endif_lbl.clone()) });
            cf_stack.push(CfFrame {
                kind: CfKind::If,
                endif_label: endif_lbl,
                else_label: else_lbl,
                je_indices: vec![je_idx],
                has_else: false,
            });
            continue;
        }

        if let Some(cond_str) = trimmed.strip_prefix("elif ") {
            let frame = cf_stack.last_mut().ok_or("elif without if")?;
            if frame.has_else {
                return Err("elif after else".to_string());
            }
            let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.je_indices.len());
            cf_counter += 1;
            // Patch the previous je to jump to this elif label
            let prev_je = frame.je_indices.last().ok_or("internal: no je indices")?;
            if let Some(RawOp { op: RawOpKind::JumpIf(_, label) }) = raw_ops.get_mut(*prev_je) {
                *label = elif_lbl.clone();
            }
            // jmp past the elif chain to endif
            raw_ops.push(RawOp { op: RawOpKind::Jump(frame.endif_label.clone()) });
            // Emit the elif label
            if labels.insert(elif_lbl, raw_ops.len()).is_some() {
                return Err("duplicate elif label".to_string());
            }
            // Emit comparison for this elif's condition
            let cond = parse_operand(cond_str.trim())?;
            raw_ops.push(RawOp { op: RawOpKind::Final(Op::Cmp(cond.clone(), Operand::Imm(0))) });
            let je_idx = raw_ops.len();
            raw_ops.push(RawOp { op: RawOpKind::JumpIf(Cond::Eq, frame.endif_label.clone()) });
            frame.je_indices.push(je_idx);
            continue;
        }

        if trimmed == "else" {
            let frame = cf_stack.last_mut().ok_or("else without if")?;
            if frame.has_else {
                return Err("duplicate else".to_string());
            }
            frame.has_else = true;
            // Patch the je from if/elif to jump to else instead of endif
            if let Some(RawOp { op: RawOpKind::JumpIf(_, label) }) = raw_ops.get_mut(*frame.je_indices.last().ok_or("internal: no je indices")?) {
                *label = frame.else_label.clone();
            }
            raw_ops.push(RawOp { op: RawOpKind::Jump(frame.endif_label.clone()) });
            if labels.insert(frame.else_label.clone(), raw_ops.len()).is_some() {
                return Err("duplicate else label".to_string());
            }
            continue;
        }

        if trimmed == "endif" {
            let frame = cf_stack.pop().ok_or("endif without if/while")?;
            if frame.kind == CfKind::While {
                return Err("endif without matching if".to_string());
            }
            if labels.insert(frame.endif_label.clone(), raw_ops.len()).is_some() {
                return Err(format!("duplicate endif label '{}'", frame.endif_label));
            }
            continue;
        }

        if let Some(cond_str) = trimmed.strip_prefix("while ") {
            let cond = parse_operand(cond_str.trim())?;
            let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
            let start_lbl = format!("__cf_{}_start", cf_counter);
            cf_counter += 1;
            // Insert start label at current position
            if labels.insert(start_lbl, raw_ops.len()).is_some() {
                return Err("duplicate while start label".to_string());
            }
            // cmp cond, 0
            raw_ops.push(RawOp { op: RawOpKind::Final(Op::Cmp(cond.clone(), Operand::Imm(0))) });
            // je endwhile
            let je_idx = raw_ops.len();
            raw_ops.push(RawOp { op: RawOpKind::JumpIf(Cond::Eq, endwhile_lbl.clone()) });
            cf_stack.push(CfFrame {
                kind: CfKind::While,
                endif_label: endwhile_lbl,
                else_label: String::new(),
                je_indices: vec![je_idx],
                has_else: false,
            });
            continue;
        }

        if trimmed == "endwhile" {
            let frame = cf_stack.pop().ok_or("endwhile without while")?;
            if frame.kind != CfKind::While {
                return Err("endwhile without matching while".to_string());
            }
            // jmp back to start
            let start_lbl = frame.endif_label.replace("_endwhile", "_start");
            raw_ops.push(RawOp { op: RawOpKind::Jump(start_lbl) });
            // Insert endwhile label at current position
            if labels.insert(frame.endif_label.clone(), raw_ops.len()).is_some() {
                return Err(format!("duplicate endwhile label '{}'", frame.endif_label));
            }
            continue;
        }

        raw_ops.push(parse_line_to_raw(trimmed, &scope)?);
    }

    if !cf_stack.is_empty() {
        return Err("unclosed control flow construct".to_string());
    }

    let mut ops = Vec::<Op>::new();
    for raw in raw_ops {
        match raw.op {
            RawOpKind::Final(op) => ops.push(op),
            RawOpKind::Jump(label) => {
                let target = labels.get(&label).ok_or_else(|| {
                    format!("unknown jump label '{}' in block '{}'", label, block.name)
                })?;
                ops.push(Op::Jump(*target as u32));
            }
            RawOpKind::JumpIf(cond, label) => {
                let target = labels.get(&label).ok_or_else(|| {
                    format!("unknown jump label '{}' in block '{}'", label, block.name)
                })?;
                ops.push(Op::JumpIf(cond, *target as u32));
            }
            RawOpKind::CallLabel(label) => {
                let target = labels.get(&label).ok_or_else(|| {
                    format!("unknown call label '{}' in block '{}'", label, block.name)
                })?;
                ops.push(Op::CallLabel(*target as u32));
            }
        }
    }

    Ok(BytecodeBlock {
        kind: block.kind,
        name: block.name.clone(),
        ops,
    })
}

fn parse_line_to_raw(line: &str, scope: &str) -> Result<RawOp, String> {
    if let Some(rest) = line.strip_prefix("emit ") {
        let name = rest.trim();
        if !is_ident(name) {
            return Err(format!("invalid event name '{}' in emit", name));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Emit(name.to_string())),
        });
    }

    if let Some(rest) = line.strip_prefix("call ") {
        let name = rest.trim();
        if name.starts_with('.') {
            return Ok(RawOp {
                op: RawOpKind::CallLabel(qualify_label(name, scope)),
            });
        }
        if !is_ident(name) {
            return Err(format!("invalid procedure name '{}' in call", name));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::CallProc(name.to_string())),
        });
    }

    if line == "ret" {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Ret),
        });
    }
    if line == "nop" {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Nop),
        });
    }
    if line == "println" {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintLn),
        });
    }
    if line == "wait_input" {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::WaitInput),
        });
    }

    if let Some(rest) = line.strip_prefix("print_data ") {
        let name = rest.trim();
        if !is_ident(name) {
            return Err(format!("invalid data label '{}'", name));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintData(name.to_string())),
        });
    }

    if let Some(rest) = line.strip_prefix("print_u64 ") {
        let operand = parse_operand(rest.trim())?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintValue(operand)),
        });
    }
    if let Some(rest) = line.strip_prefix("print_var ") {
        let operand = parse_operand(rest.trim())?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintValue(operand)),
        });
    }

    if let Some(rest) = line.strip_prefix("print ") {
        let text = parse_quoted_string(rest.trim())?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintText(text)),
        });
    }
    if let Some(rest) = line.strip_prefix("println ") {
        let text = parse_quoted_string(rest.trim())?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::PrintText(format!("{text}\n"))),
        });
    }

    if let Some(rest) = line.strip_prefix("exit ") {
        let code = parse_integer(rest.trim())?;
        if !(i32::MIN as i64..=i32::MAX as i64).contains(&code) {
            return Err(format!("exit code out of range: {code}"));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Exit(code as i32)),
        });
    }

    if let Some(rest) = line.strip_prefix("sleep_ms ") {
        let ms = parse_integer(rest.trim())?;
        if ms < 0 {
            return Err(format!("sleep_ms must be >= 0, got {ms}"));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::SleepMs(ms as u64)),
        });
    }

    if let Some(rest) = line.strip_prefix("input_str ") {
        let name = rest.trim();
        if !is_ident(name) {
            return Err(format!("invalid input_str target '{}'", name));
        }
        return Ok(RawOp {
            op: RawOpKind::Final(Op::InputStr(name.to_string())),
        });
    }
    if let Some(rest) = line.strip_prefix("input ") {
        let raw = rest.trim();
        let target = parse_dest(raw)?;
        match target {
            Dest::Mem(name) => {
                return Ok(RawOp {
                    op: RawOpKind::Final(Op::InputInt(name)),
                });
            }
            Dest::Reg(_) => {
                return Err(
                    "input target must be a memory variable (use [name] or name)".to_string(),
                );
            }
        }
    }

    if let Some(rest) = line.strip_prefix("jmp ") {
        return Ok(RawOp {
            op: RawOpKind::Jump(qualify_label(rest.trim(), scope)),
        });
    }

    if let Some(rest) = line
        .strip_prefix("je ")
        .or_else(|| line.strip_prefix("jz "))
    {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Eq, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line
        .strip_prefix("jne ")
        .or_else(|| line.strip_prefix("jnz "))
    {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Ne, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jl ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Lt, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jle ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Le, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jg ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Gt, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jge ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Ge, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jb ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Below, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jbe ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::BelowEq, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("ja ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::Above, qualify_label(rest.trim(), scope)),
        });
    }
    if let Some(rest) = line.strip_prefix("jae ") {
        return Ok(RawOp {
            op: RawOpKind::JumpIf(Cond::AboveEq, qualify_label(rest.trim(), scope)),
        });
    }

    if let Some(rest) = line.strip_prefix("cmp ") {
        let (left, right) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Cmp(parse_operand(left)?, parse_operand(right)?)),
        });
    }

    if let Some(rest) = line.strip_prefix("mov ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Mov(parse_dest(dst)?, parse_operand(src)?)),
        });
    }

    if let Some(rest) = line.strip_prefix("add ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Add(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("sub ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Sub(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line
        .strip_prefix("mul ")
        .or_else(|| line.strip_prefix("imul "))
    {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Mul(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("div ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Div(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("mod ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Mod(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("and ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::And(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("or ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Or(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("xor ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Xor(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("shl ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Shl(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("shr ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Shr(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("inc ") {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Inc(parse_dest(rest.trim())?)),
        });
    }
    if let Some(rest) = line.strip_prefix("dec ") {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Dec(parse_dest(rest.trim())?)),
        });
    }
    if let Some(rest) = line.strip_prefix("not ") {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Not(parse_dest(rest.trim())?)),
        });
    }

    if let Some(rest) = line.strip_prefix("random ") {
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Random(parse_dest(rest.trim())?)),
        });
    }

    if let Some(rest) = line.strip_prefix("read_file ") {
        let (file, dest) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::ReadFile(file.to_string(), dest.to_string())),
        });
    }
    if let Some(rest) = line.strip_prefix("write_file ") {
        let (file, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::WriteFile(file.to_string(), src.to_string())),
        });
    }
    if let Some(rest) = line.strip_prefix("get_char ") {
        let (str_var, rhs) = split2(rest)?;
        let (idx, dest) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::GetChar(
                str_var.to_string(),
                parse_operand(idx)?,
                parse_dest(dest)?,
            )),
        });
    }
    if let Some(rest) = line.strip_prefix("set_char ") {
        let (str_var, rhs) = split2(rest)?;
        let (idx, val) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::SetChar(
                str_var.to_string(),
                parse_operand(idx)?,
                parse_operand(val)?,
            )),
        });
    }
    if let Some(rest) = line.strip_prefix("str_len ") {
        let (str_var, dest) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::StrLen(str_var.to_string(), parse_dest(dest)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("str_alloc ") {
        let (str_var, size) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::StrAlloc(str_var.to_string(), parse_operand(size)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("compile_decretum ") {
        let (src, rhs) = split2(rest)?;
        let (out, tgt) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::CompileDecretum(
                src.to_string(),
                out.to_string(),
                tgt.to_string(),
            )),
        });
    }

    // High level instructions
    if let Some(rest) = line.strip_prefix("str_cat ") {
        let (dst, rhs) = split2(rest)?;
        let (src1, src2) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::StrCat(
                dst.to_string(),
                src1.to_string(),
                src2.to_string(),
            )),
        });
    }
    if let Some(rest) = line.strip_prefix("str_find ") {
        let (str_var, rhs) = split2(rest)?;
        let (substr, result) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::StrFind(
                str_var.to_string(),
                substr.to_string(),
                parse_dest(result)?,
            )),
        });
    }
    if let Some(rest) = line.strip_prefix("abs ") {
        let (dst, src) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Abs(parse_dest(dst)?, parse_operand(src)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("min ") {
        let (dst, rhs) = split2(rest)?;
        let (a, b) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Min(parse_dest(dst)?, parse_operand(a)?, parse_operand(b)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("max ") {
        let (dst, rhs) = split2(rest)?;
        let (a, b) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Max(parse_dest(dst)?, parse_operand(a)?, parse_operand(b)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("time_ms ") {
        let dst = rest.trim();
        return Ok(RawOp {
            op: RawOpKind::Final(Op::TimeMs(parse_dest(dst)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("pow ") {
        let (dst, rhs) = split2(rest)?;
        let (base, exp) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Pow(parse_dest(dst)?, parse_operand(base)?, parse_operand(exp)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("int_to_str ") {
        let (str_var, value) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::IntToStr(str_var.to_string(), parse_operand(value)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("str_to_int ") {
        let (str_var, result) = split2(rest)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::StrToInt(str_var.to_string(), parse_dest(result)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("clamp ") {
        // clamp dst, val, lo, hi
        let (dst, a) = split2(rest)?;
        let (val, b) = split2(a)?;
        let (lo, hi) = split2(b)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Clamp(
                parse_dest(dst)?,
                parse_operand(val)?,
                parse_operand(lo)?,
                parse_operand(hi)?,
            )),
        });
    }
    if let Some(rest) = line.strip_prefix("random_range ") {
        let (dst, rhs) = split2(rest)?;
        let (lo, hi) = split2(rhs)?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::RandomRange(parse_dest(dst)?, parse_operand(lo)?, parse_operand(hi)?)),
        });
    }
    if let Some(rest) = line.strip_prefix("assert ") {
        // assert cond, "message"
        let (cond, msg) = split2(rest)?;
        let text = parse_quoted_string(msg.trim())?;
        return Ok(RawOp {
            op: RawOpKind::Final(Op::Assert(parse_operand(cond)?, text)),
        });
    }

    Err(format!("unsupported portable instruction '{}'", line))
}

fn qualify_label(raw: &str, scope: &str) -> String {
    if raw.starts_with('.') {
        format!("{scope}{raw}")
    } else {
        raw.to_string()
    }
}

fn split2(rest: &str) -> Result<(&str, &str), String> {
    let (a, b) = rest
        .split_once(',')
        .ok_or_else(|| format!("expected two operands in '{}'", rest))?;
    Ok((a.trim(), b.trim()))
}

fn parse_dest(raw: &str) -> Result<Dest, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("missing destination".to_string());
    }
    if let Some(inner) = parse_memory_ref(text) {
        return Ok(Dest::Mem(inner));
    }
    if let Some(reg) = parse_register(text) {
        return Ok(Dest::Reg(reg));
    }
    if is_ident(text) {
        return Ok(Dest::Mem(text.to_string()));
    }
    Err(format!("invalid destination '{}'", text))
}

fn parse_operand(raw: &str) -> Result<Operand, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("missing operand".to_string());
    }
    if let Some(inner) = parse_memory_ref(text) {
        return Ok(Operand::Mem(inner));
    }
    if let Some(reg) = parse_register(text) {
        return Ok(Operand::Reg(reg));
    }
    if let Ok(value) = parse_integer(text) {
        return Ok(Operand::Imm(value));
    }
    if is_ident(text) {
        return Ok(Operand::Mem(text.to_string()));
    }
    Err(format!("invalid operand '{}'", text))
}

fn parse_memory_ref(text: &str) -> Option<String> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?.trim();
    if is_ident(inner) {
        Some(inner.to_string())
    } else {
        None
    }
}

fn parse_register(text: &str) -> Option<u8> {
    match text.to_ascii_lowercase().as_str() {
        "rax" => Some(0),
        "rbx" => Some(1),
        "rcx" => Some(2),
        "rdx" => Some(3),
        "rsi" => Some(4),
        "rdi" => Some(5),
        "rbp" => Some(6),
        "rsp" => Some(7),
        "r8" => Some(8),
        "r9" => Some(9),
        "r10" => Some(10),
        "r11" => Some(11),
        "r12" => Some(12),
        "r13" => Some(13),
        "r14" => Some(14),
        "r15" => Some(15),
        _ => None,
    }
}

fn encode_program(program: &BytecodeProgram) -> Result<Vec<u8>, String> {
    let mut w = ByteWriter::new();
    w.write_bytes(DCB_MAGIC);
    w.write_u16(DCB_VERSION);
    w.write_string(&program.entry_event)?;

    w.write_u32(program.int_data.len() as u32);
    for (name, value) in &program.int_data {
        w.write_string(name)?;
        w.write_i64(*value);
    }

    w.write_u32(program.string_data.len() as u32);
    for (name, value) in &program.string_data {
        w.write_string(name)?;
        w.write_string(value)?;
    }

    w.write_u32(program.blocks.len() as u32);
    for block in &program.blocks {
        let kind = match block.kind {
            BlockKind::Event => 0u8,
            BlockKind::Proc => 1u8,
        };
        w.write_u8(kind);
        w.write_string(&block.name)?;
        w.write_u32(block.ops.len() as u32);
        for op in &block.ops {
            encode_op(op, &mut w)?;
        }
    }

    Ok(w.finish())
}

fn encode_op(op: &Op, w: &mut ByteWriter) -> Result<(), String> {
    match op {
        Op::Mov(dst, src) => {
            w.write_u8(0);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Add(dst, src) => {
            w.write_u8(1);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Sub(dst, src) => {
            w.write_u8(2);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Mul(dst, src) => {
            w.write_u8(3);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Div(dst, src) => {
            w.write_u8(4);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Mod(dst, src) => {
            w.write_u8(5);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::And(dst, src) => {
            w.write_u8(6);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Or(dst, src) => {
            w.write_u8(7);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Xor(dst, src) => {
            w.write_u8(8);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Shl(dst, src) => {
            w.write_u8(9);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Shr(dst, src) => {
            w.write_u8(10);
            encode_dest(dst, w)?;
            encode_operand(src, w)?;
        }
        Op::Not(dst) => {
            w.write_u8(11);
            encode_dest(dst, w)?;
        }
        Op::Inc(dst) => {
            w.write_u8(12);
            encode_dest(dst, w)?;
        }
        Op::Dec(dst) => {
            w.write_u8(13);
            encode_dest(dst, w)?;
        }
        Op::Cmp(a, b) => {
            w.write_u8(14);
            encode_operand(a, w)?;
            encode_operand(b, w)?;
        }
        Op::Jump(target) => {
            w.write_u8(15);
            w.write_u32(*target);
        }
        Op::JumpIf(cond, target) => {
            w.write_u8(16);
            w.write_u8(encode_cond(*cond));
            w.write_u32(*target);
        }
        Op::Emit(name) => {
            w.write_u8(17);
            w.write_string(name)?;
        }
        Op::CallProc(name) => {
            w.write_u8(18);
            w.write_string(name)?;
        }
        Op::Ret => {
            w.write_u8(19);
        }
        Op::PrintText(text) => {
            w.write_u8(20);
            w.write_string(text)?;
        }
        Op::PrintData(name) => {
            w.write_u8(21);
            w.write_string(name)?;
        }
        Op::PrintValue(value) => {
            w.write_u8(22);
            encode_operand(value, w)?;
        }
        Op::PrintLn => {
            w.write_u8(23);
        }
        Op::WaitInput => {
            w.write_u8(24);
        }
        Op::Exit(code) => {
            w.write_u8(25);
            w.write_i32(*code);
        }
        Op::SleepMs(ms) => {
            w.write_u8(26);
            w.write_u64(*ms);
        }
        Op::InputInt(name) => {
            w.write_u8(27);
            w.write_string(name)?;
        }
        Op::InputStr(name) => {
            w.write_u8(37);
            w.write_string(name)?;
        }
        Op::Nop => {
            w.write_u8(28);
        }
        Op::Random(dst) => {
            w.write_u8(38);
            encode_dest(dst, w)?;
        }
        Op::CallLabel(target) => {
            w.write_u8(29);
            w.write_u32(*target);
        }
        Op::ReadFile(file, dest) => {
            w.write_u8(30);
            w.write_string(file)?;
            w.write_string(dest)?;
        }
        Op::WriteFile(file, src) => {
            w.write_u8(31);
            w.write_string(file)?;
            w.write_string(src)?;
        }
        Op::GetChar(str_var, idx, dest) => {
            w.write_u8(32);
            w.write_string(str_var)?;
            encode_operand(idx, w)?;
            encode_dest(dest, w)?;
        }
        Op::SetChar(str_var, idx, val) => {
            w.write_u8(33);
            w.write_string(str_var)?;
            encode_operand(idx, w)?;
            encode_operand(val, w)?;
        }
        Op::StrLen(str_var, dest) => {
            w.write_u8(34);
            w.write_string(str_var)?;
            encode_dest(dest, w)?;
        }
        Op::StrAlloc(str_var, size) => {
            w.write_u8(35);
            w.write_string(str_var)?;
            encode_operand(size, w)?;
        }
        Op::CompileDecretum(src, out, tgt) => {
            w.write_u8(36);
            w.write_string(src)?;
            w.write_string(out)?;
            w.write_string(tgt)?;
        }
        // High-level ops (39-50)
        Op::StrCat(dst, src1, src2) => {
            w.write_u8(39);
            w.write_string(dst)?;
            w.write_string(src1)?;
            w.write_string(src2)?;
        }
        Op::StrFind(str_var, substr, result) => {
            w.write_u8(40);
            w.write_string(str_var)?;
            w.write_string(substr)?;
            encode_dest(result, w)?;
        }
        Op::Abs(dest, src) => {
            w.write_u8(41);
            encode_dest(dest, w)?;
            encode_operand(src, w)?;
        }
        Op::Min(dest, a, b) => {
            w.write_u8(42);
            encode_dest(dest, w)?;
            encode_operand(a, w)?;
            encode_operand(b, w)?;
        }
        Op::Max(dest, a, b) => {
            w.write_u8(43);
            encode_dest(dest, w)?;
            encode_operand(a, w)?;
            encode_operand(b, w)?;
        }
        Op::TimeMs(dest) => {
            w.write_u8(44);
            encode_dest(dest, w)?;
        }
        Op::Pow(dest, base, exp) => {
            w.write_u8(45);
            encode_dest(dest, w)?;
            encode_operand(base, w)?;
            encode_operand(exp, w)?;
        }
        Op::IntToStr(str_var, value) => {
            w.write_u8(46);
            w.write_string(str_var)?;
            encode_operand(value, w)?;
        }
        Op::StrToInt(str_var, result) => {
            w.write_u8(47);
            w.write_string(str_var)?;
            encode_dest(result, w)?;
        }
        Op::Clamp(dest, val, lo, hi) => {
            w.write_u8(48);
            encode_dest(dest, w)?;
            encode_operand(val, w)?;
            encode_operand(lo, w)?;
            encode_operand(hi, w)?;
        }
        Op::RandomRange(dest, lo, hi) => {
            w.write_u8(49);
            encode_dest(dest, w)?;
            encode_operand(lo, w)?;
            encode_operand(hi, w)?;
        }
        Op::Assert(cond, msg) => {
            w.write_u8(50);
            encode_operand(cond, w)?;
            w.write_string(msg)?;
        }
    }

    Ok(())
}

fn encode_dest(dest: &Dest, w: &mut ByteWriter) -> Result<(), String> {
    match dest {
        Dest::Reg(reg) => {
            w.write_u8(0);
            w.write_u8(*reg);
        }
        Dest::Mem(name) => {
            w.write_u8(1);
            w.write_string(name)?;
        }
    }
    Ok(())
}

fn encode_operand(operand: &Operand, w: &mut ByteWriter) -> Result<(), String> {
    match operand {
        Operand::Imm(v) => {
            w.write_u8(0);
            w.write_i64(*v);
        }
        Operand::Reg(reg) => {
            w.write_u8(1);
            w.write_u8(*reg);
        }
        Operand::Mem(name) => {
            w.write_u8(2);
            w.write_string(name)?;
        }
    }
    Ok(())
}

fn encode_cond(cond: Cond) -> u8 {
    match cond {
        Cond::Eq => 0,
        Cond::Ne => 1,
        Cond::Lt => 2,
        Cond::Le => 3,
        Cond::Gt => 4,
        Cond::Ge => 5,
        Cond::Below => 6,
        Cond::BelowEq => 7,
        Cond::Above => 8,
        Cond::AboveEq => 9,
    }
}

fn decode_cond(raw: u8) -> Result<Cond, String> {
    match raw {
        0 => Ok(Cond::Eq),
        1 => Ok(Cond::Ne),
        2 => Ok(Cond::Lt),
        3 => Ok(Cond::Le),
        4 => Ok(Cond::Gt),
        5 => Ok(Cond::Ge),
        6 => Ok(Cond::Below),
        7 => Ok(Cond::BelowEq),
        8 => Ok(Cond::Above),
        9 => Ok(Cond::AboveEq),
        _ => Err(format!("unknown condition opcode {raw}")),
    }
}

fn decode_program(bytes: &[u8]) -> Result<BytecodeProgram, String> {
    let mut r = ByteReader::new(bytes);
    let magic = r.read_exact(4)?;
    if magic != DCB_MAGIC {
        return Err("invalid bytecode header (expected DCB2)".to_string());
    }
    let version = r.read_u16()?;
    if version != DCB_VERSION {
        return Err(format!(
            "unsupported bytecode version {version}, expected {DCB_VERSION}"
        ));
    }

    let entry_event = r.read_string()?;
    let int_count = r.read_u32()? as usize;
    let mut int_data = BTreeMap::new();
    for _ in 0..int_count {
        let name = r.read_string()?;
        let value = r.read_i64()?;
        int_data.insert(name, value);
    }

    let string_count = r.read_u32()? as usize;
    let mut string_data = BTreeMap::new();
    for _ in 0..string_count {
        let name = r.read_string()?;
        let value = r.read_string()?;
        string_data.insert(name, value);
    }

    let block_count = r.read_u32()? as usize;
    let mut blocks = Vec::with_capacity(block_count);
    for _ in 0..block_count {
        let kind = match r.read_u8()? {
            0 => BlockKind::Event,
            1 => BlockKind::Proc,
            other => return Err(format!("invalid block kind opcode {other}")),
        };
        let name = r.read_string()?;
        let op_count = r.read_u32()? as usize;
        let mut ops = Vec::with_capacity(op_count);
        for _ in 0..op_count {
            ops.push(decode_op(&mut r)?);
        }
        blocks.push(BytecodeBlock { kind, name, ops });
    }

    Ok(BytecodeProgram {
        entry_event,
        int_data,
        string_data,
        blocks,
    })
}

fn decode_op(r: &mut ByteReader<'_>) -> Result<Op, String> {
    let opcode = r.read_u8()?;
    match opcode {
        0 => Ok(Op::Mov(decode_dest(r)?, decode_operand(r)?)),
        1 => Ok(Op::Add(decode_dest(r)?, decode_operand(r)?)),
        2 => Ok(Op::Sub(decode_dest(r)?, decode_operand(r)?)),
        3 => Ok(Op::Mul(decode_dest(r)?, decode_operand(r)?)),
        4 => Ok(Op::Div(decode_dest(r)?, decode_operand(r)?)),
        5 => Ok(Op::Mod(decode_dest(r)?, decode_operand(r)?)),
        6 => Ok(Op::And(decode_dest(r)?, decode_operand(r)?)),
        7 => Ok(Op::Or(decode_dest(r)?, decode_operand(r)?)),
        8 => Ok(Op::Xor(decode_dest(r)?, decode_operand(r)?)),
        9 => Ok(Op::Shl(decode_dest(r)?, decode_operand(r)?)),
        10 => Ok(Op::Shr(decode_dest(r)?, decode_operand(r)?)),
        11 => Ok(Op::Not(decode_dest(r)?)),
        12 => Ok(Op::Inc(decode_dest(r)?)),
        13 => Ok(Op::Dec(decode_dest(r)?)),
        14 => Ok(Op::Cmp(decode_operand(r)?, decode_operand(r)?)),
        15 => Ok(Op::Jump(r.read_u32()?)),
        16 => Ok(Op::JumpIf(decode_cond(r.read_u8()?)?, r.read_u32()?)),
        17 => Ok(Op::Emit(r.read_string()?)),
        18 => Ok(Op::CallProc(r.read_string()?)),
        19 => Ok(Op::Ret),
        20 => Ok(Op::PrintText(r.read_string()?)),
        21 => Ok(Op::PrintData(r.read_string()?)),
        22 => Ok(Op::PrintValue(decode_operand(r)?)),
        23 => Ok(Op::PrintLn),
        24 => Ok(Op::WaitInput),
        25 => Ok(Op::Exit(r.read_i32()?)),
        26 => Ok(Op::SleepMs(r.read_u64()?)),
        27 => Ok(Op::InputInt(r.read_string()?)),
        28 => Ok(Op::Nop),
        29 => Ok(Op::CallLabel(r.read_u32()?)),
        30 => Ok(Op::ReadFile(r.read_string()?, r.read_string()?)),
        31 => Ok(Op::WriteFile(r.read_string()?, r.read_string()?)),
        32 => Ok(Op::GetChar(
            r.read_string()?,
            decode_operand(r)?,
            decode_dest(r)?,
        )),
        33 => Ok(Op::SetChar(
            r.read_string()?,
            decode_operand(r)?,
            decode_operand(r)?,
        )),
        34 => Ok(Op::StrLen(r.read_string()?, decode_dest(r)?)),
        35 => Ok(Op::StrAlloc(r.read_string()?, decode_operand(r)?)),
        36 => Ok(Op::CompileDecretum(
            r.read_string()?,
            r.read_string()?,
            r.read_string()?,
        )),
        37 => Ok(Op::InputStr(r.read_string()?)),
        38 => Ok(Op::Random(decode_dest(r)?)),
        39 => Ok(Op::StrCat(r.read_string()?, r.read_string()?, r.read_string()?)),
        40 => Ok(Op::StrFind(r.read_string()?, r.read_string()?, decode_dest(r)?)),
        41 => Ok(Op::Abs(decode_dest(r)?, decode_operand(r)?)),
        42 => Ok(Op::Min(decode_dest(r)?, decode_operand(r)?, decode_operand(r)?)),
        43 => Ok(Op::Max(decode_dest(r)?, decode_operand(r)?, decode_operand(r)?)),
        44 => Ok(Op::TimeMs(decode_dest(r)?)),
        45 => Ok(Op::Pow(decode_dest(r)?, decode_operand(r)?, decode_operand(r)?)),
        46 => Ok(Op::IntToStr(r.read_string()?, decode_operand(r)?)),
        47 => Ok(Op::StrToInt(r.read_string()?, decode_dest(r)?)),
        48 => Ok(Op::Clamp(decode_dest(r)?, decode_operand(r)?, decode_operand(r)?, decode_operand(r)?)),
        49 => Ok(Op::RandomRange(decode_dest(r)?, decode_operand(r)?, decode_operand(r)?)),
        50 => Ok(Op::Assert(decode_operand(r)?, r.read_string()?)),
        _ => Err(format!("unknown opcode {opcode}")),
    }
}

fn decode_dest(r: &mut ByteReader<'_>) -> Result<Dest, String> {
    match r.read_u8()? {
        0 => Ok(Dest::Reg(r.read_u8()?)),
        1 => Ok(Dest::Mem(r.read_string()?)),
        other => Err(format!("invalid destination tag {other}")),
    }
}

fn decode_operand(r: &mut ByteReader<'_>) -> Result<Operand, String> {
    match r.read_u8()? {
        0 => Ok(Operand::Imm(r.read_i64()?)),
        1 => Ok(Operand::Reg(r.read_u8()?)),
        2 => Ok(Operand::Mem(r.read_string()?)),
        other => Err(format!("invalid operand tag {other}")),
    }
}

struct ByteWriter {
    bytes: Vec<u8>,
}

impl ByteWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_bytes(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_string(&mut self, text: &str) -> Result<(), String> {
        let len = text.len();
        if len > u32::MAX as usize {
            return Err("string too long to encode".to_string());
        }
        self.write_u32(len as u32);
        self.write_bytes(text.as_bytes());
        Ok(())
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| "bytecode read overflow".to_string())?;
        if end > self.bytes.len() {
            return Err("unexpected end of bytecode".to_string());
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut arr = [0u8; 2];
        arr.copy_from_slice(self.read_exact(2)?);
        Ok(u16::from_le_bytes(arr))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(arr))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(self.read_exact(8)?);
        Ok(u64::from_le_bytes(arr))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(self.read_exact(4)?);
        Ok(i32::from_le_bytes(arr))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(self.read_exact(8)?);
        Ok(i64::from_le_bytes(arr))
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("invalid utf-8 in bytecode string: {e}"))
    }
}

#[derive(Debug, Clone)]
struct Frame {
    block_index: usize,
    ip: usize,
}

pub struct BytecodeRuntime {
    program: BytecodeProgram,
    event_lookup: BTreeMap<String, usize>,
    proc_lookup: BTreeMap<String, usize>,
    memory: BTreeMap<String, i64>,
    strings: BTreeMap<String, String>,
    regs: [i64; 16],
    frames: Vec<Frame>,
    cmp_signed: i8,
    cmp_unsigned: i8,
    halted: bool,
    exit_code: i32,
    // DCRNG state - 320-bit ARX-based PRNG
    rng_a: u64,
    rng_b: u64,
    rng_c: u64,
    rng_d: u64,
    rng_weyl: u64,
}

impl BytecodeRuntime {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let program = decode_program(bytes)?;
        let mut event_lookup = BTreeMap::new();
        let mut proc_lookup = BTreeMap::new();
        for (idx, block) in program.blocks.iter().enumerate() {
            match block.kind {
                BlockKind::Event => {
                    event_lookup.insert(block.name.clone(), idx);
                }
                BlockKind::Proc => {
                    proc_lookup.insert(block.name.clone(), idx);
                }
            }
        }

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        // Expand seed into 5 state words using splitmix64-style mixing
        let mut sm = seed;
        let rng_a = sm.wrapping_add(0x9E3779B97F4A7C15);
        sm = sm.wrapping_add(0x9E3779B97F4A7C15);
        let rng_b = sm;
        sm = sm.wrapping_add(0x9E3779B97F4A7C15);
        let rng_c = sm;
        sm = sm.wrapping_add(0x9E3779B97F4A7C15);
        let rng_d = sm;
        let rng_weyl = seed.wrapping_mul(6364136223846793005);

        Ok(Self {
            event_lookup,
            proc_lookup,
            memory: program.int_data.clone(),
            strings: program.string_data.clone(),
            regs: [0; 16],
            frames: Vec::new(),
            cmp_signed: 0,
            cmp_unsigned: 0,
            halted: false,
            exit_code: 0,
            rng_a,
            rng_b,
            rng_c,
            rng_d,
            rng_weyl,
            program,
        })
    }

    pub fn enqueue_event(&mut self, event: BytecodeEvent) -> Result<(), String> {
        let idx = self
            .event_lookup
            .get(&event.name)
            .copied()
            .ok_or_else(|| format!("unknown event '{}'", event.name))?;
        self.frames.push(Frame {
            block_index: idx,
            ip: 0,
        });
        Ok(())
    }

    pub fn run_entry(&mut self) -> Result<i32, String> {
        let entry = self.program.entry_event.clone();
        self.enqueue_event(BytecodeEvent { name: entry })?;
        self.run()
    }

    pub fn run(&mut self) -> Result<i32, String> {
        let mut steps: u64 = 0;
        while !self.frames.is_empty() && !self.halted {
            self.step()?;
            steps = steps.saturating_add(1);
            if steps > 50_000_000 {
                return Err(
                    "execution aborted after too many instructions (possible infinite loop)"
                        .to_string(),
                );
            }
        }
        Ok(self.exit_code)
    }

    pub fn memory_value(&self, name: &str) -> Option<i64> {
        self.memory.get(name).copied()
    }

    fn step(&mut self) -> Result<(), String> {
        if self.frames.is_empty() || self.halted {
            return Ok(());
        }

        let frame_index = self.frames.len() - 1;
        let block_index = self.frames[frame_index].block_index;
        let ip = self.frames[frame_index].ip;
        let block = self
            .program
            .blocks
            .get(block_index)
            .ok_or_else(|| format!("invalid frame block index {block_index}"))?;
        if ip >= block.ops.len() {
            self.frames.pop();
            return Ok(());
        }
        let op = block.ops[ip].clone();
        self.frames[frame_index].ip += 1;

        match op {
            Op::Mov(dst, src) => {
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, value);
            }
            Op::Add(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current.wrapping_add(value));
            }
            Op::Sub(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current.wrapping_sub(value));
            }
            Op::Mul(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current.wrapping_mul(value));
            }
            Op::Div(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                if value == 0 {
                    return Err("division by zero".to_string());
                }
                self.write_dest(&dst, current.wrapping_div(value));
            }
            Op::Mod(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                if value == 0 {
                    return Err("modulo by zero".to_string());
                }
                self.write_dest(&dst, current.wrapping_rem(value));
            }
            Op::And(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current & value);
            }
            Op::Or(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current | value);
            }
            Op::Xor(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                self.write_dest(&dst, current ^ value);
            }
            Op::Shl(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                let shift = (value & 63) as u32;
                self.write_dest(&dst, current.wrapping_shl(shift));
            }
            Op::Shr(dst, src) => {
                let current = self.read_dest(&dst);
                let value = self.read_operand(&src)?;
                let shift = (value & 63) as u32;
                self.write_dest(&dst, ((current as u64).wrapping_shr(shift)) as i64);
            }
            Op::Not(dst) => {
                let current = self.read_dest(&dst);
                self.write_dest(&dst, !current);
            }
            Op::Inc(dst) => {
                let current = self.read_dest(&dst);
                self.write_dest(&dst, current.wrapping_add(1));
            }
            Op::Dec(dst) => {
                let current = self.read_dest(&dst);
                self.write_dest(&dst, current.wrapping_sub(1));
            }
            Op::Cmp(a, b) => {
                let av = self.read_operand(&a)?;
                let bv = self.read_operand(&b)?;
                self.cmp_signed = if av < bv {
                    -1
                } else if av > bv {
                    1
                } else {
                    0
                };
                let au = av as u64;
                let bu = bv as u64;
                self.cmp_unsigned = if au < bu {
                    -1
                } else if au > bu {
                    1
                } else {
                    0
                };
            }
            Op::Jump(target) => {
                self.jump_to(frame_index, target)?;
            }
            Op::JumpIf(cond, target) => {
                if self.matches_cond(cond) {
                    self.jump_to(frame_index, target)?;
                }
            }
            Op::Emit(name) => {
                let idx = self
                    .event_lookup
                    .get(&name)
                    .copied()
                    .ok_or_else(|| format!("unknown event '{}'", name))?;
                self.frames.push(Frame {
                    block_index: idx,
                    ip: 0,
                });
            }
            Op::CallProc(name) => {
                let idx = self
                    .proc_lookup
                    .get(&name)
                    .copied()
                    .ok_or_else(|| format!("unknown procedure '{}'", name))?;
                self.frames.push(Frame {
                    block_index: idx,
                    ip: 0,
                });
            }
            Op::CallLabel(target) => {
                self.frames.push(Frame {
                    block_index,
                    ip: target as usize,
                });
            }
            Op::Ret => {
                self.frames.pop();
            }
            Op::PrintText(text) => {
                print!("{text}");
                io::stdout()
                    .flush()
                    .map_err(|e| format!("stdout flush failed: {e}"))?;
            }
            Op::PrintData(name) => {
                if let Some(text) = self.strings.get(&name) {
                    print!("{text}");
                } else {
                    let value = self.memory.get(&name).copied().unwrap_or(0);
                    print!("{value}");
                }
                io::stdout()
                    .flush()
                    .map_err(|e| format!("stdout flush failed: {e}"))?;
            }
            Op::PrintValue(operand) => {
                let value = self.read_operand(&operand)?;
                print!("{value}");
                io::stdout()
                    .flush()
                    .map_err(|e| format!("stdout flush failed: {e}"))?;
            }
            Op::PrintLn => {
                println!();
            }
            Op::WaitInput => {
                let mut line = String::new();
                io::stdin()
                    .read_line(&mut line)
                    .map_err(|e| format!("stdin read failed: {e}"))?;
            }
            Op::Exit(code) => {
                self.exit_code = code;
                self.halted = true;
                self.frames.clear();
            }
            Op::SleepMs(ms) => {
                std::thread::sleep(std::time::Duration::from_millis(ms));
            }
            Op::InputInt(name) => {
                let mut line = String::new();
                io::stdin()
                    .read_line(&mut line)
                    .map_err(|e| format!("stdin read failed: {e}"))?;
                let parsed = line
                    .trim()
                    .parse::<i64>()
                    .map_err(|e| format!("failed to parse integer input '{}': {e}", line.trim()))?;
                self.memory.insert(name, parsed);
            }
            Op::InputStr(name) => {
                let mut line = String::new();
                io::stdin()
                    .read_line(&mut line)
                    .map_err(|e| format!("stdin read failed: {e}"))?;
                self.strings.insert(name, line.trim().to_string());
            }
            Op::Nop => {}
            Op::Random(dst) => {
                // DCRNG - Decretum Custom Random Number Generator
                // ARX-based design: 320-bit state, 2-round diffusion, Weyl period extension

                // Round 1 - rotations (29, 17, 11, 7)
                self.rng_a = self.rng_a.wrapping_add(self.rng_b).rotate_left(29);
                self.rng_b ^= self.rng_c;
                self.rng_c = self.rng_c.wrapping_sub(self.rng_d).rotate_left(17);
                self.rng_d ^= self.rng_a;
                self.rng_a = self.rng_a.wrapping_add(self.rng_c).rotate_left(11);
                self.rng_b ^= self.rng_d;
                self.rng_d = self.rng_d.wrapping_sub(self.rng_b).rotate_left(7);
                self.rng_c ^= self.rng_a;

                // Round 2 - rotations (23, 19, 13, 5)
                self.rng_a = self.rng_a.wrapping_add(self.rng_b).rotate_left(23);
                self.rng_b ^= self.rng_c;
                self.rng_c = self.rng_c.wrapping_sub(self.rng_d).rotate_left(19);
                self.rng_d ^= self.rng_a;
                self.rng_a = self.rng_a.wrapping_add(self.rng_c).rotate_left(13);
                self.rng_b ^= self.rng_d;
                self.rng_d = self.rng_d.wrapping_sub(self.rng_b).rotate_left(5);
                self.rng_c ^= self.rng_a;

                // Weyl sequence - guarantees period, eliminates fixed points
                self.rng_weyl = self.rng_weyl.wrapping_add(0x9E3779B97F4A7C15);
                self.rng_a ^= self.rng_weyl;

                self.write_dest(&dst, self.rng_a.wrapping_add(self.rng_d) as i64);
            }
            Op::ReadFile(file_var, dest_var) => {
                let filename = self
                    .strings
                    .get(&file_var)
                    .cloned()
                    .unwrap_or(file_var.clone());
                let bytes = std::fs::read(&filename).unwrap_or_default();
                let content = unsafe { String::from_utf8_unchecked(bytes) };
                self.strings.insert(dest_var, content);
            }
            Op::WriteFile(file_var, src_var) => {
                let filename = self
                    .strings
                    .get(&file_var)
                    .cloned()
                    .unwrap_or(file_var.clone());
                let content = self.strings.get(&src_var).cloned().unwrap_or_default();
                let _ = std::fs::write(&filename, content.as_bytes());
            }
            Op::GetChar(str_var, idx_op, dest_op) => {
                let idx = self.read_operand(&idx_op)? as usize;
                let content = self.strings.get(&str_var).cloned().unwrap_or_default();
                let val = if idx < content.len() {
                    content.as_bytes()[idx] as i64
                } else {
                    0
                };
                self.write_dest(&dest_op, val);
            }
            Op::SetChar(str_var, idx_op, val_op) => {
                let idx = self.read_operand(&idx_op)? as usize;
                let val = self.read_operand(&val_op)? as u8;
                if let Some(content) = self.strings.get_mut(&str_var) {
                    let mut bytes = std::mem::take(content).into_bytes();
                    if idx < bytes.len() {
                        bytes[idx] = val;
                    }
                    *content = unsafe { String::from_utf8_unchecked(bytes) };
                }
            }
            Op::StrLen(str_var, dest_op) => {
                let content = self.strings.get(&str_var).cloned().unwrap_or_default();
                self.write_dest(&dest_op, content.len() as i64);
            }
            Op::StrAlloc(str_var, size_op) => {
                let size = self.read_operand(&size_op)? as usize;
                let content = unsafe { String::from_utf8_unchecked(vec![0; size]) };
                self.strings.insert(str_var, content);
            }
            Op::CompileDecretum(src_var, out_var, tgt_var) => {
                let src_content = self.strings.get(&src_var).cloned().unwrap_or(src_var);
                let out_path_str = self.strings.get(&out_var).cloned().unwrap_or(out_var);
                let tgt_str = self.strings.get(&tgt_var).cloned().unwrap_or(tgt_var);
                #[cfg(feature = "compiler-op")]
                {
                    let program = crate::Parser::parse(&src_content).map_err(|e| e.to_string())?;
                    let out_path = std::path::PathBuf::from(out_path_str);
                    if tgt_str == "pe" || tgt_str == "exe" || tgt_str == "win64" || tgt_str == "win32" {
                        crate::PortableBuilder::build_pe(&program, &out_path)?;
                    } else if tgt_str == "bytecode" {
                        crate::PortableBuilder::build_bytecode(&program, &out_path)?;
                    } else if tgt_str == "bios16" {
                        crate::DirectBiosBuilder::build_boot_image(&program, &out_path)?;
                    } else {
                        return Err(format!("unknown compilation target: {}", tgt_str));
                    }
                }
                #[cfg(not(feature = "compiler-op"))]
                {
                    let _ = src_content;
                    let _ = out_path_str;
                    let _ = tgt_str;
                    return Err(
                        "compile_decretum is unavailable in runtime-lite builds".to_string()
                    );
                }
            }
            Op::StrCat(dst, src1, src2) => {
                let s1 = self.strings.get(&src1).cloned().unwrap_or_default();
                let s2 = self.strings.get(&src2).cloned().unwrap_or_default();
                self.strings.insert(dst, s1 + &s2);
            }
            Op::StrFind(str_var, substr, result) => {
                let s = self.strings.get(&str_var).cloned().unwrap_or_default();
                let sub = self.strings.get(&substr).cloned().unwrap_or(substr);
                let pos = s.find(&sub).map(|p| p as i64).unwrap_or(-1);
                self.write_dest(&result, pos);
            }
            Op::Abs(dest, src) => {
                let val = self.read_operand(&src)?;
                self.write_dest(&dest, val.abs());
            }
            Op::Min(dest, a, b) => {
                let va = self.read_operand(&a)?;
                let vb = self.read_operand(&b)?;
                self.write_dest(&dest, va.min(vb));
            }
            Op::Max(dest, a, b) => {
                let va = self.read_operand(&a)?;
                let vb = self.read_operand(&b)?;
                self.write_dest(&dest, va.max(vb));
            }
            Op::TimeMs(dest) => {
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                self.write_dest(&dest, ms);
            }
            Op::Pow(dest, base, exp) => {
                let b = self.read_operand(&base)?;
                let e = self.read_operand(&exp)?;
                self.write_dest(&dest, b.wrapping_pow(e as u32));
            }
            Op::IntToStr(str_var, value) => {
                let val = self.read_operand(&value)?;
                self.strings.insert(str_var, val.to_string());
            }
            Op::StrToInt(str_var, result) => {
                let s = self.strings.get(&str_var).cloned().unwrap_or_default();
                let val = s.trim().parse::<i64>().unwrap_or(0);
                self.write_dest(&result, val);
            }
            Op::Clamp(dest, val, lo, hi) => {
                let v = self.read_operand(&val)?;
                let l = self.read_operand(&lo)?;
                let h = self.read_operand(&hi)?;
                self.write_dest(&dest, v.max(l).min(h));
            }
            Op::RandomRange(dest, lo, hi) => {
                let l = self.read_operand(&lo)?;
                let h = self.read_operand(&hi)?;
                let range = (h - l).abs().max(1);
                self.rng_weyl = self.rng_weyl.wrapping_add(0x9E3779B97F4A7C15);
                self.rng_a ^= self.rng_weyl;
                let r = (self.rng_a.wrapping_add(self.rng_d) as i64).abs();
                self.write_dest(&dest, l + (r % range));
            }
            Op::Assert(cond, msg) => {
                let val = self.read_operand(&cond)?;
                if val == 0 {
                    eprintln!("Assertion failed: {}", msg);
                    self.halted = true;
                    self.exit_code = 1;
                    self.frames.clear();
                }
            }
        }

        Ok(())
    }

    fn jump_to(&mut self, frame_index: usize, target: u32) -> Result<(), String> {
        let block_index = self.frames[frame_index].block_index;
        let block = self
            .program
            .blocks
            .get(block_index)
            .ok_or_else(|| format!("invalid frame block index {block_index}"))?;
        if target as usize >= block.ops.len() {
            return Err(format!(
                "jump target {} is out of bounds for block '{}'",
                target, block.name
            ));
        }
        self.frames[frame_index].ip = target as usize;
        Ok(())
    }

    fn matches_cond(&self, cond: Cond) -> bool {
        match cond {
            Cond::Eq => self.cmp_signed == 0,
            Cond::Ne => self.cmp_signed != 0,
            Cond::Lt => self.cmp_signed < 0,
            Cond::Le => self.cmp_signed <= 0,
            Cond::Gt => self.cmp_signed > 0,
            Cond::Ge => self.cmp_signed >= 0,
            Cond::Below => self.cmp_unsigned < 0,
            Cond::BelowEq => self.cmp_unsigned <= 0,
            Cond::Above => self.cmp_unsigned > 0,
            Cond::AboveEq => self.cmp_unsigned >= 0,
        }
    }

    fn read_dest(&self, dest: &Dest) -> i64 {
        match dest {
            Dest::Reg(reg) => self.regs[*reg as usize],
            Dest::Mem(name) => self.memory.get(name).copied().unwrap_or(0),
        }
    }

    fn write_dest(&mut self, dest: &Dest, value: i64) {
        match dest {
            Dest::Reg(reg) => {
                self.regs[*reg as usize] = value;
            }
            Dest::Mem(name) => {
                self.memory.insert(name.clone(), value);
            }
        }
    }

    fn read_operand(&self, operand: &Operand) -> Result<i64, String> {
        match operand {
            Operand::Imm(value) => Ok(*value),
            Operand::Reg(reg) => {
                let idx = *reg as usize;
                if idx >= self.regs.len() {
                    return Err(format!("register index out of bounds: {idx}"));
                }
                Ok(self.regs[idx])
            }
            Operand::Mem(name) => Ok(self.memory.get(name).copied().unwrap_or(0)),
        }
    }
}

pub fn dcrt_embed_magic() -> [u8; 16] {
    let mut m = [0u8; 16];
    m[0] = 0xDD + 1;
    m[1] = 0xBF + 1;
    m[2] = b'C' + 1;
    m[3] = b'B' + 1;
    m[4] = b'Q' + 1;
    m[5] = b'S' + 1;
    m[6] = b'D' + 1;
    m[7] = b'L' + 1;
    m[8] = b'A' + 1;
    m[9] = b'D' + 1;
    m[10] = b'C' + 1;
    m[11] = b'U' + 1;
    m[12] = b'0' + 1;
    m[13] = 0xFE + 1;
    m[14] = 0xFD + 1;
    m[15] = 0xFC + 1;
    m
}
