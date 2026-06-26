// RISC-V CHERI - delegates to RISC-V backend
use std::path::{Path, PathBuf};
use crate::dcrt::Program;
use crate::direct_riscv::DirectRiscvBuilder;

pub struct RisCvCheriBuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectRisCvCheriBuilder;

impl DirectRisCvCheriBuilder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<RisCvCheriBuildOutput, String> {
        if p.target != "riscv_cheri" {
            return Err(format!("need 'riscv_cheri', got '{}'", p.target));
        }
        let output = DirectRiscvBuilder::build_bin(p, out)?;
        Ok(RisCvCheriBuildOutput { bin_path: output.bin_path, bin_size: output.bin_size })
    }
}
