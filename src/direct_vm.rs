// Simple stack based virtual machine
// Bytecode format: [opcode, operands...]
// Interpreter in Rust (can also be embedded)

use std::path::{Path, PathBuf};
use crate::dcrt::{BlockKind, Program};

pub struct VmBuildOutput {
    pub bytecode_path: PathBuf,
    pub bytecode: Vec<u8>,
    pub op_count: usize,
}

pub struct DirectVmBuilder;

impl DirectVmBuilder {
    pub fn build_bytecode(program: &Program, out_path: &Path) -> Result<VmBuildOutput, String> {
        if program.target != "vm" {
            return Err(format!("vm backend requires target 'vm', got '{}'", program.target));
        }
        let (bytecode, op_count) = compile_vm(program)?;
        std::fs::write(out_path, &bytecode).map_err(|e| format!("write failed: {e}"))?;
        Ok(VmBuildOutput { bytecode_path: out_path.to_path_buf(), bytecode, op_count })
    }
}

// VM opcodes
const OP_PUSH: u8 = 0;    // push imm64
const OP_DUP: u8 = 1;     // dup
const OP_DROP: u8 = 2;    // drop
const OP_SWAP: u8 = 3;    // swap
const OP_ADD: u8 = 4;
const OP_SUB: u8 = 5;
const OP_MUL: u8 = 6;
const OP_DIV: u8 = 7;
const OP_MOD: u8 = 8;
const OP_NEG: u8 = 9;
const OP_CMP: u8 = 10;    // push -1/0/1 (less/equal/greater)
const OP_JMP: u8 = 11;    // jmp u32 offset
const OP_JZ: u8 = 12;     // pop; jz u32 offset
const OP_JNZ: u8 = 13;    // pop; jnz u32 offset
const OP_CALL: u8 = 14;   // call u32 target, push return addr
const OP_RET: u8 = 15;    // ret
const OP_PRINT: u8 = 16;  // pop and print as decimal
const OP_PRINTC: u8 = 17; // pop and print as char
const OP_READ: u8 = 18;   // read integer from stdin, push
const OP_EXIT: u8 = 19;   // exit
const OP_NOP: u8 = 20;
const OP_LOAD: u8 = 21;   // load from memory at address on stack
const OP_STORE: u8 = 22;  // store to memory at address on stack
const OP_ALLOC: u8 = 23;  // allocate N cells
const OP_PUSHS: u8 = 24;  // push string (inline utf8, null-terminated)

fn write_u32(buf: &mut Vec<u8>, v: u32) { buf.extend_from_slice(&v.to_le_bytes()); }
fn write_u64(buf: &mut Vec<u8>, v: u64) { buf.extend_from_slice(&v.to_le_bytes()); }

