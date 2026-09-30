use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    DirectUefiBuilder, DirectX86_64Builder, I4004Builder, I8008Builder, I8080Builder, I8086Builder,
    NecV20Builder,
};

pub struct X86_64Target;

impl Target for X86_64Target {
    fn name(&self) -> &'static str {
        "x86_64"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectX86_64Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct UefiTarget;

impl Target for UefiTarget {
    fn name(&self) -> &'static str {
        "uefi"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Executable
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectUefiBuilder::build_efi(program, out).map(|o| o.efi_path)
    }
}

pub struct I4004Target;

impl Target for I4004Target {
    fn name(&self) -> &'static str {
        "i4004"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        I4004Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct I8008Target;

impl Target for I8008Target {
    fn name(&self) -> &'static str {
        "i8008"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        I8008Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct I8080Target;

impl Target for I8080Target {
    fn name(&self) -> &'static str {
        "i8080"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        I8080Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct I8086Target;

impl Target for I8086Target {
    fn name(&self) -> &'static str {
        "i8086"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        I8086Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct NecV20Target;

impl Target for NecV20Target {
    fn name(&self) -> &'static str {
        "v20"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        NecV20Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}
