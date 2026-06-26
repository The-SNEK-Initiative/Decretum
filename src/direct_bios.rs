use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dcrt::{BlockKind, DataDecl, Program, ScalarWidth};

#[derive(Debug, Clone)]
pub struct BootImageOutput {
    pub image_path: PathBuf,
    pub kernel_path: PathBuf,
    pub sectors_loaded: u8,
}

pub struct DirectBiosBuilder;

impl DirectBiosBuilder {
    pub fn build_boot_image(program: &Program, out_path: &Path) -> Result<BootImageOutput, String> {
        if program.target != "bios16" {
            return Err(format!(
                "direct BIOS boot image requires target bios16, got '{}'",
                program.target
            ));
        }

        let kernel = DirectBiosAssembler::assemble(program)?;
        if kernel.is_empty() {
            return Err("empty kernel payload".to_string());
        }

        let sectors_needed = kernel.len().div_ceil(512);
        if sectors_needed == 0 || sectors_needed > 127 {
            return Err(format!(
                "kernel size {} bytes requires {} sectors, but BIOS CHS loader supports 1..127 sectors",
                kernel.len(),
                sectors_needed
            ));
        }
        let sectors_loaded =
            u8::try_from(sectors_needed).map_err(|_| "kernel sector count overflow".to_string())?;

        let mut image = build_boot_sector(sectors_loaded);
        image.extend_from_slice(&kernel);
        let pad = (512 - (image.len() % 512)) % 512;
        if pad > 0 {
            image.resize(image.len() + pad, 0);
        }

        let image_path = if out_path.is_absolute() {
            out_path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| format!("failed to get cwd: {e}"))?
                .join(out_path)
        };

        let kernel_path = image_path.with_extension("kernel.bin");
        if let Some(parent) = image_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }

        std::fs::write(&image_path, &image)
            .map_err(|e| format!("failed to write {}: {e}", image_path.display()))?;
        std::fs::write(&kernel_path, &kernel)
            .map_err(|e| format!("failed to write {}: {e}", kernel_path.display()))?;

        Ok(BootImageOutput {
            image_path,
            kernel_path,
            sectors_loaded,
        })
    }
}

fn build_boot_sector(sectors_to_load: u8) -> Vec<u8> {
    let mut boot = vec![
        0xFA, // cli
        0x31,
        0xC0, // xor ax, ax
        0x8E,
        0xD8, // mov ds, ax
        0x8E,
        0xC0, // mov es, ax
        0x8E,
        0xD0, // mov ss, ax
        0xBC,
        0x00,
        0x7C, // mov sp, 0x7C00
        0xBB,
        0x00,
        0x7E, // mov bx, 0x7E00 (stage2 load address)
        0xB4,
        0x02, // mov ah, 0x02 (disk read sectors)
        0xB0,
        sectors_to_load, // mov al, sectors_to_load
        0xB5,
        0x00, // mov ch, 0
        0xB1,
        0x02, // mov cl, 2 (first payload sector)
        0xB6,
        0x00, // mov dh, 0
        0xCD,
        0x13, // int 13h
        0x72,
        0x05, // jc disk_error
        0xEA,
        0x00,
        0x7E,
        0x00,
        0x00, // jmp 0000:7E00
        // disk_error:
        0xF4, // hlt
        0xEB,
        0xFD, // jmp disk_error
    ];

    boot.resize(510, 0);
    boot.push(0x55);
    boot.push(0xAA);
    boot
}

struct DirectBiosAssembler;

impl DirectBiosAssembler {
    fn assemble(program: &Program) -> Result<Vec<u8>, String> {
        let mut items = Vec::<Item>::new();

        let entry_label = format!("__event_{}", program.entry_event);
        items.push(Item::Label("__entry_start".to_string()));
        items.push(Item::Inst(Inst::Jmp(entry_label)));

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
            let block_label = format!("{prefix}{}", block.name);
            items.push(Item::Label(block_label.clone()));

            for raw_line in &block.lines {
                let line = raw_line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some(label) = line.strip_suffix(':') {
                    let full = qualify_label(label.trim(), &block_label);
                    items.push(Item::Label(full));
                    continue;
                }

                // if <reg>
                if let Some(cond_str) = line.strip_prefix("if ") {
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Item::Inst(Inst::MovReg16Imm(Reg16::Ax, Imm16::Value(n as i16))));
                            Reg16::Ax
                        }
                        Err(_) => {
                            parse_reg16(cond_str.trim())?
                        }
                    };
                    let endif_lbl = format!("__cf_{}_endif", cf_counter);
                    let else_lbl = format!("__cf_{}_else", cf_counter);
                    cf_counter += 1;
                    items.push(Item::Inst(Inst::CmpReg16Imm(reg, 0)));
                    let beqz_idx = items.len();
                    items.push(Item::Inst(Inst::Jcc(Cond::Z, endif_lbl.clone())));
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
                if let Some(cond_str) = line.strip_prefix("elif ") {
                    let frame = cf_stack.last_mut().ok_or("elif without if")?;
                    if frame.has_else {
                        return Err("elif after else".to_string());
                    }
                    let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.beqz_indices.len());
                    cf_counter += 1;
                    let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                    if let Item::Inst(Inst::Jcc(_, ref mut label)) = items[*prev] {
                        *label = elif_lbl.clone();
                    }
                    items.push(Item::Inst(Inst::Jmp(frame.endif_label.clone())));
                    items.push(Item::Label(elif_lbl));
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Item::Inst(Inst::MovReg16Imm(Reg16::Ax, Imm16::Value(n as i16))));
                            Reg16::Ax
                        }
                        Err(_) => {
                            parse_reg16(cond_str.trim())?
                        }
                    };
                    items.push(Item::Inst(Inst::CmpReg16Imm(reg, 0)));
                    let beqz_idx = items.len();
                    items.push(Item::Inst(Inst::Jcc(Cond::Z, frame.endif_label.clone())));
                    frame.beqz_indices.push(beqz_idx);
                    continue;
                }

                // else
                if line == "else" {
                    let frame = cf_stack.last_mut().ok_or("else without if")?;
                    if frame.has_else {
                        return Err("duplicate else".to_string());
                    }
                    frame.has_else = true;
                    let prev = frame.beqz_indices.last().ok_or("internal: no beqz indices")?;
                    if let Item::Inst(Inst::Jcc(_, ref mut label)) = items[*prev] {
                        *label = frame.else_label.clone();
                    }
                    items.push(Item::Inst(Inst::Jmp(frame.endif_label.clone())));
                    items.push(Item::Label(frame.else_label.clone()));
                    continue;
                }

                // endif
                if line == "endif" {
                    let frame = cf_stack.pop().ok_or("endif without if/while")?;
                    if frame.kind == CfKind::While {
                        return Err("endif without matching if".to_string());
                    }
                    items.push(Item::Label(frame.endif_label.clone()));
                    continue;
                }

                // while <reg>
                if let Some(cond_str) = line.strip_prefix("while ") {
                    let reg = match cond_str.trim().parse::<i64>() {
                        Ok(n) => {
                            items.push(Item::Inst(Inst::MovReg16Imm(Reg16::Ax, Imm16::Value(n as i16))));
                            Reg16::Ax
                        }
                        Err(_) => {
                            parse_reg16(cond_str.trim())?
                        }
                    };
                    let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                    let start_lbl = format!("__cf_{}_start", cf_counter);
                    cf_counter += 1;
                    items.push(Item::Label(start_lbl));
                    items.push(Item::Inst(Inst::CmpReg16Imm(reg, 0)));
                    let beqz_idx = items.len();
                    items.push(Item::Inst(Inst::Jcc(Cond::Z, endwhile_lbl.clone())));
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
                if line == "endwhile" {
                    let frame = cf_stack.pop().ok_or("endwhile without while")?;
                    if frame.kind != CfKind::While {
                        return Err("endwhile without matching while".to_string());
                    }
                    let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                    items.push(Item::Inst(Inst::Jmp(start_lbl)));
                    items.push(Item::Label(frame.endif_label.clone()));
                    continue;
                }

                let mut lowered = lower_line(line, &block_label)?;
                items.append(&mut lowered);
            }
        }

        if !cf_stack.is_empty() {
            return Err("unclosed if/while block".to_string());
        }

        // Peephole: NOP compression + dead code elimination
        {
            let mut i = 0;
            while i < items.len() {
                let is_nop = |item: &Item| matches!(item, Item::Inst(Inst::Nop));
                let is_term = |item: &Item| matches!(item, Item::Inst(Inst::Jmp(_)|Inst::Ret|Inst::Hlt|Inst::Int(_)));
                let is_label = |item: &Item| matches!(item, Item::Label(_));
                if i + 1 < items.len() && is_nop(&items[i]) && is_nop(&items[i+1]) { items.remove(i+1); continue; }
                if is_term(&items[i]) {
                    let mut j = i + 1;
                    while j < items.len() && !is_label(&items[j]) { j += 1; }
                    if j > i + 1 { items.drain(i+1..j); }
                }
                i += 1;
            }
        }

        emit_bios_runtime(&mut items);
        emit_user_data(program, &mut items);

        let layout = layout_items(&items)?;
        encode_items(&items, &layout)
    }
}

