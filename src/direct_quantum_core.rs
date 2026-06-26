// Shared quantum circuit representation and optimisations
// Used by both quantum8 and quantum64 backends

use std::f64::consts::PI;

pub const MAX_QUBITS: usize = 64;
pub const MAX_CBITS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gate {
    H, X, Y, Z, S, T, Sdg, Tdg,
    CX, CCX, Swap,
    Rx(f64), Ry(f64), Rz(f64),
    Measure, Reset, Barrier,
}

#[derive(Clone, Debug)]
pub struct QOp {
    pub gate: Gate,
    pub qubits: Vec<u8>,    // target qubit(s)
    pub cbits: Vec<u8>,     // classical bit(s) for measure
    pub angle: f64,         // rotation angle in radians (for Rx/Ry/Rz)
}

// Encode a rotation angle to 16-bit fixed point
// Maps [-PI, PI] to [0, 65535]; 0 radians = 0x8000
pub fn encode_angle(rad: f64) -> u16 {
    let mut a = rad % (2.0 * PI);
    if a > PI { a -= 2.0 * PI; } else if a < -PI { a += 2.0 * PI; }
    ((a / PI) * 32768.0 + 32768.0).round().max(0.0).min(65535.0) as u16
}

// Decode a 16-bit fixed point angle back to radians
pub fn decode_angle(enc: u16) -> f64 {
    ((enc as f64) - 32768.0) / 32768.0 * PI
}


// Remove consecutive barriers (they don't affect the computation)
pub fn remove_redundant_barriers(ops: &mut Vec<QOp>) {
    ops.retain(|op| op.gate != Gate::Barrier);
}

// Cancel adjacent inverse gate pairs: H H = I, X X = I, Y Y = I, Z Z = I
pub fn cancel_adjacent_inverses(ops: &mut Vec<QOp>) {
    let mut i = 0;
    while i + 1 < ops.len() {
        let same_qubits = ops[i].qubits == ops[i+1].qubits;
        let self_inverse = |g: Gate| matches!(g, Gate::H|Gate::X|Gate::Y|Gate::Z);
        if same_qubits && ops[i].gate == ops[i+1].gate && self_inverse(ops[i].gate) {
            ops.remove(i);
            ops.remove(i);
            if i > 0 { i -= 1; }
            continue;
        }
        i += 1;
    }
}

// Merge adjacent rotation gates on the same qubit: Rz(a) Rz(b) = Rz(a+b)
pub fn merge_adjacent_rotations(ops: &mut Vec<QOp>) {
    let mut i = 0;
    while i + 1 < ops.len() {
        let same_q = ops[i].qubits == ops[i+1].qubits;
        let gate_pair = (ops[i].gate, ops[i+1].gate);
        let merged = match gate_pair {
            (Gate::Rx(a), Gate::Rx(b)) if same_q => Some(Gate::Rx(a + b)),
            (Gate::Ry(a), Gate::Ry(b)) if same_q => Some(Gate::Ry(a + b)),
            (Gate::Rz(a), Gate::Rz(b)) if same_q => Some(Gate::Rz(a + b)),
            _ => None,
        };
        if let Some(g) = merged {
            ops[i].gate = g;
            ops[i].angle = match g { Gate::Rx(a)|Gate::Ry(a)|Gate::Rz(a) => a, _ => 0.0 };
            ops.remove(i + 1);
            continue;
        }
        i += 1;
    }
}

// Cancel CNOT pairs: CX(a,b) CX(a,b) = I
pub fn cancel_cnot_pairs(ops: &mut Vec<QOp>) {
    let mut i = 0;
    while i + 1 < ops.len() {
        if ops[i].gate == Gate::CX && ops[i+1].gate == Gate::CX
            && ops[i].qubits == ops[i+1].qubits {
            ops.remove(i);
            ops.remove(i);
            if i > 0 { i -= 1; }
            continue;
        }
        i += 1;
    }
}

// Remove measurements that target unused classical bits
pub fn remove_unused_measurements(ops: &mut Vec<QOp>) {
    let mut cbits_used = vec![false; MAX_CBITS];
    for op in ops.iter() {
        if op.gate == Gate::Measure {
            continue;
        }
        for &c in &op.cbits {
            if (c as usize) < MAX_CBITS {
                cbits_used[c as usize] = true;
            }
        }
    }
    ops.retain(|op| {
        if op.gate != Gate::Measure {
            return true;
        }
        op.cbits.iter().any(|&c| (c as usize) < MAX_CBITS && cbits_used[c as usize])
    });
}

