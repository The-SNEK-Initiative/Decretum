// Shared helpers for direct machine code backends

use std::collections::BTreeMap;

pub fn expand_str(s: &str) -> Vec<u8> {
    let mut b = Vec::new();
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' && i + 1 < c.len() {
            match c[i + 1] { 'n' => b.push(b'\n'), 'r' => b.push(b'\r'), 't' => b.push(b'\t'),
                '0' => b.push(0), '\\' => b.push(b'\\'), '"' => b.push(b'"'),
                o => { b.push(b'\\'); b.push(o as u8); } }
            i += 2;
        } else { b.push(c[i] as u8); i += 1; }
    }
    b
}

pub type LabelMap = BTreeMap<String, u32>;