fn emit_bios_runtime(items: &mut Vec<Item>) {
    // Current text attribute used by putc.
    items.push(Item::Label("__builtin_text_attr".to_string()));
    items.push(Item::Bytes(vec![0x07]));

    // Print AL to active text page.
    items.push(Item::Label("__builtin_putc".to_string()));
    items.push(Item::Inst(Inst::Push(Reg16::Ax)));
    items.push(Item::Inst(Inst::Push(Reg16::Bx)));
    items.push(Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x0E)));
    items.push(Item::Inst(Inst::MovReg8Imm(Reg8::Bh, 0x00)));
    items.push(Item::Inst(Inst::MovReg8Mem(
        Reg8::Bl,
        Mem8::Label("__builtin_text_attr".to_string()),
    )));
    items.push(Item::Inst(Inst::Int(0x10)));
    items.push(Item::Inst(Inst::Pop(Reg16::Bx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Ax)));
    items.push(Item::Inst(Inst::Ret));

    // Print CRLF.
    items.push(Item::Label("__builtin_newline".to_string()));
    items.push(Item::Inst(Inst::Push(Reg16::Ax)));
    items.push(Item::Inst(Inst::MovReg8Imm(Reg8::Al, 0x0D)));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::MovReg8Imm(Reg8::Al, 0x0A)));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::Pop(Reg16::Ax)));
    items.push(Item::Inst(Inst::Ret));

    // Print NUL terminated string pointed by SI.
    items.push(Item::Label("__builtin_print_z".to_string()));
    items.push(Item::Inst(Inst::Push(Reg16::Ax)));
    items.push(Item::Inst(Inst::Push(Reg16::Si)));
    items.push(Item::Label("__builtin_print_z.loop".to_string()));
    items.push(Item::Inst(Inst::Lodsb));
    items.push(Item::Inst(Inst::OrReg8Reg8(Reg8::Al, Reg8::Al)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Z,
        "__builtin_print_z.done".to_string(),
    )));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::Jmp("__builtin_print_z.loop".to_string())));
    items.push(Item::Label("__builtin_print_z.done".to_string()));
    items.push(Item::Inst(Inst::Pop(Reg16::Si)));
    items.push(Item::Inst(Inst::Pop(Reg16::Ax)));
    items.push(Item::Inst(Inst::Ret));

    // Print unsigned AX in decimal.
    items.push(Item::Label("__builtin_print_u16_ax".to_string()));
    items.push(Item::Inst(Inst::Push(Reg16::Ax)));
    items.push(Item::Inst(Inst::Push(Reg16::Bx)));
    items.push(Item::Inst(Inst::Push(Reg16::Cx)));
    items.push(Item::Inst(Inst::Push(Reg16::Dx)));
    items.push(Item::Inst(Inst::CmpReg16Imm(Reg16::Ax, 0)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Nz,
        "__builtin_print_u16_ax.convert".to_string(),
    )));
    items.push(Item::Inst(Inst::MovReg8Imm(Reg8::Al, b'0')));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::Jmp(
        "__builtin_print_u16_ax.done".to_string(),
    )));
    items.push(Item::Label("__builtin_print_u16_ax.convert".to_string()));
    items.push(Item::Inst(Inst::XorReg16Reg16(Reg16::Cx, Reg16::Cx)));
    items.push(Item::Inst(Inst::MovReg16Imm(Reg16::Bx, Imm16::Value(10))));
    items.push(Item::Label(
        "__builtin_print_u16_ax.divide_loop".to_string(),
    ));
    items.push(Item::Inst(Inst::XorReg16Reg16(Reg16::Dx, Reg16::Dx)));
    items.push(Item::Inst(Inst::DivReg16(Reg16::Bx)));
    items.push(Item::Inst(Inst::Push(Reg16::Dx)));
    items.push(Item::Inst(Inst::Inc(Reg16::Cx)));
    items.push(Item::Inst(Inst::CmpReg16Imm(Reg16::Ax, 0)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Nz,
        "__builtin_print_u16_ax.divide_loop".to_string(),
    )));
    items.push(Item::Label("__builtin_print_u16_ax.print_loop".to_string()));
    items.push(Item::Inst(Inst::Pop(Reg16::Dx)));
    items.push(Item::Inst(Inst::MovReg16Reg16(Reg16::Ax, Reg16::Dx)));
    items.push(Item::Inst(Inst::AddReg16Imm(Reg16::Ax, b'0' as i16)));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::Dec(Reg16::Cx)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Nz,
        "__builtin_print_u16_ax.print_loop".to_string(),
    )));
    items.push(Item::Label("__builtin_print_u16_ax.done".to_string()));
    items.push(Item::Inst(Inst::Pop(Reg16::Dx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Cx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Bx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Ax)));
    items.push(Item::Inst(Inst::Ret));

    // Print unsigned AX in hexadecimal.
    items.push(Item::Label("__builtin_print_hex16_ax".to_string()));
    items.push(Item::Inst(Inst::Push(Reg16::Ax)));
    items.push(Item::Inst(Inst::Push(Reg16::Bx)));
    items.push(Item::Inst(Inst::Push(Reg16::Cx)));
    items.push(Item::Inst(Inst::Push(Reg16::Dx)));
    items.push(Item::Inst(Inst::XorReg16Reg16(Reg16::Cx, Reg16::Cx)));
    items.push(Item::Inst(Inst::MovReg16Imm(Reg16::Bx, Imm16::Value(16))));
    items.push(Item::Label(
        "__builtin_print_hex16_ax.divide_loop".to_string(),
    ));
    items.push(Item::Inst(Inst::XorReg16Reg16(Reg16::Dx, Reg16::Dx)));
    items.push(Item::Inst(Inst::DivReg16(Reg16::Bx)));
    items.push(Item::Inst(Inst::Push(Reg16::Dx)));
    items.push(Item::Inst(Inst::Inc(Reg16::Cx)));
    items.push(Item::Inst(Inst::CmpReg16Imm(Reg16::Ax, 0)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Nz,
        "__builtin_print_hex16_ax.divide_loop".to_string(),
    )));
    items.push(Item::Label(
        "__builtin_print_hex16_ax.print_loop".to_string(),
    ));
    items.push(Item::Inst(Inst::Pop(Reg16::Dx)));
    items.push(Item::Inst(Inst::MovReg16Reg16(Reg16::Ax, Reg16::Dx)));
    items.push(Item::Inst(Inst::CmpReg16Imm(Reg16::Ax, 9)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Cbe,
        "__builtin_print_hex16_ax.numeric".to_string(),
    )));
    items.push(Item::Inst(Inst::AddReg16Imm(Reg16::Ax, (b'A' - 10) as i16)));
    items.push(Item::Inst(Inst::Jmp(
        "__builtin_print_hex16_ax.emit".to_string(),
    )));
    items.push(Item::Label("__builtin_print_hex16_ax.numeric".to_string()));
    items.push(Item::Inst(Inst::AddReg16Imm(Reg16::Ax, b'0' as i16)));
    items.push(Item::Label("__builtin_print_hex16_ax.emit".to_string()));
    items.push(Item::Inst(Inst::Call("__builtin_putc".to_string())));
    items.push(Item::Inst(Inst::Dec(Reg16::Cx)));
    items.push(Item::Inst(Inst::Jcc(
        Cond::Nz,
        "__builtin_print_hex16_ax.print_loop".to_string(),
    )));
    items.push(Item::Inst(Inst::Pop(Reg16::Dx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Cx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Bx)));
    items.push(Item::Inst(Inst::Pop(Reg16::Ax)));
    items.push(Item::Inst(Inst::Ret));
}

