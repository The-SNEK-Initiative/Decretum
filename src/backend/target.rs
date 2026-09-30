use std::path::{Path, PathBuf};

use crate::frontend::Program;

pub enum OutputKind {
    Binary,
    Bytecode,
    DiskImage,
    Executable,
}

pub trait Target {
    fn name(&self) -> &'static str;
    fn output_kind(&self) -> OutputKind;
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String>;
}
