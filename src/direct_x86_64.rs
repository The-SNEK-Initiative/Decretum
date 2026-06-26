use std::path::{Path, PathBuf};
use crate::dcrt::{Program};
use crate::direct_uefi::{DirectUefiAssembler};

pub struct X86_64BuildOutput {
    pub bin_path: PathBuf,
    pub bin_size: usize,
}

pub struct DirectX86_64Builder;

impl DirectX86_64Builder {
    pub fn build_bin(program: &Program, out_path: &Path) -> Result<X86_64BuildOutput, String> {
        if program.target != "x86_64" {
            return Err(format!("direct x86_64 backend requires target 'x86_64', got '{}'", program.target));
        }
        let kernel = DirectUefiAssembler::assemble(program)?;
        std::fs::write(out_path, &kernel).map_err(|e| format!("failed to write: {e}"))?;
        Ok(X86_64BuildOutput {
            bin_path: out_path.to_path_buf(),
            bin_size: kernel.len(),
        })
    }
}