fn emit_user_data(program: &Program, items: &mut Vec<Item>) {
    for data in &program.data {
        match data {
            DataDecl::String { name, value } => {
                items.push(Item::Label(name.clone()));
                let mut bytes = value.as_bytes().to_vec();
                bytes.push(0);
                items.push(Item::Bytes(bytes));
            }
            DataDecl::Scalar { name, width, value } => {
                items.push(Item::Label(name.clone()));
                let bytes = match width {
                    ScalarWidth::Byte => vec![(*value as i64 & 0xFF) as u8],
                    ScalarWidth::Word => (*value as i64 as i16).to_le_bytes().to_vec(),
                    ScalarWidth::Dword => (*value as i64 as i32).to_le_bytes().to_vec(),
                    ScalarWidth::Qword => (*value).to_le_bytes().to_vec(),
                };
                items.push(Item::Bytes(bytes));
            }
            DataDecl::Buffer { name, size } => {
                items.push(Item::Label(name.clone()));
                items.push(Item::Bytes(vec![0; *size]));
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Item {
    Label(String),
    Inst(Inst),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
enum Imm16 {
    Value(i16),
    Label(String),
}

#[derive(Debug, Clone)]
enum Mem8 {
    Label(String),
}

#[derive(Debug, Clone)]
enum Mem16 {
    Label(String),
}

#[derive(Debug, Clone)]
enum Inst {
    Cli,
    Sti,
    Cld,
    Std,
    Clc,
    Stc,
    Hlt,
    Nop,
    Ret,
    Lodsb,
    Int(u8),
    Jmp(String),
    Call(String),
    Jcc(Cond, String),
    Loop(String),
    Push(Reg16),
    Pop(Reg16),
    Pushf,
    Popf,
    Inc(Reg16),
    Dec(Reg16),
    MulReg16(Reg16),
    DivReg16(Reg16),
    MovReg16Imm(Reg16, Imm16),
    MovReg8Imm(Reg8, u8),
    MovSegReg(SegReg, Reg16),
    MovReg16Reg16(Reg16, Reg16),
    MovReg8Reg8(Reg8, Reg8),
    MovReg16Mem(Reg16, Mem16),
    MovMem16Reg(Mem16, Reg16),
    MovReg8Mem(Reg8, Mem8),
    MovMem8Reg(Mem8, Reg8),
    XorReg16Reg16(Reg16, Reg16),
    AndReg16Reg16(Reg16, Reg16),
    OrReg16Reg16(Reg16, Reg16),
    TestReg16Reg16(Reg16, Reg16),
    OrReg8Reg8(Reg8, Reg8),
    AddReg16Reg16(Reg16, Reg16),
    AddReg16Imm(Reg16, i16),
    SubReg16Reg16(Reg16, Reg16),
    SubReg16Imm(Reg16, i16),
    RepMovsb,
    RepStosb,
    CmpReg16Imm(Reg16, i16),
    CmpReg16Reg16(Reg16, Reg16),
}

#[derive(Debug, Clone, Copy)]
enum Cond {
    Z,
    Nz,
    C,
    Nc,
    Cbe,
    A,
    L,
    Le,
    G,
    Ge,
}

impl Cond {
    fn opcode(self) -> u8 {
        match self {
            Cond::Z => 0x74,
            Cond::Nz => 0x75,
            Cond::C => 0x72,
            Cond::Nc => 0x73,
            Cond::Cbe => 0x76,
            Cond::A => 0x77,
            Cond::L => 0x7C,
            Cond::Le => 0x7E,
            Cond::G => 0x7F,
            Cond::Ge => 0x7D,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reg16 {
    Ax,
    Cx,
    Dx,
    Bx,
    Sp,
    Bp,
    Si,
    Di,
}

impl Reg16 {
    fn code(self) -> u8 {
        match self {
            Reg16::Ax => 0,
            Reg16::Cx => 1,
            Reg16::Dx => 2,
            Reg16::Bx => 3,
            Reg16::Sp => 4,
            Reg16::Bp => 5,
            Reg16::Si => 6,
            Reg16::Di => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reg8 {
    Al,
    Cl,
    Dl,
    Bl,
    Ah,
    Ch,
    Dh,
    Bh,
}

impl Reg8 {
    fn code(self) -> u8 {
        match self {
            Reg8::Al => 0,
            Reg8::Cl => 1,
            Reg8::Dl => 2,
            Reg8::Bl => 3,
            Reg8::Ah => 4,
            Reg8::Ch => 5,
            Reg8::Dh => 6,
            Reg8::Bh => 7,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SegReg {
    Es,
    Cs,
    Ss,
    Ds,
}

impl SegReg {
    fn code(self) -> u8 {
        match self {
            SegReg::Es => 0,
            SegReg::Cs => 1,
            SegReg::Ss => 2,
            SegReg::Ds => 3,
        }
    }
}

fn qualify_label(raw: &str, scope: &str) -> String {
    let label = raw.trim();
    if label.starts_with('.') {
        format!("{scope}{label}")
    } else {
        label.to_string()
    }
}

fn lower_line(line: &str, scope: &str) -> Result<Vec<Item>, String> {
    if let Some(rest) = line.strip_prefix("emit ") {
        let name = rest.trim();
        if !is_ident(name) {
            return Err(format!("invalid event name '{name}'"));
        }
        return Ok(vec![Item::Inst(Inst::Call(format!("__event_{name}")))]);
    }

    if let Some(rest) = line.strip_prefix("call ") {
        let target = rest.trim();
        if is_ident(target) {
            return Ok(vec![Item::Inst(Inst::Call(format!("__proc_{target}")))]);
        }
    }

    if let Some(rest) = line.strip_prefix("builtin.print ") {
        let label = qualify_label(rest.trim(), scope);
        return Ok(vec![
            Item::Inst(Inst::MovReg16Imm(Reg16::Si, Imm16::Label(label))),
            Item::Inst(Inst::Call("__builtin_print_z".to_string())),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.println ") {
        let label = qualify_label(rest.trim(), scope);
        return Ok(vec![
            Item::Inst(Inst::MovReg16Imm(Reg16::Si, Imm16::Label(label))),
            Item::Inst(Inst::Call("__builtin_print_z".to_string())),
            Item::Inst(Inst::Call("__builtin_newline".to_string())),
        ]);
    }

    if line == "builtin.newline" {
        return Ok(vec![Item::Inst(Inst::Call(
            "__builtin_newline".to_string(),
        ))]);
    }

    if line == "builtin.clear_screen" {
        return Ok(vec![
            Item::Inst(Inst::MovReg16Imm(Reg16::Ax, Imm16::Value(0x0003))),
            Item::Inst(Inst::Int(0x10)),
        ]);
    }

    if line == "builtin.wait_key" || line == "builtin.get_key" {
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x00)),
            Item::Inst(Inst::Int(0x16)),
        ]);
    }

    if line == "builtin.beep" {
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Al, 0x07)),
            Item::Inst(Inst::Call("__builtin_putc".to_string())),
        ]);
    }

    if line == "builtin.disk_reset" {
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x00)),
            Item::Inst(Inst::Int(0x13)),
        ]);
    }

    if line == "builtin.reboot" {
        return Ok(vec![Item::Inst(Inst::Int(0x19))]);
    }

    if line == "builtin.halt" {
        return Ok(vec![Item::Inst(Inst::Hlt)]);
    }

    if let Some(rest) = line.strip_prefix("builtin.print_u16 ") {
        let value = parse_operand_u16(rest.trim(), scope)?;
        return Ok(vec![
            value,
            Item::Inst(Inst::Call("__builtin_print_u16_ax".to_string())),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.print_hex16 ") {
        let value = parse_operand_u16(rest.trim(), scope)?;
        return Ok(vec![
            value,
            Item::Inst(Inst::Call("__builtin_print_hex16_ax".to_string())),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.print_char ") {
        let value = parse_char_or_u8(rest.trim())?;
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Al, value)),
            Item::Inst(Inst::Call("__builtin_putc".to_string())),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.panic ") {
        let label = qualify_label(rest.trim(), scope);
        let hang = format!("{scope}.panic_hang");
        return Ok(vec![
            Item::Inst(Inst::MovReg16Imm(Reg16::Si, Imm16::Label(label))),
            Item::Inst(Inst::Call("__builtin_print_z".to_string())),
            Item::Inst(Inst::Call("__builtin_newline".to_string())),
            Item::Inst(Inst::Cli),
            Item::Label(hang.clone()),
            Item::Inst(Inst::Hlt),
            Item::Inst(Inst::Jmp(hang)),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.set_text_attr ") {
        let value = parse_char_or_u8(rest.trim())?;
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Al, value)),
            Item::Inst(Inst::MovMem8Reg(
                Mem8::Label("__builtin_text_attr".to_string()),
                Reg8::Al,
            )),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.set_cursor ") {
        let (row_raw, col_raw) = split2(rest)?;
        let row = parse_char_or_u8(row_raw)?;
        let col = parse_char_or_u8(col_raw)?;
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x02)),
            Item::Inst(Inst::MovReg8Imm(Reg8::Bh, 0x00)),
            Item::Inst(Inst::MovReg8Imm(Reg8::Dh, row)),
            Item::Inst(Inst::MovReg8Imm(Reg8::Dl, col)),
            Item::Inst(Inst::Int(0x10)),
        ]);
    }

    if let Some(rest) = line.strip_prefix("builtin.set_video_mode ") {
        let mode = parse_char_or_u8(rest.trim())?;
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x00)),
            Item::Inst(Inst::MovReg8Imm(Reg8::Al, mode)),
            Item::Inst(Inst::Int(0x10)),
        ]);
    }

    if line == "builtin.get_cursor" {
        return Ok(vec![
            Item::Inst(Inst::MovReg8Imm(Reg8::Ah, 0x03)),
            Item::Inst(Inst::MovReg8Imm(Reg8::Bh, 0x00)),
            Item::Inst(Inst::Int(0x10)),
        ]);
    }

    if line == "builtin.get_mem_kb" {
        return Ok(vec![Item::Inst(Inst::Int(0x12))]);
    }

    if let Some(rest) = line.strip_prefix("builtin.memcpy ") {
        let (dst_raw, rhs) = split2(rest)?;
        let (src_raw, count_raw) = split2(rhs)?;

        let mut out = Vec::new();
        out.push(Item::Inst(Inst::Push(Reg16::Si)));
        out.push(Item::Inst(Inst::Push(Reg16::Di)));
        out.push(Item::Inst(Inst::Push(Reg16::Cx)));
        out.extend(load_u16_operand_into_reg(dst_raw, Reg16::Di, scope)?);
        out.extend(load_u16_operand_into_reg(src_raw, Reg16::Si, scope)?);
        out.extend(load_u16_operand_into_reg(count_raw, Reg16::Cx, scope)?);
        out.push(Item::Inst(Inst::Cld));
        out.push(Item::Inst(Inst::RepMovsb));
        out.push(Item::Inst(Inst::Pop(Reg16::Cx)));
        out.push(Item::Inst(Inst::Pop(Reg16::Di)));
        out.push(Item::Inst(Inst::Pop(Reg16::Si)));
        return Ok(out);
    }

    if let Some(rest) = line.strip_prefix("builtin.memset ") {
        let (dst_raw, rhs) = split2(rest)?;
        let (value_raw, count_raw) = split2(rhs)?;
        let value = parse_char_or_u8(value_raw)?;

        let mut out = Vec::new();
        out.push(Item::Inst(Inst::Push(Reg16::Ax)));
        out.push(Item::Inst(Inst::Push(Reg16::Di)));
        out.push(Item::Inst(Inst::Push(Reg16::Cx)));
        out.extend(load_u16_operand_into_reg(dst_raw, Reg16::Di, scope)?);
        out.push(Item::Inst(Inst::MovReg8Imm(Reg8::Al, value)));
        out.extend(load_u16_operand_into_reg(count_raw, Reg16::Cx, scope)?);
        out.push(Item::Inst(Inst::Cld));
        out.push(Item::Inst(Inst::RepStosb));
        out.push(Item::Inst(Inst::Pop(Reg16::Cx)));
        out.push(Item::Inst(Inst::Pop(Reg16::Di)));
        out.push(Item::Inst(Inst::Pop(Reg16::Ax)));
        return Ok(out);
    }

    if line == "builtin.enter_protected_mode" {
        let mut out = Vec::new();
        // 1. Write GDT Null Descriptor at 0x7000
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x00, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x02, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x04, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x06, 0x70, 0x00, 0x00]));

        // 2. Write GDT Code Descriptor at 0x7008
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x08, 0x70, 0xFF, 0xFF]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x0A, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0C, 0x70, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0D, 0x70, 0x9A]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0E, 0x70, 0xCF]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0F, 0x70, 0x00]));

        // 3. Write GDT Data Descriptor at 0x7010
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x10, 0x70, 0xFF, 0xFF]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x12, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x14, 0x70, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x15, 0x70, 0x92]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x16, 0x70, 0xCF]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x17, 0x70, 0x00]));

        // 4. Write GDT Descriptor at 0x7040
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x40, 0x70, 0x17, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x42, 0x70, 0x00, 0x70]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x44, 0x70, 0x00, 0x00]));

        // 5. Load GDT
        out.push(Item::Bytes(vec![0x0F, 0x01, 0x16, 0x40, 0x70]));

        // 6. Switch to protected mode
        out.push(Item::Bytes(vec![0x0F, 0x20, 0xC0]));
        out.push(Item::Bytes(vec![0x66, 0x83, 0xC8, 0x01]));
        out.push(Item::Bytes(vec![0x0F, 0x22, 0xC0]));

        // 7. Far jump to 32-bit stage2 (Selector 0x08, address 0x9000)
        out.push(Item::Bytes(vec![0xEA, 0x00, 0x90, 0x08, 0x00]));

        return Ok(out);
    }

    if line == "builtin.enter_long_mode" {
        let mut out = Vec::new();
        // Write PML4 at 0x1000, PDPT at 0x2000, Page Dir at 0x3000
        // PML4[0] = 0x2003 (PDPT addr | flags 0x03)
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x00, 0x10, 0x03, 0x20]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x02, 0x10, 0x00, 0x00]));
        // PDPT[0] = 0x3003 (Page Dir addr | flags 0x03)
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x00, 0x20, 0x03, 0x30]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x02, 0x20, 0x00, 0x00]));
        // Page Dir[0] = 0x0083 (Identity map 2MB | flags 0x83)
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x00, 0x30, 0x83, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x02, 0x30, 0x00, 0x00]));

        // Write GDT Null Descriptor at 0x7000
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x00, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x02, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x04, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x06, 0x70, 0x00, 0x00]));

        // Write 64-bit Code Descriptor at 0x7008
        // Access: 0x9A, Flags: 0x20 (64-bit mode)
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x08, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x0A, 0x70, 0x00, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0C, 0x70, 0x00]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0D, 0x70, 0x9A]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0E, 0x70, 0x20]));
        out.push(Item::Bytes(vec![0xC6, 0x06, 0x0F, 0x70, 0x00]));

        // Write GDT Descriptor at 0x7040
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x40, 0x70, 0x0F, 0x00]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x42, 0x70, 0x00, 0x70]));
        out.push(Item::Bytes(vec![0xC7, 0x06, 0x44, 0x70, 0x00, 0x00]));

        // Enable PAE (CR4 bit 5)
        out.push(Item::Bytes(vec![0x0F, 0x20, 0xE0])); // mov eax, cr4
        out.push(Item::Bytes(vec![0x66, 0x83, 0xC8, 0x20])); // or eax, 0x20
        out.push(Item::Bytes(vec![0x0F, 0x22, 0xE0])); // mov cr4, eax

        // Load PML4 address (CR3 = 0x1000)
        out.push(Item::Bytes(vec![0xB8, 0x00, 0x10])); // mov ax, 0x1000
        out.push(Item::Bytes(vec![0x0F, 0x22, 0xD8])); // mov cr3, eax

        // Enable Long Mode in EFER MSR
        out.push(Item::Bytes(vec![0xB9, 0x80, 0x00])); // mov cx, 0xC0000080 (lower 16 bits)
        out.push(Item::Bytes(vec![0x0F, 0x32])); // rdmsr
        out.push(Item::Bytes(vec![0x66, 0x0D, 0x00, 0x01, 0x00, 0x00])); // or eax, 0x100
        out.push(Item::Bytes(vec![0x0F, 0x30])); // wrmsr

        // Enable Paging and Protected Mode (CR0 bits 31 and 0)
        out.push(Item::Bytes(vec![0x0F, 0x20, 0xC0])); // mov eax, cr0
        out.push(Item::Bytes(vec![0x66, 0x0D, 0x01, 0x00, 0x00, 0x80])); // or eax, 0x80000001
        out.push(Item::Bytes(vec![0x0F, 0x22, 0xC0])); // mov cr0, eax

        // Load GDT
        out.push(Item::Bytes(vec![0x0F, 0x01, 0x16, 0x40, 0x70]));

        // Far jump to 64-bit stage2 (Selector 0x08, address 0x9000)
        out.push(Item::Bytes(vec![0xEA, 0x00, 0x90, 0x08, 0x00]));

        return Ok(out);
    }

    Ok(vec![Item::Inst(parse_instruction(line, scope)?)])
}

