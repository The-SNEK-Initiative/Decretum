use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    DirectAlphaBuilder, DirectMipsBuilder, DirectPariscBuilder, DirectPpcBuilder,
    DirectSparcBuilder,
};

pub struct MipsTarget;

impl Target for MipsTarget {
    fn name(&self) -> &'static str {
        "mips"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectMipsBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct PpcTarget;

impl Target for PpcTarget {
    fn name(&self) -> &'static str {
        "ppc"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectPpcBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct SparcTarget;

impl Target for SparcTarget {
    fn name(&self) -> &'static str {
        "sparc"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectSparcBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct AlphaTarget;

impl Target for AlphaTarget {
    fn name(&self) -> &'static str {
        "alpha"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectAlphaBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct PariscTarget;

impl Target for PariscTarget {
    fn name(&self) -> &'static str {
        "parisc"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectPariscBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
