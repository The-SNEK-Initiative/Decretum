use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    C166Builder, DirectArduinoEsp32Builder, DirectAvrBuilder, DirectPICBuilder, FrBuilder,
    H8Builder, M16cBuilder, Msp430Builder, Nec78kBuilder, R8cBuilder, Rl78Builder, RxBuilder,
    Xc800Builder,
};

pub struct PicTarget;

impl Target for PicTarget {
    fn name(&self) -> &'static str {
        "pic"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectPICBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct AvrTarget;

impl Target for AvrTarget {
    fn name(&self) -> &'static str {
        "avr"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectAvrBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct ArduinoEsp32Target;

impl Target for ArduinoEsp32Target {
    fn name(&self) -> &'static str {
        "arduino_nano_esp32"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectArduinoEsp32Builder::build_app(program, out).map(|o| o.app_path)
    }
}

pub struct C166Target;

impl Target for C166Target {
    fn name(&self) -> &'static str {
        "c166"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        C166Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Xc800Target;

impl Target for Xc800Target {
    fn name(&self) -> &'static str {
        "xc800"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Xc800Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct M16cTarget;

impl Target for M16cTarget {
    fn name(&self) -> &'static str {
        "m16c"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        M16cBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct R8cTarget;

impl Target for R8cTarget {
    fn name(&self) -> &'static str {
        "r8c"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        R8cBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Rl78Target;

impl Target for Rl78Target {
    fn name(&self) -> &'static str {
        "rl78"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Rl78Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct RxTarget;

impl Target for RxTarget {
    fn name(&self) -> &'static str {
        "rx"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        RxBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct H8Target;

impl Target for H8Target {
    fn name(&self) -> &'static str {
        "h8"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        H8Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Msp430Target;

impl Target for Msp430Target {
    fn name(&self) -> &'static str {
        "msp430"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Msp430Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Nec78kTarget;

impl Target for Nec78kTarget {
    fn name(&self) -> &'static str {
        "nec78k"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Nec78kBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct FrTarget;

impl Target for FrTarget {
    fn name(&self) -> &'static str {
        "fr"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        FrBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
