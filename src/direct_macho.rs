// Mach-O 64-bit executable wrapper (macOS)
// Uses AArch64 machine code from the AArch64 backend

use std::path::{Path, PathBuf};
use crate::dcrt::Program;
use crate::direct_aarch64::DirectAarch64Builder;

pub struct MachoBuildOutput {
    pub macho_path: PathBuf,
    pub macho_size: usize,
}

pub struct DirectMachoBuilder;

impl DirectMachoBuilder {
    pub fn build_macho(program: &Program, out_path: &Path) -> Result<MachoBuildOutput, String> {
        if program.target != "macho" {
            return Err(format!("macho backend requires target 'macho', got '{}'", program.target));
        }
        let kernel = DirectAarch64Builder::assemble(program)?;
        let macho = build_macho_file(&kernel)?;
        std::fs::write(out_path, &macho).map_err(|e| format!("write failed: {e}"))?;
        Ok(MachoBuildOutput { macho_path: out_path.to_path_buf(), macho_size: macho.len() })
    }
}

fn build_macho_file(kernel: &[u8]) -> Result<Vec<u8>, String> {
    let mut f = Vec::new();

    // Mach-O 64 header
    let magic: u32 = 0xFEEDFACF;        // MH_MAGIC_64
    let cputype: u32 = 0x0100000C;      // CPU_TYPE_ARM64
    let cpusubtype: u32 = 0;            // CPU_SUBTYPE_ARM64_ALL
    let filetype: u32 = 2;              // MH_EXECUTE
    let ncmds: u32 = 2;                 // LC_SEGMENT_64 + LC_MAIN
    let flags: u32 = 0x200085;          // MH_NOUNDEFS | MH_DYLDLINK | MH_TWOLEVEL

    let page_size = 0x4000u64;
    let kernel_vaddr = page_size;       // Load at page boundary
    let kernel_size = ((kernel.len() as u64 + page_size - 1) / page_size) * page_size;

    // Size of load commands
    // LC_SEGMENT_64: 72 bytes + 2 * 72 = 216
    // LC_MAIN: 24 bytes
    let sizeofcmds: u32 = 216 + 24;

    f.extend_from_slice(&magic.to_le_bytes());
    f.extend_from_slice(&cputype.to_le_bytes());
    f.extend_from_slice(&cpusubtype.to_le_bytes());
    f.extend_from_slice(&filetype.to_le_bytes());
    f.extend_from_slice(&ncmds.to_le_bytes());
    f.extend_from_slice(&sizeofcmds.to_le_bytes());
    f.extend_from_slice(&flags.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes()); // reserved

    // LC_SEGMENT_64
    let seg_cmd: u32 = 0x19;            // LC_SEGMENT_64
    let seg_cmdsize: u32 = 216;         // 72 + 2*72 (2 sections)
    let segname = [0u8; 16];            // empty name
    let vmaddr: u64 = 0;                // start of address space
    let vmsize: u64 = kernel_vaddr + kernel_size;
    let fileoff: u64 = 0;
    let filesize: u64 = 0;
    let maxprot: u32 = 7;               // rwx
    let initprot: u32 = 7;              // rwx
    let nsects: u32 = 2;
    let segflags: u32 = 0;

    f.extend_from_slice(&seg_cmd.to_le_bytes());
    f.extend_from_slice(&seg_cmdsize.to_le_bytes());
    f.extend_from_slice(&segname);
    f.extend_from_slice(&vmaddr.to_le_bytes());
    f.extend_from_slice(&vmsize.to_le_bytes());
    f.extend_from_slice(&fileoff.to_le_bytes());
    f.extend_from_slice(&filesize.to_le_bytes());
    f.extend_from_slice(&maxprot.to_le_bytes());
    f.extend_from_slice(&initprot.to_le_bytes());
    f.extend_from_slice(&nsects.to_le_bytes());
    f.extend_from_slice(&segflags.to_le_bytes());

    // Section 1: __text (code)
    let sectname = pad_str(b"__text", 16);
    let segname2 = pad_str(b"__TEXT", 16);
    let text_addr: u64 = kernel_vaddr;
    let text_size: u64 = kernel.len() as u64;
    let text_off: u64 = header_size() + sizeofcmds as u64;
    let align: u32 = 2;                 // 2^2 = 4 byte alignment
    let reloff: u64 = 0;
    let nreloc: u32 = 0;
    let sect_flags: u32 = 0x80000400;   // S_ATTR_SOME_INSTRUCTIONS | S_REGULAR
    let reserved1: u32 = 0;
    let reserved2: u32 = 0;
    let reserved3: u32 = 0;

    f.extend_from_slice(&sectname);
    f.extend_from_slice(&segname2);
    f.extend_from_slice(&text_addr.to_le_bytes());
    f.extend_from_slice(&text_size.to_le_bytes());
    f.extend_from_slice(&text_off.to_le_bytes());
    f.extend_from_slice(&align.to_le_bytes());
    f.extend_from_slice(&reloff.to_le_bytes());
    f.extend_from_slice(&nreloc.to_le_bytes());
    f.extend_from_slice(&sect_flags.to_le_bytes());
    f.extend_from_slice(&reserved1.to_le_bytes());
    f.extend_from_slice(&reserved2.to_le_bytes());
    f.extend_from_slice(&reserved3.to_le_bytes());

    // Section 2: __data
    let dsectname = pad_str(b"__data", 16);
    let dsegname = pad_str(b"__DATA", 16);
    let data_addr: u64 = kernel_vaddr + kernel_size;
    let data_size: u64 = 0;
    let data_off: u64 = text_off + kernel_size;
    let d_flags: u32 = 0;

    f.extend_from_slice(&dsectname);
    f.extend_from_slice(&dsegname);
    f.extend_from_slice(&data_addr.to_le_bytes());
    f.extend_from_slice(&data_size.to_le_bytes());
    f.extend_from_slice(&data_off.to_le_bytes());
    f.extend_from_slice(&align.to_le_bytes());
    f.extend_from_slice(&reloff.to_le_bytes());
    f.extend_from_slice(&nreloc.to_le_bytes());
    f.extend_from_slice(&d_flags.to_le_bytes());
    f.extend_from_slice(&reserved1.to_le_bytes());
    f.extend_from_slice(&reserved2.to_le_bytes());
    f.extend_from_slice(&reserved3.to_le_bytes());

    // LC_MAIN
    let main_cmd: u32 = 0x28;           // LC_MAIN
    let main_cmdsize: u32 = 24;
    let entryoff: u64 = text_off as u64; // entry offset in file
    let stacksize: u64 = 0x800000;      // 8MB stack

    f.extend_from_slice(&main_cmd.to_le_bytes());
    f.extend_from_slice(&main_cmdsize.to_le_bytes());
    f.extend_from_slice(&entryoff.to_le_bytes());
    f.extend_from_slice(&stacksize.to_le_bytes());

    // Align to page boundary
    while f.len() < text_off as usize {
        f.push(0);
    }

    // Write kernel code
    f.extend_from_slice(kernel);

    Ok(f)
}

fn pad_str(s: &[u8], len: usize) -> [u8; 16] {
    let mut out = [0u8; 16];
    let copy_len = s.len().min(len);
    out[..copy_len].copy_from_slice(&s[..copy_len]);
    out
}

fn header_size() -> u64 {
    // 32 bytes mach_header_64 + load commands rounded up to page
    let base = 32u64 + 216 + 24; // header + 2 segments + LC_MAIN
    ((base + 0x3FFF) / 0x4000) * 0x4000
}
