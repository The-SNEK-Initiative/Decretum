use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{BlackfinBuilder, SharcBuilder, Tms320Builder};

pub struct Tms320Target;

impl Target for Tms320Target {
    fn name(&self) -> &'static str {
        "tms320"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Tms320Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct BlackfinTarget;

impl Target for BlackfinTarget {
    fn name(&self) -> &'static str {
        "blackfin"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        BlackfinBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct SharcTarget;

impl Target for SharcTarget {
    fn name(&self) -> &'static str {
        "sharc"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        SharcBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
