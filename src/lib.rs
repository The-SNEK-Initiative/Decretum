pub mod backend;
pub mod formats;
pub mod frontend;
pub mod optimizer;
pub mod tools;
pub mod vm;

pub use backend::{LabelMap, expand_str};
pub use backend::{OutputKind, Target, TargetRegistry};
pub use backend::arm::{Aarch64BuildOutput, DirectAarch64Builder};
pub use backend::arm::{ArmCmBuildOutput, DirectArmCmBuilder};
pub use backend::arm::{CheriBuildOutput, DirectCheriBuilder};
pub use backend::console::{
    Arm7tdmiBuildOutput, Arm7tdmiBuilder, Arm9BuildOutput, Arm9Builder, HuC6280BuildOutput,
    HuC6280Builder, Ppc740BuildOutput, Ppc740Builder, Ppc970BuildOutput, Ppc970Builder,
    V810BuildOutput, V810Builder,
};
pub use backend::dsp::{
    BlackfinBuildOutput, BlackfinBuilder, SharcBuildOutput, SharcBuilder, Tms320BuildOutput,
    Tms320Builder,
};
pub use backend::exotic::{
    DirectMicroblazeBuilder, DirectNios2Builder, DirectOpenriscBuilder, MicroblazeBuildOutput,
    Nios2BuildOutput, OpenriscBuildOutput,
};
pub use backend::exotic::{DirectTernaryBuilder, TernaryBuildOutput};
pub use backend::exotic::{DirectVliwBuilder, VliwBuildOutput};
pub use backend::exotic::{DirectIa64Builder, Ia64BuildOutput};
pub use backend::exotic::{HarvardBuildOutput, HarvardBuilder};
pub use backend::exotic::{Mil1750aBuildOutput, Mil1750aBuilder};
pub use backend::exotic::{MillBuildOutput, MillBuilder};
pub use backend::exotic::{JovialBuildOutput, JovialBuilder};
pub use backend::exotic::{
    DlxBuildOutput, DlxBuilder, Lc3BuildOutput, Lc3Builder, Mico32BuildOutput, Mico32Builder,
    MmixBuildOutput, MmixBuilder, PicoblazeBuildOutput, PicoblazeBuilder,
};
pub use backend::m68k::{
    DirectM68kBuilder, DirectSh2Builder, DirectSh4Builder, M68kBuildOutput, Sh2BuildOutput,
    Sh4BuildOutput,
};
pub use backend::mcu::{ArduinoEsp32BuildOutput, DirectArduinoEsp32Builder};
pub use backend::mcu::{AvrBuildOutput, DirectAvrBuilder, DirectPICBuilder, PicBuildOutput};
pub use backend::mcu::{C166BuildOutput, C166Builder};
pub use backend::mcu::{FrBuildOutput, FrBuilder};
pub use backend::mcu::{H8BuildOutput, H8Builder};
pub use backend::mcu::{M16cBuildOutput, M16cBuilder};
pub use backend::mcu::{Msp430BuildOutput, Msp430Builder};
pub use backend::mcu::{Nec78kBuildOutput, Nec78kBuilder};
pub use backend::mcu::{R8cBuildOutput, R8cBuilder};
pub use backend::mcu::{Rl78BuildOutput, Rl78Builder};
pub use backend::mcu::{RxBuildOutput, RxBuilder};
pub use backend::mcu::{Xc800BuildOutput, Xc800Builder};
pub use backend::quantum::core::*;
pub use backend::quantum::{DirectQuantum64Builder, Quantum64BuildOutput};
pub use backend::quantum::{DirectQuantum8Builder, Quantum8BuildOutput};
pub use backend::risc::{
    DirectMipsBuilder, DirectPpcBuilder, DirectSparcBuilder, MipsBuildOutput, PpcBuildOutput,
    SparcBuildOutput,
};
pub use backend::risc::{
    AlphaBuildOutput, DirectAlphaBuilder, DirectPariscBuilder, PariscBuildOutput,
};
pub use backend::riscv::{DirectRiscvBuilder, RiscvBuildOutput};
pub use backend::riscv::{DirectRisCvCheriBuilder, RisCvCheriBuildOutput};
pub use backend::soviet::{BesmBuildOutput, BesmBuilder};
pub use backend::soviet::{ElbrusBuildOutput, ElbrusBuilder};
pub use backend::soviet::{MirBuildOutput, MirBuilder};
pub use backend::soviet::{UralBuildOutput, UralBuilder};
pub use backend::vintage::{
    Direct6502Builder, Direct6809Builder, DirectZ80Builder, Six809BuildOutput,
    SixFiveOhTwoBuildOutput, Z80BuildOutput,
};
pub use backend::vintage::{S360BuildOutput, S360Builder, ZArchBuildOutput, ZArchBuilder};
pub use backend::vintage::{M6800BuildOutput, M6800Builder, Mos6501BuildOutput, Mos6501Builder};
pub use backend::vintage::{
    Hp3000BuildOutput, Hp3000Builder, Pdp8BuildOutput, Pdp8Builder, Pdp11BuildOutput, Pdp11Builder,
    VaxBuildOutput, VaxBuilder,
};
pub use backend::vintage::{Cdc6600BuildOutput, Cdc6600Builder, UnivacBuildOutput, UnivacBuilder};
pub use backend::x86::{
    I4004BuildOutput, I4004Builder, I8008BuildOutput, I8008Builder, I8080BuildOutput, I8080Builder,
    I8086BuildOutput, I8086Builder,
};
pub use backend::x86::{NecV20BuildOutput, NecV20Builder};
pub use backend::x86::{DirectUefiBuilder, UefiBuildOutput};
pub use backend::x86::{DirectX86_64Builder, X86_64BuildOutput};
pub use formats::{BootImageOutput, DirectBiosBuilder};
pub use formats::{DirectElfBuilder, ElfBuildOutput};
pub use formats::{DirectElf32Builder, Elf32BuildOutput};
pub use formats::{DirectMachoBuilder, MachoBuildOutput};
pub use formats::{DirectWin32Builder, Win32BuildOutput};
pub use frontend::{
    Block, BlockKind, DataDecl, ParseError, Parser, Program, ScalarWidth, collect_symbols,
};
pub use optimizer::{optimise, peephole};
pub use tools::{NativeStackBuildOutput, NativeStackBuilder, NativeStackModule};
pub use vm::{
    BytecodeBuildOutput, BytecodeEvent, BytecodeRuntime, PortableBuilder, PortablePeOutput,
};
pub use vm::{DirectVmBuilder, VmBuildOutput};
