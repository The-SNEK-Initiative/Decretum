use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{DirectQuantum64Builder, DirectQuantum8Builder};

pub struct Quantum8Target;

impl Target for Quantum8Target {
    fn name(&self) -> &'static str {
        "quantum8"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectQuantum8Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Quantum64Target;

impl Target for Quantum64Target {
    fn name(&self) -> &'static str {
        "quantum64"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectQuantum64Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}