// Run all optimisation passes on a circuit
pub fn optimise_circuit(ops: &mut Vec<QOp>) {
    remove_redundant_barriers(ops);
    cancel_adjacent_inverses(ops);
    merge_adjacent_rotations(ops);
    cancel_cnot_pairs(ops);
    remove_unused_measurements(ops);
}

// OpenQASM export!!

// Export a circuit to OpenQASM 2.0 format
pub fn to_qasm(ops: &[QOp], nqubits: usize, ncbits: usize) -> String {
    let mut qasm = String::new();
    qasm.push_str("OPENQASM 2.0;\n");
    qasm.push_str("include \"qelib1.inc\";\n");
    qasm.push_str(&format!("qreg q[{}];\n", nqubits));
    if ncbits > 0 {
        qasm.push_str(&format!("creg c[{}];\n", ncbits));
    }
    for op in ops {
        let qstr: Vec<String> = op.qubits.iter().map(|q| format!("q[{}]", q)).collect();
        let line = match op.gate {
            Gate::H => format!("h {};", qstr[0]),
            Gate::X => format!("x {};", qstr[0]),
            Gate::Y => format!("y {};", qstr[0]),
            Gate::Z => format!("z {};", qstr[0]),
            Gate::S => format!("s {};", qstr[0]),
            Gate::T => format!("t {};", qstr[0]),
            Gate::Sdg => format!("sdg {};", qstr[0]),
            Gate::Tdg => format!("tdg {};", qstr[0]),
            Gate::CX => format!("cx {}, {};", qstr[0], qstr[1]),
            Gate::CCX => format!("ccx {}, {}, {};", qstr[0], qstr[1], qstr[2]),
            Gate::Swap => format!("swap {}, {};", qstr[0], qstr[1]),
            Gate::Rx(a) => format!("rx({}) {};", a, qstr[0]),
            Gate::Ry(a) => format!("ry({}) {};", a, qstr[0]),
            Gate::Rz(a) => format!("rz({}) {};", a, qstr[0]),
            Gate::Measure => {
                let cstr: Vec<String> = op.cbits.iter().map(|c| format!("c[{}]", c)).collect();
                format!("measure {} -> {};", qstr[0], cstr[0])
            }
            Gate::Reset => format!("reset {};", qstr[0]),
            Gate::Barrier => format!("barrier {};", qstr.join(",")),
        };
        qasm.push_str(&line);
        qasm.push('\n');
    }
    qasm
}

