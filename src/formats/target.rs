use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::backend::target::{OutputKind, Target};
use super::{
    DirectBiosBuilder, DirectElf32Builder, DirectElfBuilder, DirectMachoBuilder,
    DirectWin32Builder,
};

pub struct BiosTarget;

impl Target for BiosTarget {
    fn name(&self) -> &'static str {
        "bios16"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::DiskImage
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectBiosBuilder::build_boot_image(program, out).map(|o| o.image_path)
    }
}

pub struct ElfTarget;

impl Target for ElfTarget {
    fn name(&self) -> &'static str {
        "elf64"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Executable
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectElfBuilder::build_elf(program, out).map(|o| o.elf_path)
    }
}

pub struct Elf32Target;

impl Target for Elf32Target {
    fn name(&self) -> &'static str {
        "elf32"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Executable
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectElf32Builder::build_elf(program, out).map(|o| o.elf_path)
    }
}

pub struct MachoTarget;

impl Target for MachoTarget {
    fn name(&self) -> &'static str {
        "macho"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Executable
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectMachoBuilder::build_macho(program, out).map(|o| o.macho_path)
    }
}

pub struct Win32Target;

impl Target for Win32Target {
    fn name(&self) -> &'static str {
        "win32"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Executable
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectWin32Builder::build_pe(program, out).map(|o| o.pe_path)
    }
}
