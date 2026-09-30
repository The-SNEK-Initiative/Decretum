pub mod console;

pub use console::{
    Arm7tdmiBuildOutput, Arm7tdmiBuilder, Arm9BuildOutput, Arm9Builder, HuC6280BuildOutput,
    HuC6280Builder, Ppc740BuildOutput, Ppc740Builder, Ppc970BuildOutput, Ppc970Builder,
    V810BuildOutput, V810Builder,
};
pub mod target;
