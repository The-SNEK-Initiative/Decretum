use std::path::{Path, PathBuf};

use crate::frontend::Program;

use super::super::target::{OutputKind, Target};
use super::{
    DirectIa64Builder, DirectMicroblazeBuilder, DirectNios2Builder, DirectOpenriscBuilder,
    DirectTernaryBuilder, DirectVliwBuilder, DlxBuilder, HarvardBuilder, JovialBuilder, Lc3Builder,
    Mico32Builder, Mil1750aBuilder, MillBuilder, MmixBuilder, PicoblazeBuilder,
};

pub struct TernaryTarget;

impl Target for TernaryTarget {
    fn name(&self) -> &'static str {
        "ternary"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectTernaryBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct VliwTarget;

impl Target for VliwTarget {
    fn name(&self) -> &'static str {
        "vliw"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectVliwBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Ia64Target;

impl Target for Ia64Target {
    fn name(&self) -> &'static str {
        "ia64"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectIa64Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct MillTarget;

impl Target for MillTarget {
    fn name(&self) -> &'static str {
        "mill"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        MillBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct HarvardTarget;

impl Target for HarvardTarget {
    fn name(&self) -> &'static str {
        "harvard"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        HarvardBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Mil1750aTarget;

impl Target for Mil1750aTarget {
    fn name(&self) -> &'static str {
        "mil1750a"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Mil1750aBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct JovialTarget;

impl Target for JovialTarget {
    fn name(&self) -> &'static str {
        "jovial"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        JovialBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Mico32Target;

impl Target for Mico32Target {
    fn name(&self) -> &'static str {
        "mico32"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Mico32Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct PicoblazeTarget;

impl Target for PicoblazeTarget {
    fn name(&self) -> &'static str {
        "picoblaze"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        PicoblazeBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct MmixTarget;

impl Target for MmixTarget {
    fn name(&self) -> &'static str {
        "mmix"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        MmixBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct DlxTarget;

impl Target for DlxTarget {
    fn name(&self) -> &'static str {
        "dlx"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DlxBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Lc3Target;

impl Target for Lc3Target {
    fn name(&self) -> &'static str {
        "lc3"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        Lc3Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct OpenriscTarget;

impl Target for OpenriscTarget {
    fn name(&self) -> &'static str {
        "openrisc"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectOpenriscBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct Nios2Target;

impl Target for Nios2Target {
    fn name(&self) -> &'static str {
        "nios2"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectNios2Builder::build_bin(program, out).map(|o| o.bin_path)
    }
}

pub struct MicroblazeTarget;

impl Target for MicroblazeTarget {
    fn name(&self) -> &'static str {
        "microblaze"
    }
    fn output_kind(&self) -> OutputKind {
        OutputKind::Binary
    }
    fn build(&self, program: &Program, out: &Path) -> Result<PathBuf, String> {
        DirectMicroblazeBuilder::build_bin(program, out).map(|o| o.bin_path)
    }
}
