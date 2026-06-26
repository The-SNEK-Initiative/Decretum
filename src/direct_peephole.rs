// Generic peephole optimiser shared by most backends. Each backend passes
// closures that identify NOPs, branches, returns, and labels.
// The pass collapses consecutive NOPs and drops dead code after unconditional branches and returns.

// Peephole-optimise a flat instruction list.
// - `is_nop` - returns true for no op / filler instructions
// - `is_uncond_branch` - returns true for unconditional branches / jumps / calls
// - `is_ret` - returns true for return / halt / exit instructions
// - `is_label` - returns true for label / marker pseudo-instructions
//

pub fn peephole<T, F1, F2, F3, F4>(
    instrs: &mut Vec<T>,
    is_nop: F1,
    is_uncond_branch: F2,
    is_ret: F3,
    is_label: F4,
)
where
    F1: Fn(&T) -> bool,
    F2: Fn(&T) -> bool,
    F3: Fn(&T) -> bool,
    F4: Fn(&T) -> bool,
{
    let mut i = 0;
    while i + 1 < instrs.len() {
        if is_nop(&instrs[i]) && is_nop(&instrs[i + 1]) {
            instrs.remove(i + 1);
        } else {
            i += 1;
        }
    }

    i = 0;
    while i < instrs.len() {
        let is_terminator = is_uncond_branch(&instrs[i]) || is_ret(&instrs[i]);
        if is_terminator {
            // Scan forward - delete everything until we hit a label
            let mut j = i + 1;
            while j < instrs.len() && !is_label(&instrs[j]) {
                j += 1;
            }
            if j > i + 1 {
                let dead = j - (i + 1);
                instrs.drain(i + 1..j);
            }
            i = if is_label(&instrs[i]) { i + 1 } else { i + 1 };
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum MockInst { Label(u8), Nop, Add, Jmp, Ret }

    #[test]
    fn test_nop_collapse() {
        let mut v = vec![MockInst::Nop, MockInst::Nop, MockInst::Nop, MockInst::Add];
        peephole(&mut v,
            |i| matches!(i, MockInst::Nop),
            |i| matches!(i, MockInst::Jmp),
            |i| matches!(i, MockInst::Ret),
            |i| matches!(i, MockInst::Label(_)),
        );
        assert_eq!(v, vec![MockInst::Nop, MockInst::Add]);
    }

    #[test]
    fn test_dead_code_after_jmp() {
        let mut v = vec![
            MockInst::Label(0), MockInst::Jmp,
            MockInst::Add, MockInst::Add, MockInst::Label(1),
        ];
        peephole(&mut v,
            |i| matches!(i, MockInst::Nop),
            |i| matches!(i, MockInst::Jmp),
            |i| matches!(i, MockInst::Ret),
            |i| matches!(i, MockInst::Label(_)),
        );
        assert_eq!(v, vec![MockInst::Label(0), MockInst::Jmp, MockInst::Label(1)]);
    }

    #[test]
    fn test_dead_code_after_ret() {
        let mut v = vec![
            MockInst::Add, MockInst::Ret,
            MockInst::Add, MockInst::Add,
        ];
        peephole(&mut v,
            |i| matches!(i, MockInst::Nop),
            |i| matches!(i, MockInst::Jmp),
            |i| matches!(i, MockInst::Ret),
            |i| matches!(i, MockInst::Label(_)),
        );
        assert_eq!(v, vec![MockInst::Add, MockInst::Ret]);
    }
}