fn parse_operand_u16(raw: &str, scope: &str) -> Result<Item, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("missing operand".to_string());
    }
    if let Ok(reg) = parse_reg16(text) {
        return Ok(Item::Inst(Inst::MovReg16Reg16(Reg16::Ax, reg)));
    }
    if is_ident(text) || text.starts_with('.') {
        return Ok(Item::Inst(Inst::MovReg16Imm(
            Reg16::Ax,
            Imm16::Label(qualify_label(text, scope)),
        )));
    }
    let value = parse_int(text)?;
    if !(-32768..=65535).contains(&value) {
        return Err(format!("16-bit immediate out of range: {value}"));
    }
    Ok(Item::Inst(Inst::MovReg16Imm(
        Reg16::Ax,
        Imm16::Value(value as i16),
    )))
}

fn load_u16_operand_into_reg(raw: &str, dst: Reg16, scope: &str) -> Result<Vec<Item>, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("missing operand".to_string());
    }
    if let Ok(src) = parse_reg16(text) {
        if src == dst {
            return Ok(Vec::new());
        }
        return Ok(vec![Item::Inst(Inst::MovReg16Reg16(dst, src))]);
    }
    if is_ident(text) || text.starts_with('.') {
        return Ok(vec![Item::Inst(Inst::MovReg16Imm(
            dst,
            Imm16::Label(qualify_label(text, scope)),
        ))]);
    }
    let value = parse_int(text)?;
    if !(-32768..=65535).contains(&value) {
        return Err(format!("16-bit immediate out of range: {value}"));
    }
    Ok(vec![Item::Inst(Inst::MovReg16Imm(
        dst,
        Imm16::Value(value as i16),
    ))])
}

