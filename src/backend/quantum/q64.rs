// 64-qubit quantum circuit compiler. Same shit as the 8-qubit one but adds: circuit optimisation,
// OpenQASM export and classical control via measurement feedback

use crate::frontend::*;
use crate::backend::quantum::core::*;
use std::path::{Path, PathBuf};

pub struct Quantum64BuildOutput {
    pub bin_path: PathBuf,
    pub bin_size: usize,
    pub qasm: String,
}
pub struct DirectQuantum64Builder;

impl DirectQuantum64Builder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<Quantum64BuildOutput, String> {
        if p.target != "quantum64" {
            return Err(format!("need 'quantum64', got '{}'", p.target));
        }
        let mut ops = crate::backend::quantum::q8::qparse(p, 64)?;
        optimise_circuit(&mut ops);
        let qbin = crate::backend::quantum::q8::qencode(&ops, 64, 64);
        let qasm = to_qasm(&ops, 64, 64);
        std::fs::write(out, &qbin).map_err(|e| e.to_string())?;
        // Also write the QASM export alongside the binary
        if let Some(parent) = out.parent() {
            let qasm_path = parent.join(out.file_stem().unwrap()).with_extension("qasm");
            std::fs::write(&qasm_path, &qasm).map_err(|e| format!("qasm write: {e}"))?;
        }
        Ok(Quantum64BuildOutput {
            bin_path: out.to_path_buf(),
            bin_size: qbin.len(),
            qasm,
        })
    }
}
