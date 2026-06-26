// ELF32 (Linux 32-bit) executable wrapper
// Uses i386 instruction encoding

use std::path::{Path, PathBuf};
use crate::dcrt::Program;
use crate::direct_i386::I386Assembler;

pub struct Elf32BuildOutput { pub elf_path: PathBuf, pub elf_size: usize }
pub struct DirectElf32Builder;

impl DirectElf32Builder {
    pub fn build_elf(program: &Program, out_path: &Path) -> Result<Elf32BuildOutput, String> {
        if program.target != "elf32" {
            return Err(format!("elf32 backend requires target 'elf32', got '{}'", program.target));
        }
        let kernel = I386Assembler::assemble(program)?;
        let elf = build_elf32(&kernel)?;
        std::fs::write(out_path, &elf).map_err(|e| format!("write failed: {e}"))?;
        Ok(Elf32BuildOutput { elf_path: out_path.to_path_buf(), elf_size: elf.len() })
    }
}

fn build_elf32(kernel: &[u8]) -> Result<Vec<u8>, String> {
    let mut f = Vec::new();

    // ELF32 header (52 bytes)
    f.push(0x7F); f.push(b'E'); f.push(b'L'); f.push(b'F'); // magic
    f.push(1);  // ELFCLASS32
    f.push(1);  // ELFDATA2LSB
    f.push(1);  // EV_CURRENT
    f.push(0);  // ELFOSABI_SYSV
    let padding = [0u8; 8];
    f.extend_from_slice(&padding);

    f.extend_from_slice(&2u16.to_le_bytes());  // ET_EXEC
    f.extend_from_slice(&3u16.to_le_bytes());  // EM_386
    f.extend_from_slice(&1u32.to_le_bytes());  // version
    f.extend_from_slice(&0x400000u32.to_le_bytes()); // entry (i386 Linux default base)
    f.extend_from_slice(&52u32.to_le_bytes()); // phoff
    f.extend_from_slice(&0u32.to_le_bytes());  // shoff
    f.extend_from_slice(&0u32.to_le_bytes());  // flags
    f.extend_from_slice(&52u16.to_le_bytes()); // ehsize
    f.extend_from_slice(&32u16.to_le_bytes()); // phentsize
    f.extend_from_slice(&2u16.to_le_bytes());  // phnum (text + GNU_STACK)
    f.extend_from_slice(&0u16.to_le_bytes());  // shentsize
    f.extend_from_slice(&0u16.to_le_bytes());  // shnum
    f.extend_from_slice(&0u16.to_le_bytes());  // shstrndx

    // Program headers (32 bytes each)
    let text_offset = 52 + 32 * 2; // header + 2 phdrs
    let page_size = 0x1000u32;
    let text_memsz = ((kernel.len() as u32 + page_size - 1) / page_size) * page_size;

    // PT_LOAD
    f.extend_from_slice(&1u32.to_le_bytes());  // type = PT_LOAD
    f.extend_from_slice(&0u32.to_le_bytes());  // offset
    f.extend_from_slice(&0x400000u32.to_le_bytes()); // vaddr
    f.extend_from_slice(&0x400000u32.to_le_bytes()); // paddr
    f.extend_from_slice(&(text_offset + kernel.len() as u32).to_le_bytes()); // filesz
    f.extend_from_slice(&(text_offset + text_memsz).to_le_bytes()); // memsz
    f.extend_from_slice(&5u32.to_le_bytes());  // flags (R+X)
    f.extend_from_slice(&page_size.to_le_bytes()); // align

    // PT_GNU_STACK
    f.extend_from_slice(&0x6474E551u32.to_le_bytes()); // PT_GNU_STACK
    f.extend_from_slice(&0u32.to_le_bytes());  // offset
    f.extend_from_slice(&0u32.to_le_bytes());  // vaddr
    f.extend_from_slice(&0u32.to_le_bytes());  // paddr
    f.extend_from_slice(&0u32.to_le_bytes());  // filesz
    f.extend_from_slice(&0u32.to_le_bytes());  // memsz
    f.extend_from_slice(&6u32.to_le_bytes());  // flags (R+W)
    f.extend_from_slice(&page_size.to_le_bytes()); // align

    // Kernel code
    f.extend_from_slice(kernel);

    Ok(f)
}