fn parse_instruction(line: &str, scope: &str) -> Result<Inst, String> {
    let (mnemonic, rest) = if let Some((m, r)) = line.split_once(' ') {
        (m.trim().to_ascii_lowercase(), r.trim())
    } else {
        (line.trim().to_ascii_lowercase(), "")
    };

    match mnemonic.as_str() {
        "cli" => Ok(Inst::Cli),
        "sti" => Ok(Inst::Sti),
        "cld" => Ok(Inst::Cld),
        "std" => Ok(Inst::Std),
        "clc" => Ok(Inst::Clc),
        "stc" => Ok(Inst::Stc),
        "hlt" => Ok(Inst::Hlt),
        "nop" => Ok(Inst::Nop),
        "ret" => Ok(Inst::Ret),
        "pushf" => Ok(Inst::Pushf),
        "popf" => Ok(Inst::Popf),
        "lodsb" => Ok(Inst::Lodsb),
        "int" => {
            let value = parse_int(rest)?;
            if !(0..=255).contains(&value) {
                return Err(format!("int immediate out of range: {value}"));
            }
            Ok(Inst::Int(value as u8))
        }
        "jmp" => Ok(Inst::Jmp(qualify_label(rest, scope))),
        "call" => Ok(Inst::Call(qualify_label(rest, scope))),
        "loop" => Ok(Inst::Loop(qualify_label(rest, scope))),
        "je" | "jz" => Ok(Inst::Jcc(Cond::Z, qualify_label(rest, scope))),
        "jne" | "jnz" => Ok(Inst::Jcc(Cond::Nz, qualify_label(rest, scope))),
        "jc" | "jb" => Ok(Inst::Jcc(Cond::C, qualify_label(rest, scope))),
        "jnc" | "jae" | "jnb" => Ok(Inst::Jcc(Cond::Nc, qualify_label(rest, scope))),
        "jbe" => Ok(Inst::Jcc(Cond::Cbe, qualify_label(rest, scope))),
        "ja" => Ok(Inst::Jcc(Cond::A, qualify_label(rest, scope))),
        "jl" => Ok(Inst::Jcc(Cond::L, qualify_label(rest, scope))),
        "jle" => Ok(Inst::Jcc(Cond::Le, qualify_label(rest, scope))),
        "jg" => Ok(Inst::Jcc(Cond::G, qualify_label(rest, scope))),
        "jge" => Ok(Inst::Jcc(Cond::Ge, qualify_label(rest, scope))),
        "push" => Ok(Inst::Push(parse_reg16(rest)?)),
        "pop" => Ok(Inst::Pop(parse_reg16(rest)?)),
        "inc" => Ok(Inst::Inc(parse_reg16(rest)?)),
        "dec" => Ok(Inst::Dec(parse_reg16(rest)?)),
        "mul" => Ok(Inst::MulReg16(parse_reg16(rest)?)),
        "div" => Ok(Inst::DivReg16(parse_reg16(rest)?)),
        "rep_movsb" => Ok(Inst::RepMovsb),
        "rep_stosb" => Ok(Inst::RepStosb),
        "mov" => parse_mov(rest, scope),
        "xor" => {
            let (a, b) = split2(rest)?;
            Ok(Inst::XorReg16Reg16(parse_reg16(a)?, parse_reg16(b)?))
        }
        "and" => {
            let (a, b) = split2(rest)?;
            Ok(Inst::AndReg16Reg16(parse_reg16(a)?, parse_reg16(b)?))
        }
        "add" => {
            let (a, b) = split2(rest)?;
            if let Ok(reg_rhs) = parse_reg16(b) {
                Ok(Inst::AddReg16Reg16(parse_reg16(a)?, reg_rhs))
            } else {
                Ok(Inst::AddReg16Imm(parse_reg16(a)?, parse_int(b)? as i16))
            }
        }
        "sub" => {
            let (a, b) = split2(rest)?;
            if let Ok(reg_rhs) = parse_reg16(b) {
                Ok(Inst::SubReg16Reg16(parse_reg16(a)?, reg_rhs))
            } else {
                Ok(Inst::SubReg16Imm(parse_reg16(a)?, parse_int(b)? as i16))
            }
        }
        "cmp" => {
            let (a, b) = split2(rest)?;
            let lhs = parse_reg16(a)?;
            if let Ok(rhs_reg) = parse_reg16(b) {
                Ok(Inst::CmpReg16Reg16(lhs, rhs_reg))
            } else {
                Ok(Inst::CmpReg16Imm(lhs, parse_int(b)? as i16))
            }
        }
        "or" => {
            let (a, b) = split2(rest)?;
            if let (Ok(r16a), Ok(r16b)) = (parse_reg16(a), parse_reg16(b)) {
                Ok(Inst::OrReg16Reg16(r16a, r16b))
            } else if let (Ok(r8a), Ok(r8b)) = (parse_reg8(a), parse_reg8(b)) {
                Ok(Inst::OrReg8Reg8(r8a, r8b))
            } else {
                Err(format!("unsupported or operands '{a}', '{b}'"))
            }
        }
        "test" => {
            let (a, b) = split2(rest)?;
            Ok(Inst::TestReg16Reg16(parse_reg16(a)?, parse_reg16(b)?))
        }
        _ => Err(format!("unsupported bios16 instruction '{mnemonic}'")),
    }
}

