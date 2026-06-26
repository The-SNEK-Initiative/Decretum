// ELF64 executable wrapper (Linux x86-64)
// Uses x86-64 machine code from the UEFI assembler

use std::path::{Path, PathBuf};
use crate::dcrt::Program;
use crate::direct_uefi::DirectUefiAssembler;

pub struct ElfBuildOutput {
    pub elf_path: PathBuf,
    pub elf_size: usize,
}

pub struct DirectElfBuilder;

impl DirectElfBuilder {
    pub fn build_elf(program: &Program, out_path: &Path) -> Result<ElfBuildOutput, String> {
        if program.target != "elf64" {
            return Err(format!("elf64 backend requires target 'elf64', got '{}'", program.target));
        }
        let kernel = DirectUefiAssembler::assemble(program)?;
        let elf = build_elf64(&kernel)?;
        std::fs::write(out_path, &elf).map_err(|e| format!("write failed: {e}"))?;
        Ok(ElfBuildOutput { elf_path: out_path.to_path_buf(), elf_size: elf.len() })
    }
}

fn build_elf64(kernel: &[u8]) -> Result<Vec<u8>, String> {
    let mut f = Vec::new();

    // ELF64 header (64 bytes)
    // e_ident
    f.push(0x7F); f.push(b'E'); f.push(b'L'); f.push(b'F'); // magic
    f.push(2);  // ELFCLASS64
    f.push(1);  // ELFDATA2LSB
    f.push(1);  // EV_CURRENT
    f.push(0);  // ELFOSABI_SYSV
    f.push(0);  // abi version
    let padding = [0u8; 7];
    f.extend_from_slice(&padding); // e_ident padding

    // e_type
    f.extend_from_slice(&2u16.to_le_bytes());   // ET_EXEC
    // e_machine
    f.extend_from_slice(&0x3Eu16.to_le_bytes()); // EM_X86_64
    // e_version
    f.extend_from_slice(&1u32.to_le_bytes());
    // e_entry (entry point = base_vaddr)
    let base_vaddr = 0x400000u64;
    f.extend_from_slice(&base_vaddr.to_le_bytes());
    // e_phoff (program header offset)
    let e_phoff: u64 = 64;
    f.extend_from_slice(&e_phoff.to_le_bytes());
    // e_shoff (section header offset = 0, no sections)
    f.extend_from_slice(&0u64.to_le_bytes());
    // e_flags
    f.extend_from_slice(&0u32.to_le_bytes());
    // e_ehsize
    f.extend_from_slice(&64u16.to_le_bytes());
    // e_phentsize
    f.extend_from_slice(&56u16.to_le_bytes());  // sizeof(Phdr64)
    // e_phnum
    f.extend_from_slice(&2u16.to_le_bytes());   // 2 segments: PT_LOAD for text + PT_LOAD for optional
    // e_shentsize
    f.extend_from_slice(&0u16.to_le_bytes());
    // e_shnum
    f.extend_from_slice(&0u16.to_le_bytes());
    // e_shstrndx
    f.extend_from_slice(&0u16.to_le_bytes());

    // Program headers (56 bytes each)
    let text_offset = 64 + 56 * 2; // headers + 2 phdrs
    let text_size = kernel.len() as u64;
    let page_size = 0x1000u64;
    let text_memsize = ((text_size + page_size - 1) / page_size) * page_size;

    // PT_LOAD for text segment
    let p_type: u32 = 1;        // PT_LOAD
    let p_flags: u32 = 5;      // PF_R | PF_X
    let p_offset: u64 = 0;
    let p_vaddr: u64 = base_vaddr;
    let p_paddr: u64 = base_vaddr;
    let p_filesz: u64 = text_offset + text_size;
    let p_memsz: u64 = text_offset + text_memsize;
    let p_align: u64 = page_size;

    f.extend_from_slice(&p_type.to_le_bytes());
    f.extend_from_slice(&p_flags.to_le_bytes());
    f.extend_from_slice(&p_offset.to_le_bytes());
    f.extend_from_slice(&p_vaddr.to_le_bytes());
    f.extend_from_slice(&p_paddr.to_le_bytes());
    f.extend_from_slice(&p_filesz.to_le_bytes());
    f.extend_from_slice(&p_memsz.to_le_bytes());
    f.extend_from_slice(&p_align.to_le_bytes());

    // PT_GNU_STACK (to mark stack as executable-agnostic)
    let stack_type: u32 = 0x6474E551; // PT_GNU_STACK
    let stack_flags: u32 = 6;         // PF_R | PF_W
    let stack_zero: u64 = 0;

    f.extend_from_slice(&stack_type.to_le_bytes());
    f.extend_from_slice(&stack_flags.to_le_bytes());
    f.extend_from_slice(&stack_zero.to_le_bytes()); // offset
    f.extend_from_slice(&stack_zero.to_le_bytes()); // vaddr
    f.extend_from_slice(&stack_zero.to_le_bytes()); // paddr
    f.extend_from_slice(&stack_zero.to_le_bytes()); // filesz
    f.extend_from_slice(&stack_zero.to_le_bytes()); // memsz
    f.extend_from_slice(&page_size.to_le_bytes());  // align

    // Kernel code
    f.extend_from_slice(kernel);

    Ok(f)
}
