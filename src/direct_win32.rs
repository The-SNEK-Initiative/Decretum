// PE32 (Windows 32-bit) executable wrapper
// Uses i386 instruction encoding

use std::path::{Path, PathBuf};
use crate::dcrt::Program;
use crate::direct_i386::I386Assembler;

pub struct Win32BuildOutput { pub pe_path: PathBuf, pub pe_size: usize }
pub struct DirectWin32Builder;

impl DirectWin32Builder {
    pub fn build_pe(program: &Program, out_path: &Path) -> Result<Win32BuildOutput, String> {
        if program.target != "win32" {
            return Err(format!("win32 backend requires target 'win32', got '{}'", program.target));
        }
        let kernel = I386Assembler::assemble(program)?;
        let pe = build_pe32(&kernel)?;
        std::fs::write(out_path, &pe).map_err(|e| format!("write failed: {e}"))?;
        Ok(Win32BuildOutput { pe_path: out_path.to_path_buf(), pe_size: pe.len() })
    }
}

fn build_pe32(kernel: &[u8]) -> Result<Vec<u8>, String> {
    let mut f = Vec::new();
    let header_size: u32 = 0x200; // 512 byte header
    let file_align: u32 = 0x200;
    let sect_align: u32 = 0x1000;
    let code_size = ((kernel.len() as u32 + file_align - 1) / file_align) * file_align;

    // DOS header (64 bytes)
    f.push(0x4D); f.push(0x5A); // "MZ"
    while f.len() < 0x3C { f.push(0); }
    f.extend_from_slice(&0x80u32.to_le_bytes()); // PE offset = 0x80
    while f.len() < 0x80 { f.push(0); }

    // PE signature
    f.extend_from_slice(b"PE\0\0");

    // COFF header (20 bytes)
    let machine: u16 = 0x14C;  // I386
    let num_sections: u16 = 1;
    let opt_header_size: u16 = 0xE0; // PE32 optional header
    let characteristics: u16 = 0x0102; // EXE + 32BIT

    f.extend_from_slice(&machine.to_le_bytes());
    f.extend_from_slice(&num_sections.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes()); // timestamp
    f.extend_from_slice(&0u32.to_le_bytes()); // ptr to symtab
    f.extend_from_slice(&0u32.to_le_bytes()); // num syms
    f.extend_from_slice(&opt_header_size.to_le_bytes());
    f.extend_from_slice(&characteristics.to_le_bytes());

    // PE32 optional header (0xE0 = 224 bytes)
    let magic: u16 = 0x10B; // PE32
    let entry: u32 = sect_align + 0x10; // .text base + small offset
    let image_base: u32 = 0x400000;

    f.extend_from_slice(&magic.to_le_bytes());
    f.extend_from_slice(&10u8.to_le_bytes()); // major linker
    f.extend_from_slice(&0u8.to_le_bytes());  // minor linker
    f.extend_from_slice(&code_size.to_le_bytes()); // code size
    f.extend_from_slice(&0u32.to_le_bytes()); // initialized data
    f.extend_from_slice(&0u32.to_le_bytes()); // uninit data
    f.extend_from_slice(&entry.to_le_bytes()); // entry RVA
    f.extend_from_slice(&sect_align.to_le_bytes()); // code base
    f.extend_from_slice(&image_base.to_le_bytes());
    f.extend_from_slice(&sect_align.to_le_bytes());
    f.extend_from_slice(&file_align.to_le_bytes());
    f.extend_from_slice(&4u16.to_le_bytes()); // major OS
    f.extend_from_slice(&0u16.to_le_bytes()); // minor OS
    f.extend_from_slice(&0u16.to_le_bytes()); // major image
    f.extend_from_slice(&0u16.to_le_bytes()); // minor image
    f.extend_from_slice(&4u16.to_le_bytes()); // major subsys
    f.extend_from_slice(&0u16.to_le_bytes()); // minor subsys
    f.extend_from_slice(&0u32.to_le_bytes()); // win32 version
    f.extend_from_slice(&(sect_align + code_size).to_le_bytes()); // image size
    f.extend_from_slice(&header_size.to_le_bytes()); // headers size
    f.extend_from_slice(&0u32.to_le_bytes()); // checksum
    f.extend_from_slice(&2u16.to_le_bytes()); // subsystem (GUI)
    f.extend_from_slice(&0u16.to_le_bytes()); // dll characteristics
    f.extend_from_slice(&0x100000u32.to_le_bytes()); // stack reserve
    f.extend_from_slice(&0x1000u32.to_le_bytes());  // stack commit
    f.extend_from_slice(&0x100000u32.to_le_bytes()); // heap reserve
    f.extend_from_slice(&0x1000u32.to_le_bytes());  // heap commit
    f.extend_from_slice(&0u32.to_le_bytes()); // loader flags
    f.extend_from_slice(&16u32.to_le_bytes()); // number of RVA and sizes

    // Data directories (16 entries * 8 bytes = 128 bytes)
    for _ in 0..16 { f.extend_from_slice(&0u32.to_le_bytes()); f.extend_from_slice(&0u32.to_le_bytes()); }

    // Section table (.text)
    f.extend_from_slice(b".text\0\0\0");
    f.extend_from_slice(&code_size.to_le_bytes());
    f.extend_from_slice(&sect_align.to_le_bytes()); // virtual address
    f.extend_from_slice(&code_size.to_le_bytes()); // raw size
    f.extend_from_slice(&header_size.to_le_bytes()); // raw offset
    f.extend_from_slice(&0u32.to_le_bytes()); // reloc
    f.extend_from_slice(&0u32.to_le_bytes()); // line nums
    f.extend_from_slice(&0u16.to_le_bytes()); // num relocs
    f.extend_from_slice(&0u16.to_le_bytes()); // num line nums
    f.extend_from_slice(&0x60000020u32.to_le_bytes()); // flags

    // Pad to header size
    while f.len() < header_size as usize { f.push(0); }

    // Code
    f.extend_from_slice(kernel);
    while f.len() < (header_size + code_size) as usize { f.push(0); }

    Ok(f)
}
