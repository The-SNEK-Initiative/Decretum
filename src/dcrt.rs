use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Program {
    pub target: String,
    pub entry_event: String,
    pub data: Vec<DataDecl>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum DataDecl {
    String {
        name: String,
        value: String,
    },
    Scalar {
        name: String,
        width: ScalarWidth,
        value: i64,
    },
    Buffer {
        name: String,
        size: usize,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ScalarWidth {
    Byte,
    Word,
    Dword,
    Qword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Event,
    Proc,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub name: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

pub struct Parser;

impl Parser {
    pub fn parse(source: &str) -> Result<Program, ParseError> {
        let mut target = None;
        let mut entry_event = None;
        let mut data = Vec::new();
        let mut blocks = Vec::new();
        let mut pending_block: Option<(usize, BlockKind, String, Vec<String>)> = None;

        for (index, raw_line) in source.lines().enumerate() {
            let line_num = index + 1;
            let line = strip_comment(raw_line).trim_end();
            if line.trim().is_empty() {
                continue;
            }
            let trimmed = line.trim();

            let is_indented = raw_line.starts_with(' ') || raw_line.starts_with('\t');
            if let Some((_, _, _, ref mut block_lines)) = pending_block {
                if is_indented || !looks_like_top_level(trimmed) {
                    block_lines.push(trimmed.to_string());
                    continue;
                }
            }

            if let Some((start_line, kind, name, block_lines)) = pending_block.take() {
                if block_lines.is_empty() {
                    return Err(ParseError {
                        line: start_line,
                        message: format!("{kind:?} '{name}' has no body"),
                    });
                }
                blocks.push(Block {
                    kind,
                    name,
                    lines: block_lines,
                });
            }

            if let Some(rest) = trimmed.strip_prefix("target ") {
                let value = rest.trim();
                if value.is_empty() {
                    return Err(ParseError {
                        line: line_num,
                        message: "missing target value".to_string(),
                    });
                }
                target = Some(value.to_string());
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("entry ") {
                let value = rest.trim();
                if !is_ident(value) {
                    return Err(ParseError {
                        line: line_num,
                        message: format!("invalid entry event '{value}'"),
                    });
                }
                entry_event = Some(value.to_string());
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("data ") {
                let (name, value) = parse_assign(rest, line_num)?;
                ensure_ident(&name, line_num, "data")?;
                let parsed = parse_quoted_string(&value).map_err(|m| ParseError {
                    line: line_num,
                    message: m,
                })?;
                data.push(DataDecl::String {
                    name,
                    value: parsed,
                });
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("buffer ") {
                let mut parts = rest.split_whitespace();
                let name = parts.next().ok_or(ParseError {
                    line: line_num,
                    message: "buffer declaration needs a name and size".to_string(),
                })?;
                ensure_ident(name, line_num, "buffer")?;
                let size_raw = parts.next().ok_or(ParseError {
                    line: line_num,
                    message: "buffer declaration needs a size".to_string(),
                })?;
                if parts.next().is_some() {
                    return Err(ParseError {
                        line: line_num,
                        message: "unexpected extra tokens in buffer declaration".to_string(),
                    });
                }
                let size = size_raw.parse::<usize>().map_err(|_| ParseError {
                    line: line_num,
                    message: format!("invalid buffer size '{size_raw}'"),
                })?;
                if size == 0 {
                    return Err(ParseError {
                        line: line_num,
                        message: "buffer size must be greater than zero".to_string(),
                    });
                }
                data.push(DataDecl::Buffer {
                    name: name.to_string(),
                    size,
                });
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("byte ") {
                data.push(parse_scalar_decl(rest, line_num, ScalarWidth::Byte)?);
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("word ") {
                data.push(parse_scalar_decl(rest, line_num, ScalarWidth::Word)?);
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("dword ") {
                data.push(parse_scalar_decl(rest, line_num, ScalarWidth::Dword)?);
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("qword ") {
                data.push(parse_scalar_decl(rest, line_num, ScalarWidth::Qword)?);
                continue;
            }

            if let Some(name) = parse_block_header(trimmed, "event") {
                ensure_ident(name, line_num, "event")?;
                pending_block = Some((line_num, BlockKind::Event, name.to_string(), Vec::new()));
                continue;
            }
            if let Some(name) = parse_block_header(trimmed, "proc") {
                ensure_ident(name, line_num, "proc")?;
                pending_block = Some((line_num, BlockKind::Proc, name.to_string(), Vec::new()));
                continue;
            }

            return Err(ParseError {
                line: line_num,
                message: format!("unsupported top-level declaration: '{trimmed}'"),
            });
        }

        if let Some((start_line, kind, name, block_lines)) = pending_block.take() {
            if block_lines.is_empty() {
                return Err(ParseError {
                    line: start_line,
                    message: format!("{kind:?} '{name}' has no body"),
                });
            }
            blocks.push(Block {
                kind,
                name,
                lines: block_lines,
            });
        }

        let target = target.unwrap_or_else(|| "portable".to_string());
        let entry_event = entry_event.ok_or(ParseError {
            line: 1,
            message: "missing required 'entry <event>' declaration".to_string(),
        })?;

        if target != "portable" && target != "win64" && target != "bios16"
            && target != "uefi" && target != "armcm" && target != "riscv"
            && target != "x86_64" && target != "riscv64" && target != "aarch64"
            && target != "vm" && target != "macho" && target != "elf64"
            && target != "cheri" && target != "riscv_cheri" && target != "win32"
            && target != "elf32"
            && target != "mips" && target != "ppc" && target != "sparc"
            && target != "alpha" && target != "parisc" && target != "openrisc"
            && target != "nios2" && target != "microblaze"
            && target != "6502" && target != "z80" && target != "6809"
            && target != "pic" && target != "avr"
            && target != "sh2" && target != "sh4" && target != "m68k"
            && target != "ternary"
            && target != "quantum8" && target != "quantum64"
            && target != "ia64" && target != "vliw"
            && target != "s360" && target != "zarch" && target != "univac" && target != "cdc6600"
            && target != "pdp8" && target != "pdp11" && target != "vax" && target != "hp3000"
            && target != "i4004" && target != "i8008" && target != "i8080" && target != "i8086"
            && target != "m6800" && target != "mos6501"
            && target != "tms320" && target != "blackfin" && target != "sharc"
            && target != "c166" && target != "xc800" && target != "rl78" && target != "rx"
            && target != "h8" && target != "msp430"
            && target != "v20" && target != "nec78k" && target != "m16c" && target != "r8c" && target != "fr"
            && target != "mico32" && target != "picoblaze" && target != "mmix" && target != "dlx" && target != "lc3"
            && target != "huc6280" && target != "v810" && target != "arm7tdmi" && target != "arm9"
            && target != "ppc740" && target != "ppc970"
            && target != "mil1750a" && target != "jovial" && target != "ural" && target != "besm"
            && target != "elbrus" && target != "mir" && target != "harvard" && target != "mill"
        {
            return Err(ParseError {
                line: 1,
                message: format!(
                    "unsupported target '{target}' (supported: portable, win64, bios16, uefi, armcm, riscv, x86_64, riscv64, aarch64, vm, macho, elf64, cheri, riscv_cheri, win32, elf32, mips, ppc, sparc, alpha, parisc, openrisc, nios2, microblaze, 6502, z80, 6809, pic, avr, sh2, sh4, m68k, ternary, quantum8, quantum64, ia64, vliw, s360, zarch, univac, cdc6600, pdp8, pdp11, vax, hp3000, i4004, i8008, i8080, i8086, m6800, mos6501, tms320, blackfin, sharc, c166, xc800, rl78, rx, h8, msp430, v20, nec78k, m16c, r8c, fr, mico32, picoblaze, mmix, dlx, lc3, huc6280, v810, arm7tdmi, arm9, ppc740, ppc970, mil1750a, jovial, ural, besm, elbrus, mir, harvard, mill)"
                ),
            });
        }

        let mut names = BTreeSet::new();
        for block in &blocks {
            if !names.insert(block.name.clone()) {
                return Err(ParseError {
                    line: 1,
                    message: format!("duplicate block name '{}'", block.name),
                });
            }
        }

        if !blocks
            .iter()
            .any(|b| b.kind == BlockKind::Event && b.name == entry_event)
        {
            return Err(ParseError {
                line: 1,
                message: format!("entry event '{entry_event}' was not declared"),
            });
        }

        Ok(Program {
            target,
            entry_event,
            data,
            blocks,
        })
    }
}

fn parse_assign(input: &str, line: usize) -> Result<(String, String), ParseError> {
    let (name, value) = input.split_once('=').ok_or(ParseError {
        line,
        message: "expected assignment with '='".to_string(),
    })?;
    let left = name.trim();
    if left.is_empty() {
        return Err(ParseError {
            line,
            message: "missing assignment name".to_string(),
        });
    }
    let right = value.trim();
    if right.is_empty() {
        return Err(ParseError {
            line,
            message: "missing assignment value".to_string(),
        });
    }
    Ok((left.to_string(), right.to_string()))
}

fn parse_scalar_decl(rest: &str, line: usize, width: ScalarWidth) -> Result<DataDecl, ParseError> {
    let (name, value_raw) = parse_assign(rest, line)?;
    ensure_ident(&name, line, "scalar")?;
    let value = parse_integer(&value_raw).map_err(|m| ParseError { line, message: m })?;
    Ok(DataDecl::Scalar { name, width, value })
}

pub(crate) fn parse_integer(raw: &str) -> Result<i64, String> {
    if let Some(hex) = raw.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).map_err(|_| format!("invalid hex integer '{raw}'"))
    } else if let Some(hex) = raw.strip_prefix("-0x") {
        let value =
            i64::from_str_radix(hex, 16).map_err(|_| format!("invalid hex integer '{raw}'"))?;
        Ok(-value)
    } else {
        raw.parse::<i64>()
            .map_err(|_| format!("invalid integer '{raw}'"))
    }
}

fn parse_block_header<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let prefix = format!("{keyword} ");
    let rest = line.strip_prefix(&prefix)?;
    let rest = rest.trim();
    let name = rest.strip_suffix(':')?;
    Some(name.trim())
}

fn looks_like_top_level(line: &str) -> bool {
    line.starts_with("target ")
        || line.starts_with("entry ")
        || line.starts_with("data ")
        || line.starts_with("buffer ")
        || line.starts_with("byte ")
        || line.starts_with("word ")
        || line.starts_with("dword ")
        || line.starts_with("qword ")
        || parse_block_header(line, "event").is_some()
        || parse_block_header(line, "proc").is_some()
}

fn ensure_ident(name: &str, line: usize, kind: &str) -> Result<(), ParseError> {
    if is_ident(name) {
        Ok(())
    } else {
        Err(ParseError {
            line,
            message: format!("invalid {kind} name '{name}'"),
        })
    }
}

pub(crate) fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

pub(crate) fn parse_quoted_string(raw: &str) -> Result<String, String> {
    if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
        return Err(format!("expected quoted string, got '{raw}'"));
    }
    let mut out = String::new();
    let mut chars = raw[1..raw.len() - 1].chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let next = chars
                .next()
                .ok_or_else(|| "unfinished escape sequence in string".to_string())?;
            match next {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                '0' => out.push('\0'),
                other => return Err(format!("unsupported escape sequence '\\{other}'")),
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut escape = false;
    for (idx, ch) in line.char_indices() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            in_str = true;
            continue;
        }
        if ch == ';' {
            return &line[..idx];
        }
    }
    line
}

pub fn collect_symbols(program: &Program) -> BTreeMap<String, BlockKind> {
    let mut out = BTreeMap::new();
    for block in &program.blocks {
        out.insert(block.name.clone(), block.kind);
    }
    out
}