fn parse_mov(rest: &str, scope: &str) -> Result<Inst, String> {
    let (dst, src) = split2(rest)?;
    if let (Ok(seg), Ok(reg16)) = (parse_seg(dst), parse_reg16(src)) {
        return Ok(Inst::MovSegReg(seg, reg16));
    }
    if let (Ok(r16d), Ok(r16s)) = (parse_reg16(dst), parse_reg16(src)) {
        return Ok(Inst::MovReg16Reg16(r16d, r16s));
    }
    if let (Ok(r8d), Ok(r8s)) = (parse_reg8(dst), parse_reg8(src)) {
        return Ok(Inst::MovReg8Reg8(r8d, r8s));
    }
    if let (Ok(r16d), Some(mem)) = (parse_reg16(dst), parse_mem16(src, scope)) {
        return Ok(Inst::MovReg16Mem(r16d, mem));
    }
    if let (Some(mem), Ok(r16s)) = (parse_mem16(dst, scope), parse_reg16(src)) {
        return Ok(Inst::MovMem16Reg(mem, r16s));
    }
    if let (Ok(r8d), Some(mem)) = (parse_reg8(dst), parse_mem8(src, scope)) {
        return Ok(Inst::MovReg8Mem(r8d, mem));
    }
    if let (Some(mem), Ok(r8s)) = (parse_mem8(dst, scope), parse_reg8(src)) {
        return Ok(Inst::MovMem8Reg(mem, r8s));
    }
    if let Ok(r8d) = parse_reg8(dst) {
        let imm = parse_int(src)?;
        if !(0..=255).contains(&imm) && !(-128..=255).contains(&imm) {
            return Err(format!("8-bit immediate out of range: {imm}"));
        }
        return Ok(Inst::MovReg8Imm(r8d, imm as u8));
    }
    if let Ok(r16d) = parse_reg16(dst) {
        if is_ident(src) || src.starts_with('.') {
            return Ok(Inst::MovReg16Imm(
                r16d,
                Imm16::Label(qualify_label(src, scope)),
            ));
        }
        let imm = parse_int(src)?;
        return Ok(Inst::MovReg16Imm(r16d, Imm16::Value(imm as i16)));
    }
    Err(format!("unsupported mov operands: '{dst}', '{src}'"))
}

