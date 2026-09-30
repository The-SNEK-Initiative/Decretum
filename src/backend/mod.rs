pub mod arm;
pub mod common;
pub mod console;
pub mod dsp;
pub mod exotic;
pub mod m68k;
pub mod mcu;
pub mod quantum;
pub mod registry;
pub mod risc;
pub mod riscv;
pub mod soviet;
pub mod target;
pub mod vintage;
pub mod x86;

pub use common::{LabelMap, expand_str};
pub use registry::TargetRegistry;
pub use target::{OutputKind, Target};
