use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    Cdc6600Builder, Direct6502Builder, Direct6809Builder, DirectZ80Builder, Hp3000Builder,
    M6800Builder, Mos6501Builder, Pdp11Builder, Pdp8Builder, S360Builder, UnivacBuilder, VaxBuilder,
    ZArchBuilder,
};

pub struct M6502Target;

impl Target for M6502Target {
    fn name(&self) -> &'static str {
        "6502"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Direct6502Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Z80Target;

impl Target for Z80Target {
    fn name(&self) -> &'static str {
        "z80"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectZ80Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct M6809Target;

impl Target for M6809Target {
    fn name(&self) -> &'static str {
        "6809"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Direct6809Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct M6800Target;

impl Target for M6800Target {
    fn name(&self) -> &'static str {
        "m6800"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        M6800Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Mos6501Target;

impl Target for Mos6501Target {
    fn name(&self) -> &'static str {
        "mos6501"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Mos6501Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Pdp8Target;

impl Target for Pdp8Target {
    fn name(&self) -> &'static str {
        "pdp8"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Pdp8Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Pdp11Target;

impl Target for Pdp11Target {
    fn name(&self) -> &'static str {
        "pdp11"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Pdp11Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct VaxTarget;

impl Target for VaxTarget {
    fn name(&self) -> &'static str {
        "vax"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        VaxBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Hp3000Target;

impl Target for Hp3000Target {
    fn name(&self) -> &'static str {
        "hp3000"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Hp3000Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct S360Target;

impl Target for S360Target {
    fn name(&self) -> &'static str {
        "s360"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        S360Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct ZArchTarget;

impl Target for ZArchTarget {
    fn name(&self) -> &'static str {
        "zarch"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        ZArchBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct UnivacTarget;

impl Target for UnivacTarget {
    fn name(&self) -> &'static str {
        "univac"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        UnivacBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Cdc6600Target;

impl Target for Cdc6600Target {
    fn name(&self) -> &'static str {
        "cdc6600"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Cdc6600Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}
