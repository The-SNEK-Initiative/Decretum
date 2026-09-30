use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{DirectAarch64Builder, DirectArmCmBuilder, DirectCheriBuilder};

pub struct ArmCmTarget;

impl Target for ArmCmTarget {
    fn name(&self) -> &'static str {
        "armcm"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectArmCmBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Aarch64Target;

impl Target for Aarch64Target {
    fn name(&self) -> &'static str {
        "aarch64"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectAarch64Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct CheriTarget;

impl Target for CheriTarget {
    fn name(&self) -> &'static str {
        "cheri"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectCheriBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