// Parse a simple OpenQASM 2.0 subset into a circuit
pub fn from_qasm(source: &str) -> Result<(Vec<QOp>, usize, usize), String> {
    let mut ops = Vec::new();
    let mut nqubits = 0usize;
    let mut ncbits = 0usize;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("OPENQASM") || trimmed.starts_with("include") || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("qreg ") {
            let parts: Vec<&str> = rest.trim_end_matches(';').split('[').collect();
            if parts.len() == 2 {
                let n = parts[1].trim_end_matches(']').parse::<usize>().unwrap_or(0);
                nqubits = nqubits.max(n);
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("creg ") {
            let parts: Vec<&str> = rest.trim_end_matches(';').split('[').collect();
            if parts.len() == 2 {
                let n = parts[1].trim_end_matches(']').parse::<usize>().unwrap_or(0);
                ncbits = ncbits.max(n);
            }
            continue;
        }

        let parts: Vec<&str> = trimmed.trim_end_matches(';').split(|c| c == ' ' || c == '(' || c == ')' || c == ',').filter(|s|!s.is_empty()).collect();
        if parts.is_empty() { continue; }

        let parse_q = |s: &str| s.trim_start_matches('q').trim_start_matches('[').trim_end_matches(']').parse::<u8>().unwrap_or(0);
        let parse_c = |s: &str| s.trim_start_matches('c').trim_start_matches('[').trim_end_matches(']').parse::<u8>().unwrap_or(0);

        let op = match parts[0] {
            "h" => Some(QOp { gate: Gate::H, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "x" => Some(QOp { gate: Gate::X, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "y" => Some(QOp { gate: Gate::Y, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "z" => Some(QOp { gate: Gate::Z, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "cx" => Some(QOp { gate: Gate::CX, qubits: vec![parse_q(parts[1]), parse_q(parts[2])], cbits: vec![], angle: 0.0 }),
            "ccx" => Some(QOp { gate: Gate::CCX, qubits: vec![parse_q(parts[1]), parse_q(parts[2]), parse_q(parts[3])], cbits: vec![], angle: 0.0 }),
            "swap" => Some(QOp { gate: Gate::Swap, qubits: vec![parse_q(parts[1]), parse_q(parts[2])], cbits: vec![], angle: 0.0 }),
            "s" => Some(QOp { gate: Gate::S, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "t" => Some(QOp { gate: Gate::T, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "sdg" => Some(QOp { gate: Gate::Sdg, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "tdg" => Some(QOp { gate: Gate::Tdg, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "rx" => Some(QOp { gate: Gate::Rx(0.0), qubits: vec![parse_q(parts[2])], cbits: vec![], angle: parts[1].parse::<f64>().unwrap_or(0.0) }),
            "ry" => Some(QOp { gate: Gate::Ry(0.0), qubits: vec![parse_q(parts[2])], cbits: vec![], angle: parts[1].parse::<f64>().unwrap_or(0.0) }),
            "rz" => Some(QOp { gate: Gate::Rz(0.0), qubits: vec![parse_q(parts[2])], cbits: vec![], angle: parts[1].parse::<f64>().unwrap_or(0.0) }),
            "measure" => {
                let parts2: Vec<&str> = trimmed.split("->").collect();
                if parts2.len() == 2 {
                    let q = parse_q(parts2[0].trim_start_matches("measure "));
                    let c = parse_c(parts2[1].trim());
                    Some(QOp { gate: Gate::Measure, qubits: vec![q], cbits: vec![c], angle: 0.0 })
                } else { None }
            }
            "reset" => Some(QOp { gate: Gate::Reset, qubits: vec![parse_q(parts[1])], cbits: vec![], angle: 0.0 }),
            "barrier" => Some(QOp { gate: Gate::Barrier, qubits: vec![], cbits: vec![], angle: 0.0 }),
            _ => None,
        };
        if let Some(op) = op { ops.push(op); }
    }
    Ok((ops, nqubits, ncbits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_angle() {
        let test_vals = [0.0, PI/2.0, PI, -PI, -PI/2.0, 0.5, -0.5];
        for &v in &test_vals {
            let enc = encode_angle(v);
            let dec = decode_angle(enc);
            let diff = (dec - v).abs();
            assert!(diff < 0.001, "angle mismatch for {}: enc={} dec={}", v, enc, dec);
        }
    }

    #[test]
    fn test_cancel_hh() {
        let mut ops = vec![
            QOp { gate: Gate::H, qubits: vec![0], cbits: vec![], angle: 0.0 },
            QOp { gate: Gate::H, qubits: vec![0], cbits: vec![], angle: 0.0 },
        ];
        cancel_adjacent_inverses(&mut ops);
        assert!(ops.is_empty());
    }

    #[test]
    fn test_merge_rz() {
        let mut ops = vec![
            QOp { gate: Gate::Rz(0.5), qubits: vec![0], cbits: vec![], angle: 0.5 },
            QOp { gate: Gate::Rz(0.3), qubits: vec![0], cbits: vec![], angle: 0.3 },
        ];
        merge_adjacent_rotations(&mut ops);
        assert_eq!(ops.len(), 1);
        if let Gate::Rz(a) = ops[0].gate { assert!((a - 0.8).abs() < 0.001); }
        else { panic!("expected Rz"); }
    }

    #[test]
    fn test_qasm_roundtrip() {
        let ops = vec![
            QOp { gate: Gate::H, qubits: vec![0], cbits: vec![], angle: 0.0 },
            QOp { gate: Gate::CX, qubits: vec![0, 1], cbits: vec![], angle: 0.0 },
            QOp { gate: Gate::Measure, qubits: vec![0], cbits: vec![0], angle: 0.0 },
        ];
        let qasm = to_qasm(&ops, 2, 1);
        let (parsed, nq, nc) = from_qasm(&qasm).unwrap();
        assert_eq!(nq, 2);
        assert_eq!(nc, 1);
        assert_eq!(parsed.len(), 3);
    }
}
