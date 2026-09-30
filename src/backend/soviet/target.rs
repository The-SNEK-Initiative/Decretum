use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{BesmBuilder, ElbrusBuilder, MirBuilder, UralBuilder};

pub struct UralTarget;

impl Target for UralTarget {
    fn name(&self) -> &'static str {
        "ural"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        UralBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct BesmTarget;

impl Target for BesmTarget {
    fn name(&self) -> &'static str {
        "besm"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        BesmBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct ElbrusTarget;

impl Target for ElbrusTarget {
    fn name(&self) -> &'static str {
        "elbrus"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        ElbrusBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct MirTarget;

impl Target for MirTarget {
    fn name(&self) -> &'static str {
        "mir"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        MirBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
