use super::target::Target;

use super::arm::target::{Aarch64Target, ArmCmTarget, CheriTarget};
use super::console::target::{
    Arm7tdmiTarget, Arm9Target, Huc6280Target, Ppc740Target, Ppc970Target, V810Target,
};
use super::dsp::target::{BlackfinTarget, SharcTarget, Tms320Target};
use super::exotic::target::{
    DlxTarget, HarvardTarget, Ia64Target, JovialTarget, Lc3Target, MicroblazeTarget, Mico32Target,
    Mil1750aTarget, MillTarget, MmixTarget, Nios2Target, OpenriscTarget, PicoblazeTarget,
    TernaryTarget, VliwTarget,
};
use super::m68k::target::{M68kTarget, ShTarget};
use super::mcu::target::{
    ArduinoEsp32Target, AvrTarget, C166Target, FrTarget, H8Target, M16cTarget, Msp430Target,
    Nec78kTarget, PicTarget, R8cTarget, Rl78Target, RxTarget, Xc800Target,
};
use super::quantum::target::{Quantum64Target, Quantum8Target};
use super::risc::target::{AlphaTarget, MipsTarget, PariscTarget, PpcTarget, SparcTarget};
use super::riscv::target::{RiscvCheriTarget, RiscvTarget};
use super::soviet::target::{BesmTarget, ElbrusTarget, MirTarget, UralTarget};
use super::vintage::target::{
    Cdc6600Target, Hp3000Target, M6502Target, M6800Target, M6809Target, Mos6501Target, Pdp11Target,
    Pdp8Target, S360Target, UnivacTarget, VaxTarget, Z80Target, ZArchTarget,
};
use super::x86::target::{
    I4004Target, I8008Target, I8080Target, I8086Target, NecV20Target, UefiTarget, X86_64Target,
};
use crate::formats::target::{BiosTarget, Elf32Target, ElfTarget, MachoTarget, Win32Target};
use crate::vm::target::{PortableTarget, VmTarget};

static TARGETS: &[(&str, &dyn Target)] = &[
    ("6502", &M6502Target),
    ("6809", &M6809Target),
    ("aarch64", &Aarch64Target),
    ("alpha", &AlphaTarget),
    ("arduino_nano_esp32", &ArduinoEsp32Target),
    ("arm7tdmi", &Arm7tdmiTarget),
    ("arm9", &Arm9Target),
    ("armcm", &ArmCmTarget),
    ("avr", &AvrTarget),
    ("besm", &BesmTarget),
    ("bios16", &BiosTarget),
    ("blackfin", &BlackfinTarget),
    ("c166", &C166Target),
    ("cdc6600", &Cdc6600Target),
    ("cheri", &CheriTarget),
    ("dlx", &DlxTarget),
    ("elbrus", &ElbrusTarget),
    ("elf32", &Elf32Target),
    ("elf64", &ElfTarget),
    ("fr", &FrTarget),
    ("h8", &H8Target),
    ("harvard", &HarvardTarget),
    ("hp3000", &Hp3000Target),
    ("huc6280", &Huc6280Target),
    ("i4004", &I4004Target),
    ("i8008", &I8008Target),
    ("i8080", &I8080Target),
    ("i8086", &I8086Target),
    ("ia64", &Ia64Target),
    ("jovial", &JovialTarget),
    ("lc3", &Lc3Target),
    ("m6800", &M6800Target),
    ("m68k", &M68kTarget),
    ("m16c", &M16cTarget),
    ("macho", &MachoTarget),
    ("microblaze", &MicroblazeTarget),
    ("mico32", &Mico32Target),
    ("mil1750a", &Mil1750aTarget),
    ("mill", &MillTarget),
    ("mips", &MipsTarget),
    ("mir", &MirTarget),
    ("mmix", &MmixTarget),
    ("mos6501", &Mos6501Target),
    ("msp430", &Msp430Target),
    ("nec78k", &Nec78kTarget),
    ("nios2", &Nios2Target),
    ("openrisc", &OpenriscTarget),
    ("parisc", &PariscTarget),
    ("pdp11", &Pdp11Target),
    ("pdp8", &Pdp8Target),
    ("pic", &PicTarget),
    ("picoblaze", &PicoblazeTarget),
    ("portable", &PortableTarget),
    ("ppc", &PpcTarget),
    ("ppc740", &Ppc740Target),
    ("ppc970", &Ppc970Target),
    ("quantum64", &Quantum64Target),
    ("quantum8", &Quantum8Target),
    ("r8c", &R8cTarget),
    ("riscv", &RiscvTarget),
    ("riscv64", &RiscvTarget),
    ("riscv_cheri", &RiscvCheriTarget),
    ("rl78", &Rl78Target),
    ("rx", &RxTarget),
    ("s360", &S360Target),
    ("sh2", &ShTarget),
    ("sh4", &ShTarget),
    ("sharc", &SharcTarget),
    ("sparc", &SparcTarget),
    ("ternary", &TernaryTarget),
    ("tms320", &Tms320Target),
    ("uefi", &UefiTarget),
    ("univac", &UnivacTarget),
    ("ural", &UralTarget),
    ("v20", &NecV20Target),
    ("v810", &V810Target),
    ("vax", &VaxTarget),
    ("vliw", &VliwTarget),
    ("vm", &VmTarget),
    ("win32", &Win32Target),
    ("x86_64", &X86_64Target),
    ("xc800", &Xc800Target),
    ("z80", &Z80Target),
    ("zarch", &ZArchTarget),
];

pub struct TargetRegistry;

impl TargetRegistry {
    pub fn get(name: &str) -> Option<&'static dyn Target> {
        TARGETS.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
    }
    pub fn names() -> Vec<&'static str> {
        TARGETS.iter().map(|(n, _)| *n).collect()
    }
}
