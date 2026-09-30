use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{DirectRisCvCheriBuilder, DirectRiscvBuilder};

pub struct RiscvTarget;

impl Target for RiscvTarget {
    fn name(&self) -> &'static str {
        "riscv"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectRiscvBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct RiscvCheriTarget;

impl Target for RiscvCheriTarget {
    fn name(&self) -> &'static str {
        "riscv_cheri"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectRisCvCheriBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
