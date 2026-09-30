// Byte lvl optimisation pass. Strips trailing filler bytes from compiled binaries.
// Most architectures pad sections with zeros or NOPs after the useful code, and this cleans that up.

// Collapse consecutive filler bytes down to one.
// e.g. `00 00 00 00 00` -> `00`
pub fn dedup_filler_bytes(bin: &mut Vec<u8>, filler: u8) {
    let mut i = 0;
    while i + 1 < bin.len() {
        if bin[i] == filler && bin[i + 1] == filler {
            bin.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

// Remove all NOP style bytes from the end of the binary.
pub fn strip_trailing_nops(bin: &mut Vec<u8>, nop_bytes: &[u8]) {
    while !bin.is_empty() {
        let last = bin[bin.len() - 1];
        if nop_bytes.contains(&last) {
            bin.pop();
        } else {
            break;
        }
    }
}

// Remove trailing zeros (common filler for word aligned RISC sections).
pub fn strip_trailing_zeros(bin: &mut Vec<u8>) {
    while !bin.is_empty() && bin[bin.len() - 1] == 0 {
        bin.pop();
    }
}

// Remove trailing bytes matching any value in `trailing`.
pub fn strip_trailing(bin: &mut Vec<u8>, trailing: &[u8]) {
    strip_trailing_nops(bin, trailing);
}

// Full optimise pass for a given architecture.
// `filler` - byte value that represents a NOP / no op fill (e.g. 0x00 for RISC, 0xEA for 6502).
// `trailing` - bytes that are safe to strip from the end.
pub fn optimise(bin: &mut Vec<u8>, filler: u8, trailing: &[u8]) {
    // Deduplicate consecutive filler bytes
    dedup_filler_bytes(bin, filler);
    // Strip any trailing filler / safe bytes
    let mut safe = trailing.to_vec();
    if !safe.contains(&filler) {
        safe.push(filler);
    }
    strip_trailing(bin, &safe);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_filler() {
        let mut v = vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02];
        dedup_filler_bytes(&mut v, 0x00);
        assert_eq!(v, vec![0x00, 0x01, 0x00, 0x02]);
    }

    #[test]
    fn test_strip_trailing_zeros() {
        let mut v = vec![0x01, 0x02, 0x00, 0x00];
        strip_trailing_zeros(&mut v);
        assert_eq!(v, vec![0x01, 0x02]);
    }

    #[test]
    fn test_strip_trailing_zeros_none() {
        let mut v = vec![0x01, 0x02, 0x03];
        strip_trailing_zeros(&mut v);
        assert_eq!(v, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_strip_trailing_nops() {
        let mut v = vec![0x01, 0xEA, 0xEA];
        strip_trailing_nops(&mut v, &[0xEA]);
        assert_eq!(v, vec![0x01]);
    }

    #[test]
    fn test_optimise_risc() {
        let mut v = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        optimise(&mut v, 0x00, &[]);
        // After dedup: [0x00, 0x01, 0x00]
        // After strip: [0x00, 0x01]
        assert_eq!(v, vec![0x00, 0x01]);
    }
}