fn compile_vm(program: &Program) -> Result<(Vec<u8>, usize), String> {
    let mut bc = Vec::new();
    bc.extend_from_slice(b"DECVM01");
    write_u32(&mut bc, 0);

    struct CfFrame {
        kind: CfKind,
        endif_label: String,
        else_label: String,
        br_positions: Vec<u32>,
        has_else: bool,
    }
    enum CfKind { If, While }
    let mut cf_stack: Vec<CfFrame> = Vec::new();
    let mut cf_counter: u32 = 0;

    let mut labels: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut pending_fixups: Vec<(u32, String)> = Vec::new();
    let mut op_count = 0;

    let entry_name = format!("__event_{}", program.entry_event);
    bc.push(OP_CALL);
    write_u32(&mut bc, 0);
    pending_fixups.push(((bc.len() - 4) as u32, entry_name.clone()));
    bc.push(OP_EXIT);
    labels.insert("__entry".to_string(), 0);

    for block in &program.blocks {
        let prefix = match block.kind {
            BlockKind::Event => "__event_",
            BlockKind::Proc => "__proc_",
        };
        let block_label = format!("{}{}", prefix, block.name);
        labels.insert(block_label, bc.len() as u32);

        for line in &block.lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') { continue; }
            if trimmed.ends_with(':') {
                let label = format!("{}.{}", block.name, trimmed[..trimmed.len()-1].trim());
                labels.insert(label, bc.len() as u32);
                continue;
            }
            if let Some(target) = trimmed.strip_prefix("emit ") {
                bc.push(OP_CALL);
                write_u32(&mut bc, 0);
                pending_fixups.push(((bc.len() - 4) as u32, format!("__event_{}", target.trim())));
                op_count += 1;
                continue;
            }
            if let Some(target) = trimmed.strip_prefix("call ") {
                bc.push(OP_CALL);
                write_u32(&mut bc, 0);
                pending_fixups.push(((bc.len() - 4) as u32, format!("__proc_{}", target.trim())));
                op_count += 1;
                continue;
            }
            if trimmed == "ret" {
                bc.push(OP_RET);
                op_count += 1;
                continue;
            }

            // if <cond>
            if let Some(cond_str) = trimmed.strip_prefix("if ") {
                let _ = cond_str.trim(); // ignored for VM (checks top of stack)
                let endif_label = format!("__cf_{}_endif", cf_counter);
                let else_label = format!("__cf_{}_else", cf_counter);
                cf_counter += 1;
                bc.push(OP_JZ);
                write_u32(&mut bc, 0);
                let br_pos = (bc.len() - 4) as u32;
                pending_fixups.push((br_pos, endif_label.clone()));
                cf_stack.push(CfFrame { kind: CfKind::If, endif_label, else_label, br_positions: vec![br_pos], has_else: false });
                continue;
            }
            // elif <cond>
            if let Some(cond_str) = trimmed.strip_prefix("elif ") {
                let frame = cf_stack.last_mut().ok_or("elif without if")?;
                if frame.has_else { return Err("elif after else".into()); }
                let _ = cond_str.trim();
                let elif_lbl = format!("__cf_{}_elif_{}", cf_counter, frame.br_positions.len());
                cf_counter += 1;
                let prev = frame.br_positions.last().ok_or("internal")?;
                pending_fixups.push((*prev, elif_lbl.clone()));
                bc.push(OP_JMP);
                write_u32(&mut bc, 0);
                pending_fixups.push(((bc.len() - 4) as u32, frame.endif_label.clone()));
                labels.insert(elif_lbl, bc.len() as u32);
                bc.push(OP_JZ);
                write_u32(&mut bc, 0);
                let br_pos = (bc.len() - 4) as u32;
                pending_fixups.push((br_pos, frame.endif_label.clone()));
                frame.br_positions.push(br_pos);
                continue;
            }
            if trimmed == "else" {
                let frame = cf_stack.last_mut().ok_or("else without if")?;
                if frame.has_else { return Err("duplicate else".into()); }
                frame.has_else = true;
                let prev = frame.br_positions.last().ok_or("internal")?;
                pending_fixups.push((*prev, frame.else_label.clone()));
                bc.push(OP_JMP);
                write_u32(&mut bc, 0);
                pending_fixups.push(((bc.len() - 4) as u32, frame.endif_label.clone()));
                labels.insert(frame.else_label.clone(), bc.len() as u32);
                continue;
            }
            if trimmed == "endif" {
                let frame = cf_stack.pop().ok_or("endif without if/while")?;
                match frame.kind { CfKind::While => return Err("endif without matching if".into()), _ => {} }
                labels.insert(frame.endif_label.clone(), bc.len() as u32);
                continue;
            }
            // while <cond>
            if let Some(cond_str) = trimmed.strip_prefix("while ") {
                let _ = cond_str.trim();
                let endwhile_lbl = format!("__cf_{}_endwhile", cf_counter);
                let start_lbl = format!("__cf_{}_start", cf_counter);
                cf_counter += 1;
                labels.insert(start_lbl, bc.len() as u32);
                bc.push(OP_JZ);
                write_u32(&mut bc, 0);
                let br_pos = (bc.len() - 4) as u32;
                pending_fixups.push((br_pos, endwhile_lbl.clone()));
                cf_stack.push(CfFrame { kind: CfKind::While, endif_label: endwhile_lbl, else_label: String::new(), br_positions: vec![br_pos], has_else: false });
                continue;
            }
            if trimmed == "endwhile" {
                let frame = cf_stack.pop().ok_or("endwhile without while")?;
                match frame.kind { CfKind::If => return Err("endwhile without matching while".into()), _ => {} }
                let start_lbl = frame.endif_label.replace("_endwhile", "_start");
                bc.push(OP_JMP);
                write_u32(&mut bc, 0);
                pending_fixups.push(((bc.len() - 4) as u32, start_lbl));
                labels.insert(frame.endif_label.clone(), bc.len() as u32);
                continue;
            }

            let parts: Vec<&str> = trimmed.splitn(4, |c: char| c == ' ' || c == '\t').filter(|s| !s.is_empty()).collect();
            if parts.is_empty() { continue; }
            let mnemonic = parts[0];
            let rest = parts[1..].join(" ");
            let args: Vec<&str> = rest.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

            match mnemonic {
                "push" if args.len() == 1 => {
                    if let Ok(v) = args[0].parse::<i64>() {
                        bc.push(OP_PUSH); write_u64(&mut bc, v as u64); op_count += 1;
                    } else if args[0].starts_with('"') && args[0].ends_with('"') {
                        // String literal
                        let inner = &args[0][1..args[0].len()-1];
                        bc.push(OP_PUSHS);
                        bc.extend_from_slice(inner.as_bytes());
                        bc.push(0);
                        op_count += 1;
                    } else if let Ok(v) = args[0].parse::<u64>() {
                        bc.push(OP_PUSH); write_u64(&mut bc, v); op_count += 1;
                    } else {
                        return Err(format!("bad push value '{}'", args[0]));
                    }
                }
                "dup" => { bc.push(OP_DUP); op_count += 1; }
                "drop" => { bc.push(OP_DROP); op_count += 1; }
                "swap" => { bc.push(OP_SWAP); op_count += 1; }
                "add" => { bc.push(OP_ADD); op_count += 1; }
                "sub" => { bc.push(OP_SUB); op_count += 1; }
                "mul" => { bc.push(OP_MUL); op_count += 1; }
                "div" => { bc.push(OP_DIV); op_count += 1; }
                "mod" => { bc.push(OP_MOD); op_count += 1; }
                "neg" => { bc.push(OP_NEG); op_count += 1; }
                "cmp" => { bc.push(OP_CMP); op_count += 1; }
                "jmp" if args.len() == 1 => {
                    bc.push(OP_JMP); write_u32(&mut bc, 0);
                    pending_fixups.push(((bc.len()-4) as u32, args[0].to_string()));
                    op_count += 1;
                }
                "jz" if args.len() == 1 => {
                    bc.push(OP_JZ); write_u32(&mut bc, 0);
                    pending_fixups.push(((bc.len()-4) as u32, args[0].to_string()));
                    op_count += 1;
                }
                "jnz" if args.len() == 1 => {
                    bc.push(OP_JNZ); write_u32(&mut bc, 0);
                    pending_fixups.push(((bc.len()-4) as u32, args[0].to_string()));
                    op_count += 1;
                }
                "print" => { bc.push(OP_PRINT); op_count += 1; }
                "printc" => { bc.push(OP_PRINTC); op_count += 1; }
                "read" => { bc.push(OP_READ); op_count += 1; }
                "exit" => { bc.push(OP_EXIT); op_count += 1; }
                "nop" => { bc.push(OP_NOP); op_count += 1; }
                _ => return Err(format!("unknown vm instruction '{mnemonic}'")),
            }
        }
    }

    if !cf_stack.is_empty() { return Err("unclosed if/while block".into()); }

    // Patch fixups
    for (pos, label) in &pending_fixups {
        let target = labels.get(label)
            .ok_or_else(|| format!("unknown vm label '{label}'"))?;
        let pos = *pos as usize;
        bc[pos..pos+4].copy_from_slice(&target.to_le_bytes());
    }

    // Patch entry point in header
    let entry_offset = labels.get(&entry_name).copied().unwrap_or(0);
    bc[7..11].copy_from_slice(&entry_offset.to_le_bytes());

    Ok((bc, op_count))
}

