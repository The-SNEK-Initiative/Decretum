// 8-qubit quantum circuit compiler. Shares the core representation and
// optimisers with quantum64 but targets smaller circuits

use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use crate::dcrt::*;
use crate::direct_quantum_core::*;

pub struct Quantum8BuildOutput { pub bin_path: PathBuf, pub bin_size: usize }
pub struct DirectQuantum8Builder;

impl DirectQuantum8Builder {
    pub fn build_bin(p: &Program, out: &Path) -> Result<Quantum8BuildOutput, String> {
        if p.target != "quantum8" {
            return Err(format!("need 'quantum8', got '{}'", p.target));
        }
        let ops = qparse(p, 8)?;
        let qbin = qencode(&ops, 8, 8);
        std::fs::write(out, &qbin).map_err(|e| e.to_string())?;
        Ok(Quantum8BuildOutput { bin_path: out.to_path_buf(), bin_size: qbin.len() })
    }
}

pub fn qparse(p: &Program, max_qubits: u8) -> Result<Vec<QOp>, String> {
    let mut ops = Vec::new();
    let mut entry_found = false;

    for b in &p.blocks {
        let pr = match b.kind { BlockKind::Event => "__event_", BlockKind::Proc => "__proc_" };
        if format!("__event_{}", p.entry_event) == format!("{}{}", pr, b.name) {
            entry_found = true;
        }

        for l in &b.lines {
            let t = l.trim();
            if t.is_empty() || t.starts_with(';') || t.ends_with(':') { continue; }
            if t.starts_with("emit ") || t.starts_with("call ") { continue; }
            if t == "ret" || t == "hlt" { continue; }

            let parts: Vec<&str> = t.split(|c: char| c == ' ' || c == '\t').filter(|s|!s.is_empty()).collect();
            if parts.is_empty() { continue; }
            let m = parts[0];

            let qp = |s: &str| -> Result<u8, String> {
                let s = s.trim_start_matches('q').trim_start_matches('[').trim_end_matches(']').trim();
                let q = s.parse::<u8>().map_err(|_| format!("bad qubit '{}'", s))?;
                if q >= max_qubits { return Err(format!("qubit {} out of range for this target (max {})", q, max_qubits - 1)); }
                Ok(q)
            };
            let cp = |s: &str| -> Result<u8, String> {
                let s = s.trim_start_matches('c').trim_start_matches('[').trim_end_matches(']').trim();
                s.parse::<u8>().map_err(|_| format!("bad cbit '{}'", s))
            };
            let ang = |s: &str| -> Result<f64, String> {
                s.parse::<f64>().map_err(|_| format!("bad angle '{}'", s))
            };

            let args_joined = parts[1..].join(" ");
            let args: Vec<&str> = args_joined.split(',').map(|s| s.trim()).filter(|s|!s.is_empty()).collect();

            let op = match m {
                "h"|"hadamard" if args.len() == 1 => Some(QOp { gate: Gate::H, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "x" if args.len() == 1 => Some(QOp { gate: Gate::X, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "y" if args.len() == 1 => Some(QOp { gate: Gate::Y, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "z" if args.len() == 1 => Some(QOp { gate: Gate::Z, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "cx"|"cnot" if args.len() == 2 => Some(QOp { gate: Gate::CX, qubits: vec![qp(args[0])?, qp(args[1])?], cbits: vec![], angle: 0.0 }),
                "ccx"|"toffoli" if args.len() == 3 => Some(QOp { gate: Gate::CCX, qubits: vec![qp(args[0])?, qp(args[1])?, qp(args[2])?], cbits: vec![], angle: 0.0 }),
                "swap" if args.len() == 2 => Some(QOp { gate: Gate::Swap, qubits: vec![qp(args[0])?, qp(args[1])?], cbits: vec![], angle: 0.0 }),
                "s" if args.len() == 1 => Some(QOp { gate: Gate::S, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "t" if args.len() == 1 => Some(QOp { gate: Gate::T, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "sdg"|"sdag" if args.len() == 1 => Some(QOp { gate: Gate::Sdg, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "tdg"|"tdag" if args.len() == 1 => Some(QOp { gate: Gate::Tdg, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "rx" if args.len() == 2 => Some(QOp { gate: Gate::Rx(ang(args[1])?), qubits: vec![qp(args[0])?], cbits: vec![], angle: ang(args[1])? * PI / 180.0 }),
                "ry" if args.len() == 2 => Some(QOp { gate: Gate::Ry(ang(args[1])?), qubits: vec![qp(args[0])?], cbits: vec![], angle: ang(args[1])? * PI / 180.0 }),
                "rz" if args.len() == 2 => Some(QOp { gate: Gate::Rz(ang(args[1])?), qubits: vec![qp(args[0])?], cbits: vec![], angle: ang(args[1])? * PI / 180.0 }),
                "measure" if args.len() == 2 => Some(QOp { gate: Gate::Measure, qubits: vec![qp(args[0])?], cbits: vec![cp(args[1])?], angle: 0.0 }),
                "reset" if args.len() == 1 => Some(QOp { gate: Gate::Reset, qubits: vec![qp(args[0])?], cbits: vec![], angle: 0.0 }),
                "barrier" => Some(QOp { gate: Gate::Barrier, qubits: vec![], cbits: vec![], angle: 0.0 }),
                _ => return Err(format!("unknown quantum op '{}'", m)),
            };
            if let Some(op) = op { ops.push(op); }
        }
    }
    if !entry_found { return Err(format!("entry event '{}' not found", p.entry_event)); }
    Ok(ops)
}

pub fn qencode(ops: &[QOp], nqubits: u8, _ncbits: u8) -> Vec<u8> {
    let mut bin: Vec<u8> = b"QNTM".to_vec();
    bin.extend_from_slice(&(nqubits as u32).to_le_bytes());
    bin.extend_from_slice(&(ops.len() as u32).to_le_bytes());
    bin.extend_from_slice(&0u32.to_le_bytes());
    for op in ops {
        let (code, q2) = match op.gate {
            Gate::H => (0x01, 0), Gate::X => (0x02, 0), Gate::Y => (0x03, 0), Gate::Z => (0x04, 0),
            Gate::CX => (0x05, op.qubits.get(1).copied().unwrap_or(0)),
            Gate::CCX => (0x06, op.qubits.get(1).copied().unwrap_or(0)),
            Gate::Swap => (0x07, op.qubits.get(1).copied().unwrap_or(0)),
            Gate::S => (0x08, 0), Gate::T => (0x09, 0),
            Gate::Sdg => (0x0A, 0), Gate::Tdg => (0x0B, 0),
            Gate::Rx(_) => (0x10, 0), Gate::Ry(_) => (0x11, 0), Gate::Rz(_) => (0x12, 0),
            Gate::Measure => (0x20, op.cbits.first().copied().unwrap_or(0)),
            Gate::Reset => (0x21, 0), Gate::Barrier => (0xFF, 0),
        };
        let q1 = op.qubits.first().copied().unwrap_or(0);
        let ang_f16 = encode_angle(op.angle);
        bin.push(code); bin.push(q1); bin.push(q2);
        bin.extend_from_slice(&ang_f16.to_le_bytes());
    }
    bin
}