fn parse_mem8(raw: &str, scope: &str) -> Option<Mem8> {
    let trimmed = raw.trim();
    let core = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    if core.is_empty() {
        return None;
    }
    if is_ident(core) || core.starts_with('.') {
        return Some(Mem8::Label(qualify_label(core, scope)));
    }
    None
}

fn parse_mem16(raw: &str, scope: &str) -> Option<Mem16> {
    let trimmed = raw.trim();
    let core = trimmed.strip_prefix('[')?.strip_suffix(']')?.trim();
    if core.is_empty() {
        return None;
    }
    if is_ident(core) || core.starts_with('.') {
        return Some(Mem16::Label(qualify_label(core, scope)));
    }
    None
}

fn split2(rest: &str) -> Result<(&str, &str), String> {
    let (a, b) = rest
        .split_once(',')
        .ok_or_else(|| format!("expected two operands in '{rest}'"))?;
    Ok((a.trim(), b.trim()))
}

fn parse_reg16(raw: &str) -> Result<Reg16, String> {
    match raw.to_ascii_lowercase().as_str() {
        "ax" => Ok(Reg16::Ax),
        "cx" => Ok(Reg16::Cx),
        "dx" => Ok(Reg16::Dx),
        "bx" => Ok(Reg16::Bx),
        "sp" => Ok(Reg16::Sp),
        "bp" => Ok(Reg16::Bp),
        "si" => Ok(Reg16::Si),
        "di" => Ok(Reg16::Di),
        _ => Err(format!("not a 16-bit register: '{raw}'")),
    }
}

fn parse_reg8(raw: &str) -> Result<Reg8, String> {
    match raw.to_ascii_lowercase().as_str() {
        "al" => Ok(Reg8::Al),
        "cl" => Ok(Reg8::Cl),
        "dl" => Ok(Reg8::Dl),
        "bl" => Ok(Reg8::Bl),
        "ah" => Ok(Reg8::Ah),
        "ch" => Ok(Reg8::Ch),
        "dh" => Ok(Reg8::Dh),
        "bh" => Ok(Reg8::Bh),
        _ => Err(format!("not an 8-bit register: '{raw}'")),
    }
}

fn parse_seg(raw: &str) -> Result<SegReg, String> {
    match raw.to_ascii_lowercase().as_str() {
        "es" => Ok(SegReg::Es),
        "cs" => Ok(SegReg::Cs),
        "ss" => Ok(SegReg::Ss),
        "ds" => Ok(SegReg::Ds),
        _ => Err(format!("not a segment register: '{raw}'")),
    }
}

fn parse_int(raw: &str) -> Result<i32, String> {
    let text = raw.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).map_err(|_| format!("invalid hex integer '{text}'"))
    } else if let Some(hex) = text.strip_prefix("-0x") {
        i32::from_str_radix(hex, 16)
            .map(|v| -v)
            .map_err(|_| format!("invalid hex integer '{text}'"))
    } else {
        text.parse::<i32>()
            .map_err(|_| format!("invalid integer '{text}'"))
    }
}

fn parse_char_or_u8(raw: &str) -> Result<u8, String> {
    let trimmed = raw.trim();
    if let Some(content) = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        let ch = parse_char_literal(content)?;
        return Ok(ch as u8);
    }
    let value = parse_int(trimmed)?;
    if !(0..=255).contains(&value) {
        return Err(format!("8-bit immediate out of range: {value}"));
    }
    Ok(value as u8)
}

fn parse_char_literal(raw: &str) -> Result<char, String> {
    let mut chars = raw.chars();
    let first = chars
        .next()
        .ok_or_else(|| "empty character literal".to_string())?;
    if first == '\\' {
        let escaped = chars
            .next()
            .ok_or_else(|| "unfinished escape sequence in character literal".to_string())?;
        if chars.next().is_some() {
            return Err(format!("invalid character literal '{raw}'"));
        }
        return match escaped {
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            '0' => Ok('\0'),
            '\\' => Ok('\\'),
            '\'' => Ok('\''),
            '"' => Ok('"'),
            other => Err(format!("unsupported escape sequence '\\{other}'")),
        };
    }
    if chars.next().is_some() {
        return Err(format!("invalid character literal '{raw}'"));
    }
    Ok(first)
}

fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric() || c == '.')
}

fn inst_len(inst: &Inst) -> usize {
    match inst {
        Inst::Cli
        | Inst::Sti
        | Inst::Cld
        | Inst::Std
        | Inst::Clc
        | Inst::Stc
        | Inst::Hlt
        | Inst::Nop
        | Inst::Ret
        | Inst::Lodsb
        | Inst::Push(_)
        | Inst::Pop(_)
        | Inst::Pushf
        | Inst::Popf
        | Inst::Inc(_)
        | Inst::Dec(_) => 1,
        Inst::Int(_) => 2,
        Inst::Jmp(_) | Inst::Call(_) => 3,
        Inst::Jcc(_, _) | Inst::Loop(_) => 2,
        Inst::RepMovsb | Inst::RepStosb => 2,
        Inst::MulReg16(_) | Inst::DivReg16(_) => 2,
        Inst::MovReg16Imm(_, _) => 3,
        Inst::MovReg8Imm(_, _) => 2,
        Inst::MovSegReg(_, _)
        | Inst::MovReg16Reg16(_, _)
        | Inst::MovReg16Mem(_, _)
        | Inst::MovMem16Reg(_, _)
        | Inst::MovReg8Reg8(_, _)
        | Inst::XorReg16Reg16(_, _)
        | Inst::AndReg16Reg16(_, _)
        | Inst::OrReg16Reg16(_, _)
        | Inst::TestReg16Reg16(_, _)
        | Inst::AddReg16Reg16(_, _)
        | Inst::SubReg16Reg16(_, _)
        | Inst::CmpReg16Reg16(_, _)
        | Inst::OrReg8Reg8(_, _) => 2,
        Inst::MovReg8Mem(_, _) | Inst::MovMem8Reg(_, _) => 4,
        Inst::AddReg16Imm(_, v) | Inst::SubReg16Imm(_, v) | Inst::CmpReg16Imm(_, v) => {
            if (-128..=127).contains(v) { 3 } else { 4 }
        }
    }
}

fn layout_items(items: &[Item]) -> Result<BTreeMap<String, u16>, String> {
    let mut labels = BTreeMap::<String, u16>::new();
    let mut offset = 0usize;
    for item in items {
        match item {
            Item::Label(name) => {
                if labels.insert(name.clone(), offset as u16).is_some() {
                    return Err(format!("duplicate label '{name}'"));
                }
            }
            Item::Inst(inst) => offset += inst_len(inst),
            Item::Bytes(bytes) => offset += bytes.len(),
        }
    }
    if offset > 0xFFFF {
        return Err(format!("bios16 stage2 too large: {offset} bytes"));
    }
    Ok(labels)
}

