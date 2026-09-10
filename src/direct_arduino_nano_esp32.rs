// Random Arduino ESP32 backend
// Needed it randomly so here it is
// Outputs .dapp files, that you can use directly
// ¯\(°_o)/¯

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::dcrt::{BlockKind, DataDecl, Program, ScalarWidth};

pub const APP_MAGIC: &[u8; 8] = b"DCRTESP\0";
pub const APP_FORMAT_VERSION: u16 = 1;

pub struct ArduinoEsp32BuildOutput {
    pub app_path: PathBuf,
    pub app_size: usize,
    pub event_count: usize,
}

pub struct DirectArduinoEsp32Builder;

#[derive(Clone, Copy)]
#[repr(u8)]
enum Opcode {
    Ret = 0,
    Emit = 1,
    Host = 2,
}

const HOST_OPERATIONS: &[(&str, u8)] = &[
    ("system_boot", 1),
    ("wifi_start", 2),
    ("motor_start", 3),
    ("dashboard_start", 4),
    ("api_poll", 5),
    ("motion_tick", 6),
    ("teach_toggle", 7),
    ("capture", 8),
    ("undo", 9),
    ("redo", 10),
    ("finish", 11),
    ("play", 12),
    ("stop", 13),
    ("delete_program", 14),
];

impl DirectArduinoEsp32Builder {
    pub fn build_app(program: &Program, output: &Path) -> Result<ArduinoEsp32BuildOutput, String> {
        if program.target != "arduino_nano_esp32" {
            return Err(format!(
                "need target 'arduino_nano_esp32', got '{}'",
                program.target
            ));
        }
        let bytes = encode(program)?;
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(output, &bytes).map_err(|error| error.to_string())?;
        Ok(ArduinoEsp32BuildOutput {
            app_path: output.to_path_buf(),
            app_size: bytes.len(),
            event_count: program.blocks.len(),
        })
    }
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn push_name(output: &mut Vec<u8>, name: &str) -> Result<(), String> {
    let length: u8 = name
        .len()
        .try_into()
        .map_err(|_| format!("name '{name}' is too long for ESP32 application format"))?;
    output.push(length);
    output.extend_from_slice(name.as_bytes());
    Ok(())
}

fn encode(program: &Program) -> Result<Vec<u8>, String> {
    if program.blocks.len() > u16::MAX as usize || program.data.len() > u16::MAX as usize {
        return Err("too many Decretum declarations for ESP32 application format".into());
    }
    let event_indices: BTreeMap<&str, u16> = program
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.name.as_str(), index as u16))
        .collect();
    let entry_index = event_indices
        .get(program.entry_event.as_str())
        .copied()
        .ok_or_else(|| "entry event missing".to_string())?;

    let mut payload = Vec::new();
    for data in &program.data {
        match data {
            DataDecl::String { name, value } => {
                payload.push(1);
                push_name(&mut payload, name)?;
                push_u32(&mut payload, value.len() as u32);
                payload.extend_from_slice(value.as_bytes());
            }
            DataDecl::Scalar { name, width, value } => {
                payload.push(match width {
                    ScalarWidth::Byte => 2,
                    ScalarWidth::Word => 3,
                    ScalarWidth::Dword => 4,
                    ScalarWidth::Qword => 5,
                });
                push_name(&mut payload, name)?;
                let bytes = match width {
                    ScalarWidth::Byte => vec![*value as u8],
                    ScalarWidth::Word => (*value as u16).to_le_bytes().to_vec(),
                    ScalarWidth::Dword => (*value as u32).to_le_bytes().to_vec(),
                    ScalarWidth::Qword => (*value as u64).to_le_bytes().to_vec(),
                };
                push_u32(&mut payload, bytes.len() as u32);
                payload.extend_from_slice(&bytes);
            }
            DataDecl::Buffer { name, size } => {
                payload.push(6);
                push_name(&mut payload, name)?;
                push_u32(&mut payload, *size as u32);
                payload.resize(payload.len() + size, 0);
            }
        }
    }

    for block in &program.blocks {
        payload.push(match block.kind {
            BlockKind::Event => 1,
            BlockKind::Proc => 2,
        });
        push_name(&mut payload, &block.name)?;
        let mut instructions = Vec::new();
        for line in &block.lines {
            let line = line.trim();
            if line == "ret" {
                instructions.push(Opcode::Ret as u8);
            } else if let Some(name) = line
                .strip_prefix("emit ")
                .or_else(|| line.strip_prefix("call "))
            {
                let index = event_indices
                    .get(name.trim())
                    .ok_or_else(|| format!("unknown event or procedure '{}'", name.trim()))?;
                instructions.push(Opcode::Emit as u8);
                push_u16(&mut instructions, *index);
            } else if let Some(name) = line.strip_prefix("host ") {
                let operation = HOST_OPERATIONS
                    .iter()
                    .find_map(|(known, code)| (*known == name.trim()).then_some(*code))
                    .ok_or_else(|| {
                        format!("unknown Arduino ESP32 host operation '{}'", name.trim())
                    })?;
                instructions.push(Opcode::Host as u8);
                instructions.push(operation);
            } else {
                return Err(format!(
                    "unsupported Arduino ESP32 instruction '{}' in block '{}'",
                    line, block.name
                ));
            }
        }
        if instructions.last() != Some(&(Opcode::Ret as u8)) {
            instructions.push(Opcode::Ret as u8);
        }
        push_u32(&mut payload, instructions.len() as u32);
        payload.extend_from_slice(&instructions);
    }

    let mut output = Vec::with_capacity(32 + payload.len());
    output.extend_from_slice(APP_MAGIC);
    push_u16(&mut output, APP_FORMAT_VERSION);
    push_u16(&mut output, 32);
    push_u32(&mut output, (32 + payload.len()) as u32);
    push_u32(&mut output, crc32(&payload));
    push_u16(&mut output, program.data.len() as u16);
    push_u16(&mut output, program.blocks.len() as u16);
    push_u16(&mut output, entry_index);
    push_u16(&mut output, 0);
    push_u32(&mut output, 0);
    output.extend_from_slice(&payload);
    Ok(output)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcrt::Parser;

    #[test]
    fn emits_checked_application_image() {
        let source = r#"target arduino_nano_esp32
entry boot
data product = "SNEK Aletheia"
event boot:
    host system_boot
    emit tick
    ret
event tick:
    host motion_tick
    ret
"#;
        let program = Parser::parse(source).unwrap();
        let bytes = encode(&program).unwrap();
        assert_eq!(&bytes[..8], APP_MAGIC);
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), APP_FORMAT_VERSION);
        assert_eq!(
            u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize,
            bytes.len()
        );
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 2);
    }

    #[test]
    fn rejects_unknown_host_operation() {
        let source = "target arduino_nano_esp32\nentry boot\nevent boot:\n    host magic\n";
        let program = Parser::parse(source).unwrap();
        assert!(
            encode(&program)
                .unwrap_err()
                .contains("unknown Arduino ESP32 host operation")
        );
    }
}
