use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    Arm7tdmiBuilder, Arm9Builder, HuC6280Builder, Ppc740Builder, Ppc970Builder, V810Builder,
};

pub struct Huc6280Target;

impl Target for Huc6280Target {
    fn name(&self) -> &'static str {
        "huc6280"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        HuC6280Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct V810Target;

impl Target for V810Target {
    fn name(&self) -> &'static str {
        "v810"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        V810Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Arm7tdmiTarget;

impl Target for Arm7tdmiTarget {
    fn name(&self) -> &'static str {
        "arm7tdmi"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Arm7tdmiBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Arm9Target;

impl Target for Arm9Target {
    fn name(&self) -> &'static str {
        "arm9"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Arm9Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Ppc740Target;

impl Target for Ppc740Target {
    fn name(&self) -> &'static str {
        "ppc740"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Ppc740Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Ppc970Target;

impl Target for Ppc970Target {
    fn name(&self) -> &'static str {
        "ppc970"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Ppc970Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}