// VM interpreter - standalone execution
pub fn run_vm_bytecode(bytecode: &[u8]) -> Result<i64, String> {
    if &bytecode[0..7] != b"DECVM01" {
        return Err("bad vm magic".to_string());
    }
    let mut stack: Vec<i64> = Vec::new();
    let mut memory: Vec<i64> = Vec::new();
    let entry_offset = u32::from_le_bytes([bytecode[7], bytecode[8], bytecode[9], bytecode[10]]) as usize;
    let mut ip = if entry_offset > 0 { entry_offset } else { 12usize };

    loop {
        if ip >= bytecode.len() {
            return if stack.is_empty() { Ok(0) } else { Ok(stack[stack.len()-1]) };
        }
        let op = bytecode[ip];
        ip += 1;
        match op {
            OP_PUSH => {
                let v = i64::from_le_bytes([bytecode[ip], bytecode[ip+1], bytecode[ip+2], bytecode[ip+3],
                    bytecode[ip+4], bytecode[ip+5], bytecode[ip+6], bytecode[ip+7]]);
                stack.push(v); ip += 8;
            }
            OP_DUP => { let v = *stack.last().ok_or("empty stack for dup")?; stack.push(v); }
            OP_DROP => { stack.pop().ok_or("empty stack for drop")?; }
            OP_SWAP => { let a = stack.pop().ok_or("empty stack")?; let b = stack.pop().ok_or("empty stack")?; stack.push(a); stack.push(b); }
            OP_ADD => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(b + a); }
            OP_SUB => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(b - a); }
            OP_MUL => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(b * a); }
            OP_DIV => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(b / a); }
            OP_MOD => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(b % a); }
            OP_NEG => { let a = stack.pop().ok_or("empty")?; stack.push(-a); }
            OP_CMP => { let a = stack.pop().ok_or("empty")?; let b = stack.pop().ok_or("empty")?; stack.push(if b < a { -1 } else if b > a { 1 } else { 0 }); }
            OP_JMP => {
                let target = u32::from_le_bytes([bytecode[ip], bytecode[ip+1], bytecode[ip+2], bytecode[ip+3]]) as usize;
                ip = target; continue;
            }
            OP_JZ => {
                let target = u32::from_le_bytes([bytecode[ip], bytecode[ip+1], bytecode[ip+2], bytecode[ip+3]]) as usize;
                ip += 4;
                if stack.pop().ok_or("empty stack")? == 0 { ip = target; }
            }
            OP_JNZ => {
                let target = u32::from_le_bytes([bytecode[ip], bytecode[ip+1], bytecode[ip+2], bytecode[ip+3]]) as usize;
                ip += 4;
                if stack.pop().ok_or("empty stack")? != 0 { ip = target; }
            }
            OP_CALL => {
                let target = u32::from_le_bytes([bytecode[ip], bytecode[ip+1], bytecode[ip+2], bytecode[ip+3]]) as usize;
                ip += 4;
                stack.push(ip as i64);
                ip = target;
            }
            OP_RET => {
                let ret = stack.pop().ok_or("empty stack for ret")?;
                ip = ret as usize;
                if ip >= bytecode.len() { return Ok(stack.last().copied().unwrap_or(0)); }
            }
            OP_PRINT => { let v = stack.pop().ok_or("empty")?; print!("{}", v); }
            OP_PRINTC => { let v = stack.pop().ok_or("empty")? as u8; print!("{}", v as char); }
            OP_READ => {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
                stack.push(input.trim().parse::<i64>().unwrap_or(0));
            }
            OP_EXIT => { return Ok(stack.last().copied().unwrap_or(0)); }
            OP_NOP => {}
            OP_LOAD => {
                let addr = stack.pop().ok_or("empty")? as usize;
                if addr >= memory.len() { stack.push(0); } else { stack.push(memory[addr]); }
            }
            OP_STORE => {
                let addr = stack.pop().ok_or("empty")? as usize;
                let val = stack.pop().ok_or("empty")?;
                if addr >= memory.len() { memory.resize(addr + 1, 0); }
                memory[addr] = val;
            }
            OP_ALLOC => {
                let size = stack.pop().ok_or("empty")? as usize;
                memory.resize(memory.len() + size, 0);
                stack.push((memory.len() - size) as i64);
            }
            OP_PUSHS => {
                let start = ip;
                while ip < bytecode.len() && bytecode[ip] != 0 { ip += 1; }
                let _s = std::str::from_utf8(&bytecode[start..ip]).map_err(|e| e.to_string())?;
                // Push chars as integer values onto stack
                for &ch in bytecode[start..ip].iter().rev() {
                    stack.push(ch as i64);
                }
                ip += 1; // skip null
            }
            _ => return Err(format!("unknown vm opcode {op} at {ip}")),
        }
    }
}
