use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{DirectM68kBuilder, DirectSh2Builder};

pub struct ShTarget;

impl Target for ShTarget {
    fn name(&self) -> &'static str {
        "sh2"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectSh2Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct M68kTarget;

impl Target for M68kTarget {
    fn name(&self) -> &'static str {
        "m68k"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectM68kBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