fn encode_items(items: &[Item], labels: &BTreeMap<String, u16>) -> Result<Vec<u8>, String> {
    let mut out = Vec::<u8>::new();
    let mut offset = 0usize;

    for item in items {
        match item {
            Item::Label(_) => {}
            Item::Bytes(bytes) => {
                out.extend_from_slice(bytes);
                offset += bytes.len();
            }
            Item::Inst(inst) => {
                let encoded = encode_inst(inst, offset as u16, labels)?;
                offset += encoded.len();
                out.extend_from_slice(&encoded);
            }
        }
    }
    Ok(out)
}

fn encode_inst(inst: &Inst, at: u16, labels: &BTreeMap<String, u16>) -> Result<Vec<u8>, String> {
    let bytes = match inst {
        Inst::Cli => vec![0xFA],
        Inst::Sti => vec![0xFB],
        Inst::Cld => vec![0xFC],
        Inst::Std => vec![0xFD],
        Inst::Clc => vec![0xF8],
        Inst::Stc => vec![0xF9],
        Inst::Hlt => vec![0xF4],
        Inst::Nop => vec![0x90],
        Inst::Ret => vec![0xC3],
        Inst::Lodsb => vec![0xAC],
        Inst::Pushf => vec![0x9C],
        Inst::Popf => vec![0x9D],
        Inst::Int(n) => vec![0xCD, *n],
        Inst::Push(r) => vec![0x50 + r.code()],
        Inst::Pop(r) => vec![0x58 + r.code()],
        Inst::Inc(r) => vec![0x40 + r.code()],
        Inst::Dec(r) => vec![0x48 + r.code()],
        Inst::RepMovsb => vec![0xF3, 0xA4],
        Inst::RepStosb => vec![0xF3, 0xAA],
        Inst::MulReg16(reg) => vec![0xF7, 0xE0 | reg.code()],
        Inst::DivReg16(reg) => vec![0xF7, 0xF0 | reg.code()],
        Inst::MovReg16Imm(reg, imm) => {
            let value = match imm {
                Imm16::Value(v) => *v as u16,
                Imm16::Label(name) => *labels
                    .get(name)
                    .ok_or_else(|| format!("unknown label '{name}'"))?,
            };
            let mut v = vec![0xB8 + reg.code()];
            v.extend_from_slice(&value.to_le_bytes());
            v
        }
        Inst::MovReg8Imm(reg, imm) => vec![0xB0 + reg.code(), *imm],
        Inst::MovSegReg(seg, reg) => vec![0x8E, 0xC0 | (seg.code() << 3) | reg.code()],
        Inst::MovReg16Reg16(dst, src) => vec![0x89, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::MovReg16Mem(dst, mem) => match mem {
            Mem16::Label(name) => {
                let addr = *labels
                    .get(name)
                    .ok_or_else(|| format!("unknown label '{name}'"))?;
                let mut v = vec![0x8B, 0x06 | (dst.code() << 3)];
                v.extend_from_slice(&addr.to_le_bytes());
                v
            }
        },
        Inst::MovMem16Reg(mem, src) => match mem {
            Mem16::Label(name) => {
                let addr = *labels
                    .get(name)
                    .ok_or_else(|| format!("unknown label '{name}'"))?;
                let mut v = vec![0x89, 0x06 | (src.code() << 3)];
                v.extend_from_slice(&addr.to_le_bytes());
                v
            }
        },
        Inst::MovReg8Reg8(dst, src) => vec![0x88, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::MovReg8Mem(dst, mem) => match mem {
            Mem8::Label(name) => {
                let addr = *labels
                    .get(name)
                    .ok_or_else(|| format!("unknown label '{name}'"))?;
                let mut v = vec![0x8A, 0x06 | (dst.code() << 3)];
                v.extend_from_slice(&addr.to_le_bytes());
                v
            }
        },
        Inst::MovMem8Reg(mem, src) => match mem {
            Mem8::Label(name) => {
                let addr = *labels
                    .get(name)
                    .ok_or_else(|| format!("unknown label '{name}'"))?;
                let mut v = vec![0x88, 0x06 | (src.code() << 3)];
                v.extend_from_slice(&addr.to_le_bytes());
                v
            }
        },
        Inst::XorReg16Reg16(dst, src) => vec![0x31, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::AndReg16Reg16(dst, src) => vec![0x21, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::OrReg16Reg16(dst, src) => vec![0x09, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::TestReg16Reg16(dst, src) => vec![0x85, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::CmpReg16Reg16(dst, src) => vec![0x39, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::OrReg8Reg8(dst, src) => vec![0x08, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::AddReg16Reg16(dst, src) => vec![0x01, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::AddReg16Imm(dst, imm) => encode_r16_imm(*dst, *imm, 0x00),
        Inst::SubReg16Reg16(dst, src) => vec![0x29, 0xC0 | (src.code() << 3) | dst.code()],
        Inst::SubReg16Imm(dst, imm) => encode_r16_imm(*dst, *imm, 0x05),
        Inst::CmpReg16Imm(dst, imm) => encode_r16_imm(*dst, *imm, 0x07),
        Inst::Jmp(target) => {
            let next = at.wrapping_add(3);
            let target_off = *labels
                .get(target)
                .ok_or_else(|| format!("unknown label '{target}'"))?;
            let rel = (target_off as i32) - (next as i32);
            let rel16 = i16::try_from(rel)
                .map_err(|_| format!("jump target out of range for '{target}'"))?;
            let mut v = vec![0xE9];
            v.extend_from_slice(&rel16.to_le_bytes());
            v
        }
        Inst::Call(target) => {
            let next = at.wrapping_add(3);
            let target_off = *labels
                .get(target)
                .ok_or_else(|| format!("unknown label '{target}'"))?;
            let rel = (target_off as i32) - (next as i32);
            let rel16 = i16::try_from(rel)
                .map_err(|_| format!("call target out of range for '{target}'"))?;
            let mut v = vec![0xE8];
            v.extend_from_slice(&rel16.to_le_bytes());
            v
        }
        Inst::Jcc(cond, target) => {
            let next = at.wrapping_add(2);
            let target_off = *labels
                .get(target)
                .ok_or_else(|| format!("unknown label '{target}'"))?;
            let rel = (target_off as i32) - (next as i32);
            let rel8 = i8::try_from(rel).map_err(|_| {
                format!("conditional jump target out of short range for '{target}'")
            })?;
            vec![cond.opcode(), rel8 as u8]
        }
        Inst::Loop(target) => {
            let next = at.wrapping_add(2);
            let target_off = *labels
                .get(target)
                .ok_or_else(|| format!("unknown label '{target}'"))?;
            let rel = (target_off as i32) - (next as i32);
            let rel8 = i8::try_from(rel)
                .map_err(|_| format!("loop target out of short range for '{target}'"))?;
            vec![0xE2, rel8 as u8]
        }
    };
    Ok(bytes)
}

fn encode_r16_imm(dst: Reg16, imm: i16, subcode: u8) -> Vec<u8> {
    if (-128..=127).contains(&imm) {
        vec![0x83, 0xC0 | (subcode << 3) | dst.code(), imm as u8]
    } else {
        let mut v = vec![0x81, 0xC0 | (subcode << 3) | dst.code()];
        v.extend_from_slice(&imm.to_le_bytes());
        v
    }
}
