use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::backend::target::{OutputKind, Target};
use super::{DirectVmBuilder, PortableBuilder};

pub struct PortableTarget;

impl Target for PortableTarget {
    fn name(&self) -> &'static str {
        "portable"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Bytecode
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        PortableBuilder::build_bytecode(program, out).map(|o| o.bytecode_path)
    }
}

pub struct VmTarget;

impl Target for VmTarget {
    fn name(&self) -> &'static str {
        "vm"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Bytecode
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectVmBuilder::build_bytecode(program, out).map(|o| o.bytecode_path)
    }
}
