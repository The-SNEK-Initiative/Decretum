// Still needs some serious work done on it, but it exists. Kinda.
// I hate it tho, so just use VSC, or Vim, or whatever fancypants program you want to use.
// And if you wanna critique me for the IDE, either prove yourself better first by fixing it, or shut the fuck up.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(deprecated)]

#[path = "../docs_data.rs"]
mod docs_data;

use decretum::{
    BytecodeRuntime, Parser, PortableBuilder, Program,
    DirectBiosBuilder, DirectUefiBuilder, DirectRiscvBuilder, DirectRisCvCheriBuilder,
    DirectArmCmBuilder, DirectAarch64Builder, DirectX86_64Builder, DirectMachoBuilder,
    DirectElfBuilder, DirectCheriBuilder, DirectWin32Builder, DirectElf32Builder,
    DirectVmBuilder, DirectMipsBuilder, DirectPpcBuilder, DirectSparcBuilder,
    DirectAlphaBuilder, DirectPariscBuilder, DirectOpenriscBuilder, DirectNios2Builder,
    DirectMicroblazeBuilder, Direct6502Builder, DirectZ80Builder, Direct6809Builder,
    DirectPICBuilder, DirectAvrBuilder, DirectSh2Builder, DirectM68kBuilder,
    DirectTernaryBuilder, DirectQuantum8Builder, DirectQuantum64Builder,
    DirectIa64Builder, DirectVliwBuilder,
    S360Builder, ZArchBuilder, UnivacBuilder, Cdc6600Builder,
    Pdp8Builder, Pdp11Builder, VaxBuilder, Hp3000Builder,
    I4004Builder, I8008Builder, I8080Builder, I8086Builder,
    M6800Builder, Mos6501Builder,
    Tms320Builder, BlackfinBuilder, SharcBuilder,
    C166Builder, Xc800Builder, Rl78Builder, RxBuilder, H8Builder,
    Msp430Builder, NecV20Builder, Nec78kBuilder, M16cBuilder, R8cBuilder,
    FrBuilder, Mico32Builder, PicoblazeBuilder, MmixBuilder, DlxBuilder, Lc3Builder,
    HuC6280Builder, V810Builder, Arm7tdmiBuilder, Arm9Builder,
    Ppc740Builder, Ppc970Builder,
    Mil1750aBuilder, JovialBuilder, UralBuilder, BesmBuilder, ElbrusBuilder,
    MirBuilder, HarvardBuilder, MillBuilder,
};
use eframe::egui;
use eframe::egui::text::LayoutJob;
use eframe::egui::{FontId, TextFormat, ViewportBuilder};
use eframe::NativeOptions;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const IDE_ICON_PNG: &[u8] = include_bytes!("../../github_images/Decretum.png");

fn load_window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(IDE_ICON_PNG).ok()?;
    let rgba = img.to_rgba8();
    let scale_to = |img: &image::RgbaImage| -> image::RgbaImage {
        let (w, h) = img.dimensions();
        if w > 256 || h > 256 {
            let new_w = w.min(256);
            let new_h = h.min(256);
            image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else {
            img.clone()
        }
    };
    let scaled = scale_to(&rgba);
    let (w, h) = scaled.dimensions();
    Some(egui::IconData {
        rgba: scaled.into_raw(),
        width: w,
        height: h,
    })
}

fn main() -> Result<(), eframe::Error> {
    if let Some(result) = run_embedded_payload_if_present() {
        match result {
            Ok(code) => std::process::exit(code),
            Err(err) => {
                eprintln!("runtime error: {err}");
                std::process::exit(1);
            }
        }
    }

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1460.0, 920.0])
            .with_min_inner_size([1080.0, 720.0])
            .with_title("Decretum IDE")
            .with_icon(load_window_icon().unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native(
        "Decretum IDE",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            setup_style(&cc.egui_ctx);
            Ok(Box::new(DecretumIde::default()))
        }),
    )
}

fn run_embedded_payload_if_present() -> Option<Result<i32, String>> {
    let current_exe = std::env::current_exe().ok()?;
    let data = std::fs::read(&current_exe).ok()?;
    let magic = decretum::portable::dcrt_embed_magic();
    if data.len() < 20 || data[data.len() - 16..] != magic {
        return None;
    }
    let len_off = data.len() - 20;
    let bc_len = u32::from_le_bytes([
        data[len_off],
        data[len_off + 1],
        data[len_off + 2],
        data[len_off + 3],
    ]) as usize;
    let bc_start = len_off.saturating_sub(bc_len);
    let bytecode = &data[bc_start..len_off];
    Some(
        BytecodeRuntime::from_bytes(bytecode)
            .and_then(|mut runtime| runtime.run_entry())
            .map_err(|e| e.to_string()),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains".to_owned());
    ctx.set_fonts(fonts);
}

fn setup_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.panel_fill = egui::Color32::from_rgb(18, 18, 24);
    visuals.window_fill = egui::Color32::from_rgb(18, 18, 24);
    visuals.extreme_bg_color = egui::Color32::from_rgb(10, 12, 18);
    visuals.faint_bg_color = egui::Color32::from_rgb(24, 24, 33);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(28, 28, 38);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 35, 48);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 68);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 60, 80);
    visuals.selection.bg_fill = egui::Color32::from_rgb(80, 60, 140);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 130, 255));
    style.spacing.window_margin = egui::Margin::same(8);
    ctx.set_global_style(style);
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum TargetClass {
    X86, IntelEvolution, Arm, RiscV, ClassicRisc, FpgaSoftCore,
    Academic, EightBitClassic, EightBitMcu, SixteenBitMcu,
    ThirtyTwoBitMcu, Dsp, Hitachi68kConsole, PdpVaxHp,
    MainframeVintage, Vliw, MilAero, SovietExotic, Special,
}

impl TargetClass {
    fn all() -> &'static [TargetClass] {
        &[
            TargetClass::X86, TargetClass::IntelEvolution, TargetClass::Arm,
            TargetClass::RiscV, TargetClass::ClassicRisc, TargetClass::FpgaSoftCore,
            TargetClass::Academic, TargetClass::EightBitClassic, TargetClass::EightBitMcu,
            TargetClass::SixteenBitMcu, TargetClass::ThirtyTwoBitMcu, TargetClass::Dsp,
            TargetClass::Hitachi68kConsole, TargetClass::PdpVaxHp,
            TargetClass::MainframeVintage, TargetClass::Vliw,
            TargetClass::MilAero, TargetClass::SovietExotic, TargetClass::Special,
        ]
    }
    fn label(self) -> &'static str {
        match self {
            TargetClass::X86 => "x86",
            TargetClass::IntelEvolution => "Intel Evolution",
            TargetClass::Arm => "ARM",
            TargetClass::RiscV => "RISC-V",
            TargetClass::ClassicRisc => "Classic RISC",
            TargetClass::FpgaSoftCore => "FPGA Soft Core",
            TargetClass::Academic => "Academic",
            TargetClass::EightBitClassic => "8-bit Classic",
            TargetClass::EightBitMcu => "8-bit MCU",
            TargetClass::SixteenBitMcu => "16-bit MCU",
            TargetClass::ThirtyTwoBitMcu => "32-bit MCU / RISC",
            TargetClass::Dsp => "DSP",
            TargetClass::Hitachi68kConsole => "Hitachi, 68k, Console",
            TargetClass::PdpVaxHp => "PDP, VAX, HP",
            TargetClass::MainframeVintage => "Mainframe & Vintage",
            TargetClass::Vliw => "VLIW",
            TargetClass::MilAero => "Military & Aerospace",
            TargetClass::SovietExotic => "Soviet & Exotic",
            TargetClass::Special => "Special",
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum CompileTarget {
    // Portable / Special
    Portable, Vm, Ternary, Quantum8, Quantum64,
    // x86
    Pe, Bytecode, Bios16, Uefi, X8664, Win64, Win32, Elf64, Elf32, I8086,
    // Intel Evolution
    I4004, I8008, I8080,
    // ARM
    ArmCm, Arm7tdmi, Arm9, Aarch64, Macho, Cheri,
    // RISC-V
    RiscV, RiscV64, RiscVCheri,
    // Classic RISC
    Mips, Ppc, Ppc740, Ppc970, Sparc, Alpha, Parisc,
    // FPGA Soft Core
    OpenRisc, Nios2, Microblaze, Mico32, Picoblaze,
    // Academic
    Mmix, Dlx, Lc3,
    // 8-bit Classic
    C6502, Z80, C6809, M6800, Mos6501,
    // 8-bit MCU
    Pic, Avr, Xc800, Nec78k, R8c,
    // 16-bit MCU
    Msp430, C166, Rl78, H8, M16c, V20,
    // 32-bit MCU / RISC
    Rx, Fr, V810,
    // DSP
    Tms320, Blackfin, Sharc,
    // Hitachi, 68k, Console
    Sh2, Sh4, M68k, HuC6280,
    // PDP, VAX, HP
    Pdp8, Pdp11, Vax, Hp3000,
    // Mainframe & Vintage
    S360, Zarch, Univac, Cdc6600,
    // VLIW
    Vliw, Elbrus, Ia64,
    // Military & Aerospace
    Mil1750a, Jovial,
    // Soviet & Exotic
    Ural, Besm, Mir, Harvard, Mill,
}

impl CompileTarget {
    fn class(self) -> TargetClass {
        match self {
            CompileTarget::Pe | CompileTarget::Bytecode | CompileTarget::Bios16
                | CompileTarget::Uefi | CompileTarget::X8664 | CompileTarget::Win64
                | CompileTarget::Win32 | CompileTarget::Elf64 | CompileTarget::Elf32
                | CompileTarget::I8086 => TargetClass::X86,
            CompileTarget::I4004 | CompileTarget::I8008 | CompileTarget::I8080 => TargetClass::IntelEvolution,
            CompileTarget::ArmCm | CompileTarget::Arm7tdmi | CompileTarget::Arm9
                | CompileTarget::Aarch64 | CompileTarget::Macho | CompileTarget::Cheri => TargetClass::Arm,
            CompileTarget::RiscV | CompileTarget::RiscV64 | CompileTarget::RiscVCheri => TargetClass::RiscV,
            CompileTarget::Mips | CompileTarget::Ppc | CompileTarget::Ppc740
                | CompileTarget::Ppc970 | CompileTarget::Sparc | CompileTarget::Alpha
                | CompileTarget::Parisc => TargetClass::ClassicRisc,
            CompileTarget::OpenRisc | CompileTarget::Nios2 | CompileTarget::Microblaze
                | CompileTarget::Mico32 | CompileTarget::Picoblaze => TargetClass::FpgaSoftCore,
            CompileTarget::Mmix | CompileTarget::Dlx | CompileTarget::Lc3 => TargetClass::Academic,
            CompileTarget::C6502 | CompileTarget::Z80 | CompileTarget::C6809
                | CompileTarget::M6800 | CompileTarget::Mos6501 => TargetClass::EightBitClassic,
            CompileTarget::Pic | CompileTarget::Avr | CompileTarget::Xc800
                | CompileTarget::Nec78k | CompileTarget::R8c => TargetClass::EightBitMcu,
            CompileTarget::Msp430 | CompileTarget::C166 | CompileTarget::Rl78
                | CompileTarget::H8 | CompileTarget::M16c | CompileTarget::V20 => TargetClass::SixteenBitMcu,
            CompileTarget::Rx | CompileTarget::Fr | CompileTarget::V810 => TargetClass::ThirtyTwoBitMcu,
            CompileTarget::Tms320 | CompileTarget::Blackfin | CompileTarget::Sharc => TargetClass::Dsp,
            CompileTarget::Sh2 | CompileTarget::Sh4 | CompileTarget::M68k
                | CompileTarget::HuC6280 => TargetClass::Hitachi68kConsole,
            CompileTarget::Pdp8 | CompileTarget::Pdp11 | CompileTarget::Vax
                | CompileTarget::Hp3000 => TargetClass::PdpVaxHp,
            CompileTarget::S360 | CompileTarget::Zarch | CompileTarget::Univac
                | CompileTarget::Cdc6600 => TargetClass::MainframeVintage,
            CompileTarget::Vliw | CompileTarget::Elbrus | CompileTarget::Ia64 => TargetClass::Vliw,
            CompileTarget::Mil1750a | CompileTarget::Jovial => TargetClass::MilAero,
            CompileTarget::Ural | CompileTarget::Besm | CompileTarget::Mir
                | CompileTarget::Harvard | CompileTarget::Mill => TargetClass::SovietExotic,
            CompileTarget::Portable | CompileTarget::Vm | CompileTarget::Ternary
                | CompileTarget::Quantum8 | CompileTarget::Quantum64 => TargetClass::Special,
        }
    }

    fn targets_for_class(class: TargetClass) -> &'static [CompileTarget] {
        match class {
            TargetClass::X86 => &[CompileTarget::Pe, CompileTarget::Bytecode, CompileTarget::Bios16,
                CompileTarget::Uefi, CompileTarget::X8664, CompileTarget::Win64,
                CompileTarget::Win32, CompileTarget::Elf64, CompileTarget::Elf32, CompileTarget::I8086],
            TargetClass::IntelEvolution => &[CompileTarget::I4004, CompileTarget::I8008, CompileTarget::I8080],
            TargetClass::Arm => &[CompileTarget::ArmCm, CompileTarget::Arm7tdmi, CompileTarget::Arm9,
                CompileTarget::Aarch64, CompileTarget::Macho, CompileTarget::Cheri],
            TargetClass::RiscV => &[CompileTarget::RiscV, CompileTarget::RiscV64, CompileTarget::RiscVCheri],
            TargetClass::ClassicRisc => &[CompileTarget::Mips, CompileTarget::Ppc, CompileTarget::Ppc740,
                CompileTarget::Ppc970, CompileTarget::Sparc, CompileTarget::Alpha, CompileTarget::Parisc],
            TargetClass::FpgaSoftCore => &[CompileTarget::OpenRisc, CompileTarget::Nios2, CompileTarget::Microblaze,
                CompileTarget::Mico32, CompileTarget::Picoblaze],
            TargetClass::Academic => &[CompileTarget::Mmix, CompileTarget::Dlx, CompileTarget::Lc3],
            TargetClass::EightBitClassic => &[CompileTarget::C6502, CompileTarget::Z80, CompileTarget::C6809,
                CompileTarget::M6800, CompileTarget::Mos6501],
            TargetClass::EightBitMcu => &[CompileTarget::Pic, CompileTarget::Avr, CompileTarget::Xc800,
                CompileTarget::Nec78k, CompileTarget::R8c],
            TargetClass::SixteenBitMcu => &[CompileTarget::Msp430, CompileTarget::C166, CompileTarget::Rl78,
                CompileTarget::H8, CompileTarget::M16c, CompileTarget::V20],
            TargetClass::ThirtyTwoBitMcu => &[CompileTarget::Rx, CompileTarget::Fr, CompileTarget::V810],
            TargetClass::Dsp => &[CompileTarget::Tms320, CompileTarget::Blackfin, CompileTarget::Sharc],
            TargetClass::Hitachi68kConsole => &[CompileTarget::Sh2, CompileTarget::Sh4, CompileTarget::M68k,
                CompileTarget::HuC6280],
            TargetClass::PdpVaxHp => &[CompileTarget::Pdp8, CompileTarget::Pdp11, CompileTarget::Vax,
                CompileTarget::Hp3000],
            TargetClass::MainframeVintage => &[CompileTarget::S360, CompileTarget::Zarch, CompileTarget::Univac,
                CompileTarget::Cdc6600],
            TargetClass::Vliw => &[CompileTarget::Vliw, CompileTarget::Elbrus, CompileTarget::Ia64],
            TargetClass::MilAero => &[CompileTarget::Mil1750a, CompileTarget::Jovial],
            TargetClass::SovietExotic => &[CompileTarget::Ural, CompileTarget::Besm, CompileTarget::Mir,
                CompileTarget::Harvard, CompileTarget::Mill],
            TargetClass::Special => &[CompileTarget::Portable, CompileTarget::Vm, CompileTarget::Ternary,
                CompileTarget::Quantum8, CompileTarget::Quantum64],
        }
    }

    fn label(self) -> &'static str {
        match self {
            CompileTarget::Pe => "PE (.exe)",
            CompileTarget::Bytecode => "Bytecode (.dcb)",
            CompileTarget::Bios16 => "BIOS16 (.img)",
            CompileTarget::Uefi => "UEFI (.efi)",
            CompileTarget::X8664 => "x86-64 (.bin)",
            CompileTarget::Win64 => "Win64 (.exe)",
            CompileTarget::Win32 => "Win32 (.exe)",
            CompileTarget::Elf64 => "ELF64",
            CompileTarget::Elf32 => "ELF32",
            CompileTarget::I8086 => "i8086 (.bin)",
            CompileTarget::I4004 => "i4004",
            CompileTarget::I8008 => "i8008",
            CompileTarget::I8080 => "i8080",
            CompileTarget::ArmCm => "ARM Cortex-M (.bin)",
            CompileTarget::Arm7tdmi => "ARM7TDMI",
            CompileTarget::Arm9 => "ARM9",
            CompileTarget::Aarch64 => "AArch64 (.bin)",
            CompileTarget::Macho => "Mach-O (macOS)",
            CompileTarget::Cheri => "CHERI (.bin)",
            CompileTarget::RiscV => "RISC-V (.bin)",
            CompileTarget::RiscV64 => "RISC-V 64 (.bin)",
            CompileTarget::RiscVCheri => "RISC-V CHERI (.bin)",
            CompileTarget::Mips => "MIPS",
            CompileTarget::Ppc => "PowerPC",
            CompileTarget::Ppc740 => "PowerPC 740",
            CompileTarget::Ppc970 => "PowerPC 970",
            CompileTarget::Sparc => "SPARC",
            CompileTarget::Alpha => "Alpha",
            CompileTarget::Parisc => "PA-RISC",
            CompileTarget::OpenRisc => "OpenRISC",
            CompileTarget::Nios2 => "Nios II",
            CompileTarget::Microblaze => "MicroBlaze",
            CompileTarget::Mico32 => "Mico32",
            CompileTarget::Picoblaze => "PicoBlaze",
            CompileTarget::Mmix => "MMIX",
            CompileTarget::Dlx => "DLX",
            CompileTarget::Lc3 => "LC-3",
            CompileTarget::C6502 => "6502",
            CompileTarget::Z80 => "Z80",
            CompileTarget::C6809 => "6809",
            CompileTarget::M6800 => "M6800",
            CompileTarget::Mos6501 => "MOS 6501",
            CompileTarget::Pic => "PIC",
            CompileTarget::Avr => "AVR",
            CompileTarget::Xc800 => "XC800",
            CompileTarget::Nec78k => "NEC 78K",
            CompileTarget::R8c => "R8C",
            CompileTarget::Msp430 => "MSP430",
            CompileTarget::C166 => "C166",
            CompileTarget::Rl78 => "RL78",
            CompileTarget::H8 => "H8",
            CompileTarget::M16c => "M16C",
            CompileTarget::V20 => "NEC V20",
            CompileTarget::Rx => "RX",
            CompileTarget::Fr => "FR",
            CompileTarget::V810 => "V810",
            CompileTarget::Tms320 => "TMS320",
            CompileTarget::Blackfin => "Blackfin",
            CompileTarget::Sharc => "SHARC",
            CompileTarget::Sh2 => "SH-2",
            CompileTarget::Sh4 => "SH-4",
            CompileTarget::M68k => "M68k",
            CompileTarget::HuC6280 => "HuC6280",
            CompileTarget::Pdp8 => "PDP-8",
            CompileTarget::Pdp11 => "PDP-11",
            CompileTarget::Vax => "VAX",
            CompileTarget::Hp3000 => "HP 3000",
            CompileTarget::S360 => "System/360",
            CompileTarget::Zarch => "z/Architecture",
            CompileTarget::Univac => "UNIVAC",
            CompileTarget::Cdc6600 => "CDC 6600",
            CompileTarget::Vliw => "VLIW",
            CompileTarget::Elbrus => "Elbrus",
            CompileTarget::Ia64 => "IA-64",
            CompileTarget::Mil1750a => "MIL-STD-1750A",
            CompileTarget::Jovial => "JOVIAL (VM)",
            CompileTarget::Ural => "Ural",
            CompileTarget::Besm => "BESM",
            CompileTarget::Mir => "Mir",
            CompileTarget::Harvard => "Harvard",
            CompileTarget::Mill => "Mill",
            CompileTarget::Portable => "Portable Bytecode",
            CompileTarget::Vm => "Stack VM (.vbc)",
            CompileTarget::Ternary => "Ternary",
            CompileTarget::Quantum8 => "Quantum 8",
            CompileTarget::Quantum64 => "Quantum 64",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            CompileTarget::Pe => "exe",
            CompileTarget::Bytecode => "dcb",
            CompileTarget::Bios16 => "img",
            CompileTarget::Uefi => "efi",
            CompileTarget::X8664 | CompileTarget::Win64 | CompileTarget::Win32 => "exe",
            CompileTarget::Elf64 | CompileTarget::Elf32 => "elf",
            CompileTarget::Macho => "macho",
            CompileTarget::Vm => "vbc",
            CompileTarget::Quantum8 | CompileTarget::Quantum64 => "qbin",
            CompileTarget::Portable | CompileTarget::Ternary => "dcb",
            _ => "bin",
        }
    }
}

#[derive(Clone)]
struct BuildRecord {
    when: Instant,
    mode: String,
    target: CompileTarget,
    output: PathBuf,
    success: bool,
    message: String,
}

#[derive(Clone)]
struct WorkspaceSearchResult {
    path: PathBuf,
    line: usize,
    text: String,
}

#[derive(Clone)]
struct OutlineSymbol {
    kind: String,
    name: String,
    line: usize,
}

#[derive(Clone)]
struct OpenTab {
    id: u64,
    title: String,
    file_path: Option<PathBuf>,
    code: String,
    dirty: bool,
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,
    max_undo: usize,
}

impl OpenTab {
    fn snapshot(&mut self) {
        if self.max_undo == 0 { return; }
        self.undo_stack.push(self.code.clone());
        if self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.code.clone());
            self.code = prev;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.code.clone());
            self.code = next;
            self.dirty = true;
        }
    }
}

struct DecretumIde {
    code: String,
    file_path: Option<PathBuf>,
    workspace_root: PathBuf,
    workspace_files: Vec<PathBuf>,
    workspace_filter: String,
    workspace_content_query: String,
    workspace_content_results: Vec<WorkspaceSearchResult>,
    status: String,
    status_color: egui::Color32,
    output_log: String,
    out_file: String,
    target: CompileTarget,
    selected_target_class: TargetClass,
    last_artifact: Option<PathBuf>,
    show_output_panel: bool,
    show_help_panel: bool,
    show_workspace_panel: bool,
    show_inspector_panel: bool,
    show_outline_panel: bool,
    show_find: bool,
    dirty: bool,
    find_text: String,
    replace_text: String,
    case_sensitive_search: bool,
    last_find_index: Option<usize>,
    last_error: String,
    undo_debounce: Instant,
    autosave_enabled: bool,
    autosave_interval_secs: u64,
    last_autosave_at: Instant,
    build_history: Vec<BuildRecord>,
    external_command: String,
    external_args: String,
    show_reference_panel: bool,
    selected_doc_index: usize,
    markdown_cache: egui_commonmark::CommonMarkCache,
    tabs: Vec<OpenTab>,
    active_tab: usize,
    next_tab_id: u64,
    show_command_palette: bool,
    command_palette_query: String,
    show_quick_open: bool,
    quick_open_query: String,
    cursor_row_col: Option<(usize, usize)>,
    outline_symbols: Vec<OutlineSymbol>,
    error_lines: Vec<usize>,
    show_compile_all: bool,
    show_progress: bool,
    progress_msg: String,
    workspace_build_output: String,
    run_output: String,
    show_run_output: bool,
    run_output_arc: Option<Arc<Mutex<String>>>,
    enter_pressed_flag: bool,
    previous_code: String,
}

impl Default for DecretumIde {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            code: r#"target portable
entry main

event main:
    print "Hello from Decretum!\n"
    exit 0
"#
            .to_string(),
            file_path: None,
            workspace_root: cwd,
            workspace_files: Vec::new(),
            workspace_filter: String::new(),
            workspace_content_query: String::new(),
            workspace_content_results: Vec::new(),
            status: "Ready".to_string(),
            status_color: egui::Color32::from_rgb(120, 180, 120),
            output_log: String::new(),
            out_file: "build/ide_output.exe".to_string(),
            target: CompileTarget::Pe,
            selected_target_class: TargetClass::X86,
            last_artifact: None,
            show_output_panel: true,
            show_help_panel: false,
            show_workspace_panel: true,
            show_inspector_panel: true,
            dirty: false,
            find_text: String::new(),
            replace_text: String::new(),
            case_sensitive_search: false,
            last_find_index: None,
            last_error: String::new(),
            undo_debounce: Instant::now(),
            autosave_enabled: true,
            autosave_interval_secs: 20,
            last_autosave_at: Instant::now(),
            build_history: Vec::new(),
            external_command: "cargo".to_string(),
            external_args: "run --bin decretum -- validate examples/compute_pe.dcrt".to_string(),
            show_reference_panel: true,
            show_outline_panel: true,
            show_find: false,
            selected_doc_index: 0,
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 1,
            show_command_palette: false,
            command_palette_query: String::new(),
            show_quick_open: false,
            quick_open_query: String::new(),
            cursor_row_col: None,
            outline_symbols: Vec::new(),
            error_lines: Vec::new(),
            show_compile_all: false,
            show_progress: false,
            progress_msg: String::new(),
            workspace_build_output: String::new(),
            run_output: String::new(),
            show_run_output: false,
            run_output_arc: None,
            enter_pressed_flag: false,
            previous_code: r#"target portable
entry main

event main:
    print "Hello from Decretum!\n"
    exit 0
"#
            .to_string(),
        };
        app.tabs.push(OpenTab {
            id: 0,
            title: "untitled.dcrt".to_string(),
            file_path: None,
            code: app.code.clone(),
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 50,
        });
        let _ = app.refresh_workspace();
        app
    }
}

impl eframe::App for DecretumIde {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Poll threaded output capture
        if let Some(arc) = &self.run_output_arc {
            if let Ok(mut buf) = arc.lock() {
                if !buf.is_empty() {
                    self.run_output.push_str(&buf);
                    buf.clear();
                }
            }
        }
        self.handle_shortcuts(&ctx);
        self.maybe_autosave();
        self.render_top_bar(ui);
        self.render_tab_strip(ui);
        if self.show_workspace_panel {
            self.render_explorer_panel(ui);
        }
        if self.show_outline_panel {
            self.render_outline_panel(ui);
        }
        if self.show_inspector_panel {
            self.render_inspector_panel(ui);
        }
        if self.show_reference_panel {
            self.render_reference_panel(&ctx);
        }
        if self.show_compile_all {
            self.render_compile_all_dialog(&ctx);
        }
        if self.show_command_palette {
            self.render_command_palette(&ctx);
        }
        if self.show_quick_open {
            self.render_quick_open(&ctx);
        }
        if self.show_find {
            self.render_find_overlay(&ctx);
        }
        self.render_status_bar(&ctx);
        self.render_bottom_panel(ui);
        self.render_editor(ui);
    }
}

impl DecretumIde {
    fn sync_buffer_to_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.code = self.code.clone();
            tab.file_path = self.file_path.clone();
            tab.dirty = self.dirty;
            tab.title = self
                .file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(ToString::to_string)
                .unwrap_or_else(|| "untitled.dcrt".to_string());
        }
    }

    fn load_active_tab_to_buffer(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab).cloned() {
            self.code = tab.code;
            self.file_path = tab.file_path;
            self.dirty = tab.dirty;
        }
        self.parse_outline();
        self.parse_errors();
    }

    fn open_tab_with_content(&mut self, file_path: Option<PathBuf>, title: String, code: String) {
        self.sync_buffer_to_active_tab();
        let id = self.next_tab_id;
        self.next_tab_id = self.next_tab_id.saturating_add(1);
        self.tabs.push(OpenTab {
            id,
            title,
            file_path: file_path.clone(),
            code,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 50,
        });
        self.active_tab = self.tabs.len().saturating_sub(1);
        self.load_active_tab_to_buffer();
    }

    fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        self.sync_buffer_to_active_tab();
        self.tabs.remove(index);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.load_active_tab_to_buffer();
    }

    fn switch_to_tab(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }
        self.sync_buffer_to_active_tab();
        self.active_tab = index;
        self.load_active_tab_to_buffer();
    }

    fn maybe_autosave(&mut self) {
        if !self.autosave_enabled || !self.dirty {
            return;
        }
        if self.last_autosave_at.elapsed() < Duration::from_secs(self.autosave_interval_secs) {
            return;
        }
        if let Some(path) = self.file_path.clone()
            && fs::write(&path, &self.code).is_ok()
        {
            self.dirty = false;
            self.last_autosave_at = Instant::now();
            self.sync_buffer_to_active_tab();
            self.push_log("autosave", &format!("saved {}", path.display()));
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let save = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S));
        let open = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O));
        let compile = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::B));
        let validate = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K));
        let find_next = ctx.input(|i| i.key_pressed(egui::Key::F3));
        let toggle_find = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::F));
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let undo = ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z));
        let redo = ctx.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z));
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let duplicate = ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::D));
        let toggle_comment = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Slash));
        let delete_line = ctx.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::K));

        let cmd_palette =
            ctx.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::P));
        let quick_open =
            ctx.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::P));

        if enter {
            self.enter_pressed_flag = true;
        }

        if save {
            self.save_current_file();
        }
        if open {
            self.open_file_picker();
        }
        if compile {
            self.compile_code();
        }
        if validate {
            self.validate_code();
        }
        if find_next {
            let _ = self.find_next();
        }
        if toggle_find {
            self.show_find = !self.show_find;
            if !self.show_find {
                self.find_text.clear();
                self.replace_text.clear();
            }
        }
        if escape && self.show_find {
            self.show_find = false;
            self.find_text.clear();
            self.replace_text.clear();
        }
        if undo {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.undo();
                self.code = tab.code.clone();
                self.dirty = tab.dirty;
            }
        }
        if redo {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.redo();
                self.code = tab.code.clone();
                self.dirty = tab.dirty;
            }
        }
        if duplicate {
            self.duplicate_current_line();
        }
        if toggle_comment {
            self.toggle_comment_current();
        }
        if delete_line {
            self.delete_current_line();
        }
        if cmd_palette {
            self.show_command_palette = !self.show_command_palette;
            if self.show_command_palette {
                self.show_quick_open = false;
            }
        }
        if quick_open {
            self.show_quick_open = !self.show_quick_open;
            if self.show_quick_open {
                self.show_command_palette = false;
            }
        }
    }

    // Duplicates the current line (or selection) below.
    fn duplicate_current_line(&mut self) {
        let code = &self.code;
        let line_count = code.lines().count().max(1);
        let cursor_line = self.cursor_row_col.map(|(r, _)| r).unwrap_or(1);
        let line_idx = cursor_line.saturating_sub(1).min(line_count.saturating_sub(1));
        let mut start = 0usize;
        for _ in 0..line_idx {
            if let Some(pos) = code[start..].find('\n') {
                start = start + pos + 1;
            }
        }
        let end = code[start..].find('\n').map(|p| start + p + 1).unwrap_or(code.len());
        let line = &code[start..end];
        let insert_pos = end;
        let mut new_code = code[..insert_pos].to_string();
        new_code.push_str(line);
        new_code.push_str(&code[insert_pos..]);
        self.code = new_code;
        self.dirty = true;
        self.sync_buffer_to_active_tab();
        self.parse_outline();
        self.parse_errors();
    }

    // Toggle `;` comment on the current line (Decretum uses semicolon like asm iydk)
    fn toggle_comment_current(&mut self) {
        let code = &self.code;
        let line_count = code.lines().count().max(1);
        let cursor_line = self.cursor_row_col.map(|(r, _)| r).unwrap_or(1);
        let line_idx = cursor_line.saturating_sub(1).min(line_count.saturating_sub(1));
        let mut start = 0usize;
        for _ in 0..line_idx {
            if let Some(pos) = code[start..].find('\n') {
                start = start + pos + 1;
            }
        }
        let end = code[start..].find('\n').map(|p| start + p + 1).unwrap_or(code.len());
        let line = &code[start..end];
        let trimmed = line.trim_start();
        let new_line = if trimmed.starts_with(";") {
            let after_ws = line.len() - trimmed.len();
            let after_comment = ";".len();
            let comment_content = &trimmed[after_comment..];
            let rest = comment_content.strip_prefix(' ').unwrap_or(comment_content);
            let indent = &line[..after_ws];
            format!("{}{}", indent, rest)
        } else {
            format!("{}; {}", &line[..line.len() - trimmed.len()], trimmed)
        };
        let mut new_code = code[..start].to_string();
        new_code.push_str(&new_line);
        new_code.push_str(&code[end..]);
        self.code = new_code;
        self.dirty = true;
        self.sync_buffer_to_active_tab();
        self.parse_outline();
        self.parse_errors();
    }

    fn delete_current_line(&mut self) {
        let code = &self.code;
        let line_count = code.lines().count().max(1);
        let cursor_line = self.cursor_row_col.map(|(r, _)| r).unwrap_or(1);
        let line_idx = cursor_line.saturating_sub(1).min(line_count.saturating_sub(1));
        let mut start = 0usize;
        for _ in 0..line_idx {
            if let Some(pos) = code[start..].find('\n') {
                start = start + pos + 1;
            }
        }
        let end = code[start..].find('\n')
            .map(|p| start + p + 1)    // include the newline
            .unwrap_or(code.len());
        let mut new_code = code[..start].to_string();
        new_code.push_str(&code[end..]);
        self.code = new_code;
        self.dirty = true;
        self.sync_buffer_to_active_tab();
        self.parse_outline();
        self.parse_errors();
    }

    fn render_command_palette(&mut self, ctx: &egui::Context) {
        egui::Window::new("Command Palette")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Type a command...")
                        .desired_width(f32::INFINITY),
                );
                if edit.clicked_elsewhere() {
                    self.show_command_palette = false;
                }
                edit.request_focus();

                let commands = [
                    ("File: New", "new_file"),
                    ("File: Open", "open_file_picker"),
                    ("File: Save", "save_current_file"),
                    ("File: Save As", "save_as"),
                    ("File: Save All", "save_all_tabs"),
                    ("Workspace: Refresh", "refresh_workspace"),
                    ("Workspace: Open Folder", "open_workspace_folder"),
                    ("Workspace: Close All Tabs", "close_all_tabs"),
                    ("Build: Compile", "compile_code"),
                    ("Build: Run Last Artifact", "run_last_artifact"),
                    ("Build: Validate Code", "validate_code"),
                    ("View: Toggle Explorer", "toggle_explorer"),
                    ("View: Toggle Output", "toggle_output"),
                    ("View: Toggle Inspector", "toggle_inspector"),
                    ("View: Toggle Docs", "toggle_docs"),
                    ("Help: Export Diagnostics", "export_diagnostics_report"),
                ];

                let query = self.command_palette_query.to_lowercase();
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (name, action) in commands {
                            if query.is_empty() || name.to_lowercase().contains(&query) {
                                if ui.selectable_label(false, name).clicked() {
                                    self.show_command_palette = false;
                                    self.command_palette_query.clear();
                                    match action {
                                        "new_file" => self.new_file(),
                                        "open_file_picker" => self.open_file_picker(),
                                        "save_current_file" => self.save_current_file(),
                                        "save_as" => self.save_as(),
                                        "save_all_tabs" => self.save_all_tabs(),
                                        "refresh_workspace" => {
                                            let _ = self.refresh_workspace();
                                        }
                                        "open_workspace_folder" => self.open_workspace_folder(),
                                        "close_all_tabs" => self.close_all_tabs(),
                                        "compile_code" => self.compile_code(),
                                        "run_last_artifact" => self.run_last_artifact(),
                                        "validate_code" => self.validate_code(),
                                        "toggle_explorer" => {
                                            self.show_workspace_panel = !self.show_workspace_panel
                                        }
                                        "toggle_output" => {
                                            self.show_output_panel = !self.show_output_panel
                                        }
                                        "toggle_inspector" => {
                                            self.show_inspector_panel = !self.show_inspector_panel
                                        }
                                        "toggle_docs" => {
                                            self.show_reference_panel = !self.show_reference_panel
                                        }
                                        "export_diagnostics_report" => {
                                            self.export_diagnostics_report()
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    });
            });
    }

    fn render_find_overlay(&mut self, ctx: &egui::Context) {
        egui::Window::new("Find / Replace")
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    if ui.button("X").clicked() {
                        self.show_find = false;
                        self.find_text.clear();
                        self.replace_text.clear();
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.find_text)
                        .desired_width(f32::INFINITY)
                        .hint_text("text to find")
                );
                ui.horizontal(|ui| {
                    ui.label("Replace:");
                    ui.checkbox(&mut self.case_sensitive_search, "Aa");
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.replace_text)
                        .desired_width(f32::INFINITY)
                        .hint_text("replace with")
                );
                ui.horizontal(|ui| {
                    if ui.button("Find Next (F3)").clicked() {
                        let _ = self.find_next();
                    }
                    if ui.button("Replace").clicked() {
                        let _ = self.replace_next();
                    }
                    if ui.button("Replace All").clicked() {
                        let count = self.replace_all();
                        self.push_log("replace", &format!("replaced {count} occurrence(s)"));
                    }
                });
            });
    }

    fn render_quick_open(&mut self, ctx: &egui::Context) {
        egui::Window::new("Go to File")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.quick_open_query)
                        .hint_text("Search files by name...")
                        .desired_width(f32::INFINITY),
                );
                if edit.clicked_elsewhere() {
                    self.show_quick_open = false;
                }
                edit.request_focus();

                let query = self.quick_open_query.to_lowercase();
                let mut open_path: Option<PathBuf> = None;

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for path in &self.workspace_files {
                            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if query.is_empty() || name.to_lowercase().contains(&query) {
                                let rel = path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(path)
                                    .display()
                                    .to_string();
                                if ui.selectable_label(false, rel).clicked() {
                                    open_path = Some(path.clone());
                                }
                            }
                        }
                    });

                if let Some(path) = open_path {
                    self.open_workspace_file(&path);
                    self.show_quick_open = false;
                    self.quick_open_query.clear();
                }
            });
    }

    fn close_all_tabs(&mut self) {
        self.tabs.clear();
        self.next_tab_id = 1;
        self.new_file();
    }
    fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .default_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Workspace: {}", self.workspace_root.display()));
                    ui.separator();

                    ui.label("Class:");
                    egui::ComboBox::from_id_salt("class_sel")
                        .selected_text(self.selected_target_class.label())
                        .show_ui(ui, |ui| {
                            for &c in TargetClass::all() {
                                ui.selectable_value(&mut self.selected_target_class, c, c.label());
                            }
                        });
                    ui.label("Target:");
                    egui::ComboBox::from_id_salt("target_sel")
                        .selected_text(self.target.label())
                        .show_ui(ui, |ui| {
                            for &t in CompileTarget::targets_for_class(self.selected_target_class) {
                                ui.selectable_value(&mut self.target, t, t.label());
                            }
                        });
                    ui.separator();
                    if let Some(path) = &self.file_path {
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        ui.label(format!("Lang: {}", ext));
                        ui.separator();
                    }
                    if self.dirty {
                        ui.colored_label(egui::Color32::from_rgb(250, 150, 100), "● Modified");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "● Saved");
                    }
                    ui.separator();

                    if ui.button("Outline").clicked() {
                        self.show_outline_panel = !self.show_outline_panel;
                    }
                    ui.separator();
                    if let Some((row, col)) = self.cursor_row_col {
                        ui.label(format!("Ln {}, Col {}", row, col));
                    }
                });
            });
    }

    fn render_reference_panel(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        let w = screen_rect.width() * 0.35;
        let h = screen_rect.height() * 0.35;

        egui::Window::new("Decretum References")
            .default_size([w, h])
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(120, 120, 120), "🌐 /decretum/docs/");
                });
                ui.separator();

                ui.horizontal_top(|ui| {
                    let sidebar_width = (ui.available_width() * 0.25).clamp(120.0, 180.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(255, 255, 255))
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgb(208, 215, 222),
                        ))
                        .inner_margin(egui::Margin::symmetric(8, 8))
                        .show(ui, |ui| {
                            ui.set_width(sidebar_width);
                            ui.vertical(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(31, 35, 40), "Docs Tree");
                                ui.separator();
                                egui::ScrollArea::both().show(ui, |ui| {
                                    for i in 0..docs_data::DOCS.len() {
                                        let selected = self.selected_doc_index == i;
                                        if ui
                                            .selectable_label(selected, docs_data::DOCS[i].0)
                                            .clicked()
                                        {
                                            self.selected_doc_index = i;
                                        }
                                    }
                                });
                            });
                        });

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(246, 248, 250))
                        .inner_margin(egui::Margin::symmetric(10, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::both().show(ui, |ui| {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(255, 255, 255))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(208, 215, 222),
                                    ))
                                    .rounding(6.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        let raw_html = docs_data::DOCS[self.selected_doc_index].1;
                                        let markdown = html2md::parse_html(raw_html);
                                        egui_commonmark::CommonMarkViewer::new().show(
                                            ui,
                                            &mut self.markdown_cache,
                                            &markdown,
                                        );
                                    });
                            });
                        });
                });
            });
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("menu_top").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_file();
                        ui.close();
                    }
                    if ui.button("Open...  (Ctrl+O)").clicked() {
                        self.open_file_picker();
                        ui.close();
                    }
                    if ui.button("Save  (Ctrl+S)").clicked() {
                        self.save_current_file();
                        ui.close();
                    }
                    if ui.button("Save As...").clicked() {
                        self.save_as();
                        ui.close();
                    }
                    if ui.button("Refresh Workspace").clicked() {
                        let _ = self.refresh_workspace();
                        ui.close();
                    }
                    if ui.button("Open Workspace Folder...").clicked() {
                        self.open_workspace_folder();
                        ui.close();
                    }
                    if ui.button("Export Diagnostics Report").clicked() {
                        self.export_diagnostics_report();
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        ui.text_edit_singleline(&mut self.find_text);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Replace:");
                        ui.text_edit_singleline(&mut self.replace_text);
                    });
                    ui.checkbox(&mut self.case_sensitive_search, "Case Sensitive");
                    if ui.button("Find Next (F3)").clicked() {
                        let _ = self.find_next();
                    }
                    if ui.button("Replace Next").clicked() {
                        let _ = self.replace_next();
                    }
                    if ui.button("Replace All").clicked() {
                        let count = self.replace_all();
                        self.push_log("replace", &format!("replaced {count} occurrence(s)"));
                    }
                });
                ui.menu_button("Build", |ui| {
                    if ui.button("Validate  (Ctrl+K)").clicked() {
                        self.validate_code();
                        ui.close();
                    }
                    if ui.button("Compile  (Ctrl+B)").clicked() {
                        self.compile_code();
                        ui.close();
                    }
                    if ui.button("Run Last Artifact").clicked() {
                        self.run_last_artifact();
                        ui.close();
                    }
                    if ui.button("Compile Workspace").clicked() {
                        self.show_compile_all = true;
                        self.compile_all_workspace();
                        ui.close();
                    }
                    if ui.button("Compile Workspace Project").clicked() {
                        self.compile_workspace_project();
                        ui.close();
                    }
                    if ui.button("Run External Command").clicked() {
                        self.run_external_command();
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_workspace_panel, "Workspace");
                    ui.checkbox(&mut self.show_outline_panel, "Outline");
                    ui.checkbox(&mut self.show_inspector_panel, "Inspector");
                    ui.checkbox(&mut self.show_output_panel, "Output");
                    ui.checkbox(&mut self.show_help_panel, "Help");
                    ui.checkbox(&mut self.show_reference_panel, "Reference");
                });
                ui.menu_button("Insert", |ui| {
                    if ui.button("Portable Hello World").clicked() {
                        self.insert_snippet(Snippet::PortableHello);
                        ui.close();
                    }
                    if ui.button("Portable Loop").clicked() {
                        self.insert_snippet(Snippet::PortLoop);
                        ui.close();
                    }
                    if ui.button("RISC-V CHERI").clicked() {
                        self.insert_snippet(Snippet::RisCvCheri);
                        ui.close();
                    }
                    if ui.button("ARM Cortex-M").clicked() {
                        self.insert_snippet(Snippet::ArmCm);
                        ui.close();
                    }
                    if ui.button("8-bit (6502)").clicked() {
                        self.insert_snippet(Snippet::EightBit);
                        ui.close();
                    }
                    if ui.button("BIOS16 Skeleton").clicked() {
                        self.insert_snippet(Snippet::BiosSkeleton);
                        ui.close();
                    }
                    if ui.button("Procedure Template").clicked() {
                        self.insert_snippet(Snippet::ProcTemplate);
                        ui.close();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(160, 130, 255),
                    egui::RichText::new("DECRETUM IDE").strong(),
                );
                if self.dirty {
                    ui.colored_label(egui::Color32::from_rgb(255, 190, 90), "unsaved");
                }
                ui.separator();
                ui.label("Class:");
                egui::ComboBox::from_id_salt("class_top")
                    .selected_text(self.selected_target_class.label())
                    .show_ui(ui, |ui| {
                        for &c in TargetClass::all() {
                            ui.selectable_value(&mut self.selected_target_class, c, c.label());
                        }
                    });
                ui.label("Target:");
                egui::ComboBox::from_id_salt("target_top")
                    .selected_text(self.target.label())
                    .show_ui(ui, |ui| {
                        for &t in CompileTarget::targets_for_class(self.selected_target_class) {
                            ui.selectable_value(&mut self.target, t, t.label());
                        }
                    });
                ui.checkbox(&mut self.autosave_enabled, "Autosave");
                ui.add(
                    egui::DragValue::new(&mut self.autosave_interval_secs)
                        .range(5..=600)
                        .speed(1)
                        .suffix("s"),
                );
                ui.separator();
                if ui.button("Validate").clicked() {
                    self.validate_code();
                }
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Compile").strong())
                            .fill(egui::Color32::from_rgb(80, 60, 160)),
                    )
                    .clicked()
                {
                    self.compile_code();
                }
                if ui.button("Run").clicked() {
                    self.run_last_artifact();
                }
                if ui.button("Compile Project").clicked() {
                    self.compile_workspace_project();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Output:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.out_file)
                        .desired_width(f32::INFINITY)
                        .hint_text("build/output.exe"),
                );
            });
        });
    }

    fn render_tab_strip(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("tab_strip").show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut to_switch = None;
                let mut to_close = None;
                for idx in 0..self.tabs.len() {
                    let tab = &self.tabs[idx];
                    let mut label = tab.title.clone();
                    if tab.dirty {
                        label.push('*');
                    }
                    if ui.selectable_label(idx == self.active_tab, label).clicked() {
                        to_switch = Some(idx);
                    }
                    if ui.small_button("x").clicked() {
                        to_close = Some(idx);
                    }
                    ui.separator();
                }
                if ui.button("+").clicked() {
                    self.open_tab_with_content(
                        None,
                        format!("untitled{}.dcrt", self.next_tab_id),
                        "target portable\nentry main\n\nevent main:\n    exit 0\n".to_string(),
                    );
                }
                if ui.button("Duplicate").clicked() {
                    self.sync_buffer_to_active_tab();
                    if let Some(tab) = self.tabs.get(self.active_tab).cloned() {
                        self.open_tab_with_content(
                            None,
                            format!("{} copy", tab.title),
                            tab.code.clone(),
                        );
                    }
                }
                if ui.button("Save All").clicked() {
                    self.save_all_tabs();
                }
                if let Some(idx) = to_switch {
                    self.switch_to_tab(idx);
                }
                if let Some(idx) = to_close {
                    self.close_tab(idx);
                }
            });
        });
    }

    fn render_explorer_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("explorer_left")
            .resizable(true)
            .default_width(300.0)
            .min_width(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Explorer");
                ui.label(self.workspace_root.display().to_string());
                ui.horizontal(|ui| {
                    if ui.button("Refresh").clicked() {
                        let _ = self.refresh_workspace();
                    }
                    if ui.button("Open...").clicked() {
                        self.open_file_picker();
                    }
                    if ui.button("Set Root").clicked() {
                        self.open_workspace_folder();
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 180.0)
                    .show(ui, |ui| {
                        let root = self.workspace_root.clone();
                        self.render_directory_node(ui, &root);
                    });

                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.workspace_filter)
                        .hint_text("Filter files..."),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.workspace_content_query)
                            .hint_text("Search in contents..."),
                    );
                    if ui.button("Search").clicked() {
                        self.search_workspace_contents();
                    }
                });

                if !self.workspace_content_results.is_empty() {
                    ui.separator();
                    ui.heading("Content Matches");
                    let mut open_match: Option<PathBuf> = None;
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for hit in &self.workspace_content_results {
                                let rel = hit
                                    .path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(&hit.path)
                                    .display()
                                    .to_string();
                                let label = format!("{rel}:{}  {}", hit.line, hit.text.trim());
                                if ui.selectable_label(false, label).clicked() {
                                    open_match = Some(hit.path.clone());
                                }
                            }
                        });
                    if let Some(path) = open_match {
                        self.open_workspace_file(&path);
                    }
                }
            });
    }

    fn render_directory_node(&mut self, ui: &mut egui::Ui, dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();
            if a_is_dir != b_is_dir {
                b_is_dir.cmp(&a_is_dir)
            } else {
                a.file_name().cmp(&b.file_name())
            }
        });

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if name == ".git" || name == "target" || name == "build" {
                    continue;
                }
                ui.collapsing(name, |ui| {
                    self.render_directory_node(ui, &path);
                })
                .header_response
                .context_menu(|ui| {
                    if ui.button("New File").clicked() {
                        let file_path = path.join("untitled.dcrt");
                        let _ = fs::write(&file_path, "");
                        let _ = self.refresh_workspace();
                        ui.close_menu();
                    }
                    if ui.button("New Folder").clicked() {
                        let dir_path = path.join("new_folder");
                        let _ = fs::create_dir_all(&dir_path);
                        let _ = self.refresh_workspace();
                        ui.close_menu();
                    }
                });
            } else {
                let selected = self.file_path.as_ref() == Some(&path);
                let label = ui.selectable_label(selected, name);
                if label.clicked() {
                    self.open_workspace_file(&path);
                }
                label.context_menu(|ui| {
                    if ui.button("Delete File").clicked() {
                        let _ = fs::remove_file(&path);
                        let _ = self.refresh_workspace();
                        ui.close_menu();
                    }
                });
            }
        }
    }

    fn render_inspector_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("inspector_right")
            .resizable(true)
            .default_width(320.0)
            .min_width(250.0)
            .show_inside(ui, |ui| {
                ui.heading("Inspector");
                if let Some(path) = &self.file_path {
                    ui.label(format!("File: {}", path.display()));
                } else {
                    ui.label("File: <untitled>");
                }
                ui.label(format!("Target: {:?}", self.target));
                ui.label(format!("Lines: {}", self.code.lines().count()));
                ui.label(format!("Bytes: {}", self.code.len()));
                ui.separator();
                let ext = self
                    .file_path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|s| s.to_str())
                    .unwrap_or("dcrt")
                    .to_ascii_lowercase();
                if ext == "dcrt" {
                    ui.heading("Program");
                    match Parser::parse(&self.code) {
                        Ok(program) => {
                            self.render_program_summary(ui, &program);
                        }
                        Err(err) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(230, 100, 100),
                                format!("Parse error: {err}"),
                            );
                            if let Some(line) = parse_error_line(&err.to_string()) {
                                ui.label(format!("Likely line: {line}"));
                            }
                        }
                    }
                }
                ui.separator();
                ui.heading("Build History");
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for record in self.build_history.iter().rev().take(20) {
                            let status = if record.success { "ok" } else { "fail" };
                            ui.label(format!(
                                "[{status}] {} {:?} -> {}",
                                record.mode,
                                record.target,
                                record.output.display()
                            ));
                            if !record.message.is_empty() {
                                ui.small(record.message.clone());
                            }
                            let age = record.when.elapsed().as_secs();
                            ui.small(format!("{age}s ago"));
                        }
                    });
                ui.separator();
                ui.heading("Command Runner");
                ui.add(
                    egui::TextEdit::singleline(&mut self.external_command)
                        .desired_width(f32::INFINITY)
                        .hint_text("command"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.external_args)
                        .desired_width(f32::INFINITY)
                        .hint_text("args"),
                );
                if ui.button("Run Command").clicked() {
                    self.run_external_command();
                }
                if self.show_help_panel {
                    ui.separator();
                    ui.heading("Shortcuts");
                    ui.label("Ctrl+S Save");
                    ui.label("Ctrl+O Open");
                    ui.label("Ctrl+B Compile");
                    ui.label("Ctrl+K Validate");
                    ui.label("F3 Find next");
                }
            });
    }

    fn render_program_summary(&self, ui: &mut egui::Ui, program: &Program) {
        ui.label(format!("Entry: {}", program.entry_event));
        ui.label(format!("Data: {}", program.data.len()));
        ui.label(format!("Blocks: {}", program.blocks.len()));
        ui.separator();
        ui.collapsing("Events/Procedures", |ui| {
            for block in &program.blocks {
                ui.label(format!("{:?}: {}", block.kind, block.name));
            }
        });
    }

    fn render_outline_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("outline_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(120.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Outline");
                    if ui.button("X").clicked() {
                        self.show_outline_panel = false;
                    }
                });
                ui.separator();
                self.parse_outline();
                let symbols = self.outline_symbols.clone();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for sym in &symbols {
                        let label = format!("{} {}", sym.kind, sym.name);
                        if ui.selectable_label(false, &label).clicked() {

                            self.jump_to_line(sym.line);
                        }
                    }
                });
            });
    }

    fn render_editor_context_menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Undo (Ctrl+Z)").clicked() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.undo();
                self.code = tab.code.clone();
                self.dirty = tab.dirty;
            }
            ui.close();
        }
        if ui.button("Redo (Ctrl+Shift+Z)").clicked() {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                tab.redo();
                self.code = tab.code.clone();
                self.dirty = tab.dirty;
            }
            ui.close();
        }
        ui.separator();
        if ui.button("Cut").clicked() {
            ui.close();
        }
        if ui.button("Copy").clicked() {
            ui.close();
        }
        if ui.button("Paste").clicked() {
            ui.close();
        }
        ui.separator();
        if ui.button("Compile (Ctrl+B)").clicked() {
            self.compile_code();
            ui.close();
        }
        if ui.button("Validate (Ctrl+K)").clicked() {
            self.validate_code();
            ui.close();
        }
    }

    fn render_compile_all_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Compile All Workspace")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                if self.show_progress {
                    ui.label(&self.progress_msg);
                    ui.spinner();
                } else {
                    ui.label("Compile all .dcrt files in the workspace?");
                    ui.horizontal(|ui| {
                        if ui.button("Compile").clicked() {
                            self.compile_all_workspace();
                        }
                        if ui.button("Close").clicked() {
                            self.show_compile_all = false;
                        }
                    });
                    if !self.workspace_build_output.is_empty() {
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&self.workspace_build_output)
                                        .monospace()
                                        .size(12.0),
                                );
                            });
                    }
                }
            });
    }

    fn jump_to_line(&mut self, line: usize) {
        let mut target_idx = 0;
        for _ in 1..line {
            if let Some(idx) = self.code[target_idx..].find('\n') {
                target_idx += idx + 1;
            } else {
                break;
            }
        }
        if target_idx <= self.code.len() {
            self.cursor_row_col = Some((line, 1));
        }
    }

    fn parse_outline(&mut self) {
        self.outline_symbols.clear();
        for (line_idx, line) in self.code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("event ") {
                let name = trimmed[6..].trim().trim_end_matches(':');
                self.outline_symbols.push(OutlineSymbol {
                    kind: "event".into(),
                    name: name.to_string(),
                    line: line_idx + 1,
                });
            } else if trimmed.starts_with("proc ") {
                let name = trimmed[5..].trim().trim_end_matches(':');
                self.outline_symbols.push(OutlineSymbol {
                    kind: "proc".into(),
                    name: name.to_string(),
                    line: line_idx + 1,
                });
            } else if trimmed.starts_with("data ") {
                let rest = trimmed[5..].trim();
                let name = rest.split(&[' ', '='][..]).next().unwrap_or(rest);
                self.outline_symbols.push(OutlineSymbol {
                    kind: "data".into(),
                    name: name.to_string(),
                    line: line_idx + 1,
                });
            } else if trimmed.starts_with("target ") {
                let name = trimmed[7..].trim();
                self.outline_symbols.push(OutlineSymbol {
                    kind: "target".into(),
                    name: name.to_string(),
                    line: line_idx + 1,
                });
            } else if trimmed.starts_with("entry ") {
                let name = trimmed[6..].trim();
                self.outline_symbols.push(OutlineSymbol {
                    kind: "entry".into(),
                    name: name.to_string(),
                    line: line_idx + 1,
                });
            }
        }
    }

    fn parse_errors(&mut self) {
        self.error_lines.clear();
        self.last_error.clear();
        match Parser::parse(&self.code) {
            Ok(program) => {
                if program.target.is_empty() {
                    self.error_lines.push(1);
                    self.last_error = "No target specified".into();
                }
                if program.entry_event.is_empty() {
                    let has_main = program.blocks.iter().any(|b| b.name == "main");
                    if !has_main {
                        for (i, line) in self.code.lines().enumerate() {
                            if line.trim().starts_with("event ") {
                                self.error_lines.push(i + 1);
                                self.last_error = "No entry point specified. Use 'entry' directive.".into();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                self.error_lines.push(1);
                self.last_error = format!("Parse error: {}", e);
            }
        }
    }

    fn compile_all_workspace(&mut self) {
        self.show_progress = true;
        self.progress_msg = "Compiling workspace...".to_string();
        self.workspace_build_output.clear();

        let files: Vec<PathBuf> = self.workspace_files.clone();
        let mut results: Vec<Result<String, String>> = Vec::new();

        for file in &files {
            match fs::read_to_string(file) {
                Ok(source) => match Parser::parse(&source) {
                    Ok(program) => {
                        let out_name = file.with_extension("");
                        let out_file = PathBuf::from("build").join(
                            out_name.file_name().unwrap_or_default(),
                        );
                        results.push(
                            dispatch_compile(&program, self.target, &out_file)
                                .map(|msg| format!("{} -> {}", file.display(), msg))
                        );
                    }
                    Err(e) => {
                        results.push(Err(format!("{}: parse error: {}", file.display(), e)));
                    }
                },
                Err(e) => {
                    results.push(Err(format!("{}: read error: {}", file.display(), e)));
                }
            }
        }

        let mut buf = String::new();
        for r in &results {
            match r {
                Ok(msg) => {
                    buf.push_str(&format!("OK  {}\n", msg));
                }
                Err(e) => {
                    buf.push_str(&format!("ERR {}\n", e));
                }
            }
        }
        buf.push_str(&format!("\n{} file(s) compiled, {} error(s)",
            results.len(),
            results.iter().filter(|r| r.is_err()).count()));
        self.workspace_build_output = buf;
        self.show_progress = false;
        self.progress_msg.clear();
    }

    fn render_bottom_panel(&mut self, ui: &mut egui::Ui) {
        if !self.show_output_panel {
            return;
        }
        egui::TopBottomPanel::bottom("output_bottom")
            .resizable(true)
            .default_height(220.0)
            .min_height(120.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    // Tab buttons: Output / Run Output
                    let out_tab = ui.selectable_label(self.show_run_output, "Output").clicked();
                    let run_tab = ui.selectable_label(!self.show_run_output, "Run Output").clicked();
                    if out_tab { self.show_run_output = false; }
                    if run_tab { self.show_run_output = true; }

                    if ui.button("Clear").clicked() {
                        if self.show_run_output { self.run_output.clear(); }
                        else { self.output_log.clear(); }
                    }
                    if ui.button("Hide").clicked() {
                        self.show_output_panel = false;
                    }
                    if !self.show_run_output {
                        if ui.button("Open Output Folder").clicked() {
                            self.open_output_folder();
                        }
                        if ui.button("Export Log").clicked() {
                            self.export_output_log();
                        }
                    }
                });
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let text = if self.show_run_output {
                            &self.run_output
                        } else {
                            &self.output_log
                        };
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(text)
                                    .monospace()
                                    .size(13.0)
                                    .color(egui::Color32::from_rgb(190, 190, 206)),
                            )
                            .selectable(true),
                        );
                    });
            });
    }

    fn render_editor(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {

                        let lines: Vec<&str> = self.code.lines().collect();
                        let line_count = lines.len().max(1);
                        let digit_count = line_count.to_string().len().max(3);

                        let mut bracket_pairs: Vec<(usize, usize)> = Vec::new();
                        if let Some((row, col)) = self.cursor_row_col {
                            let cursor_idx = line_idx_to_byte(&self.code, row.saturating_sub(1))
                                + col.saturating_sub(1);
                            let chars: Vec<char> = self.code.chars().collect();
                            if cursor_idx < chars.len() {
                                let c = chars[cursor_idx];
                                if let Some(matching) = find_matching_bracket(&chars, cursor_idx, c) {
                                    bracket_pairs.push((cursor_idx, matching));
                                }
                            }
                        }

                        let mut gutter_buf = String::new();
                        for i in 0..line_count {
                            let n = i + 1;
                            let err_char = if self.error_lines.contains(&n) { "⚠" } else { " " };
                            gutter_buf.push_str(&format!("{:>width$} {}\n", n, err_char, width = digit_count));
                        }
                        let gutter_resp = ui.add(
                            egui::Label::new(
                                egui::RichText::new(gutter_buf)
                                    .monospace()
                                    .size(14.0)  // match editor font size
                                    .color(egui::Color32::from_rgb(95, 95, 116)),
                            )
                            .sense(egui::Sense::click())
                            .selectable(false),
                        );
                        if let Some(pointer_pos) = gutter_resp.hover_pos() {
                            if gutter_resp.clicked() {
                                let rel_y = pointer_pos.y - gutter_resp.rect.top();
                                // Exact line height from the label's actual rendered height
                                let line_h = gutter_resp.rect.height() / line_count as f32;
                                let line = (rel_y / line_h.max(1.0)) as usize + 1;
                                let clamped = line.min(line_count);
                                self.cursor_row_col = Some((clamped, 1));
                                self.push_log("editor", &format!("goto line {clamped}"));
                            }
                        }

                        let ext = self
                            .file_path
                            .as_ref()
                            .and_then(|p| p.extension())
                            .and_then(|s| s.to_str())
                            .unwrap_or("dcrt")
                            .to_ascii_lowercase();

                        let error_lines_copy = self.error_lines.clone();
                        let bracket_pairs_copy = bracket_pairs.clone();
                        let mut layouter =
                            |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                                let mut job = if ext == "dcrt" {
                                    let base = highlight_decretum(text.as_str(), &error_lines_copy);
                                    add_bracket_highlight(base, &bracket_pairs_copy, text.as_str())
                                } else {
                                    let theme =
                                        egui_extras::syntax_highlighting::CodeTheme::from_memory(
                                            ui.ctx(),
                                            ui.style(),
                                        );
                                    let base = egui_extras::syntax_highlighting::highlight(
                                        ui.ctx(),
                                        ui.style(),
                                        &theme,
                                        text.as_str(),
                                        &ext,
                                    );
                                    add_bracket_highlight(base, &bracket_pairs_copy, text.as_str())
                                };
                                job.wrap.max_width = wrap_width;
                                ui.fonts_mut(|f| f.layout_job(job))
                            };

                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.code)
                                .font(FontId::monospace(14.0))
                                .code_editor()
                                .desired_rows(38)
                                .lock_focus(true)
                                .desired_width(f32::INFINITY)
                                .layouter(&mut layouter),
                        );

                        if response.changed() {
                            if self.enter_pressed_flag {
                                self.enter_pressed_flag = false;
                                if let Some(state) =
                                    egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                                {
                                    if let Some(cursor_range) = state.cursor.char_range() {
                                        let cursor_idx = cursor_range.primary.index;
                                        let line = self.code[..cursor_idx].bytes().filter(|&b| b == b'\n').count();
                                        let prev_line = if line > 0 { line - 1 } else { 0 };
                                        let ws = leading_whitespace_of_line(&self.code, prev_line);
                                        let byte_pos = line_idx_to_byte(&self.code, line);
                                        if byte_pos <= self.code.len() {
                                            let rest = &self.code[byte_pos..];
                                            if rest.trim_start().is_empty() || rest.trim_start().starts_with('\n') {
                                                self.code = format!("{}{}{}", &self.code[..byte_pos], ws, rest);
                                            }
                                        }
                                    }
                                }
                            }

                            // Update previous_code for next comparison
                            self.previous_code = self.code.clone();

                            let now = Instant::now();
                            if now.duration_since(self.undo_debounce) > Duration::from_millis(500) {
                                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                                    tab.snapshot();
                                }
                                self.undo_debounce = now;
                            }
                            self.dirty = true;
                            self.sync_buffer_to_active_tab();
                            self.parse_outline();
                            self.parse_errors();
                        }

                        if let Some(state) =
                            egui::text_edit::TextEditState::load(ui.ctx(), response.id)
                        {
                            if let Some(range) = state.cursor.char_range() {
                                let idx = range.primary.index;
                                let mut row = 1;
                                let mut col = 1;
                                for (i, ch) in self.code.char_indices() {
                                    if i >= idx {
                                        break;
                                    }
                                    if ch == '\n' {
                                        row += 1;
                                        col = 1;
                                    } else {
                                        col += 1;
                                    }
                                }
                                self.cursor_row_col = Some((row, col));
                            }
                        }

                        response.context_menu(|ui| {
                            self.render_editor_context_menu(ui);
                        });
                    });
                });

            ui.separator();
            ui.horizontal(|ui| {
                ui.colored_label(self.status_color, &self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let file_text = self
                        .file_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "untitled.dcrt".to_string());
                    ui.colored_label(egui::Color32::from_rgb(110, 110, 134), file_text);
                });
            });
        });
    }

    fn open_file_picker(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Decretum source", &["dcrt"])
            .add_filter("All files", &["*"])
            .pick_file()
        {
            self.open_workspace_file(&path);
        }
    }

    fn open_workspace_file(&mut self, path: &Path) {
        if let Some(existing_idx) = self
            .tabs
            .iter()
            .position(|t| t.file_path.as_deref() == Some(path))
        {
            self.switch_to_tab(existing_idx);
            return;
        }
        match fs::read_to_string(path) {
            Ok(content) => {
                let title = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file.dcrt")
                    .to_string();
                self.open_tab_with_content(Some(path.to_path_buf()), title, content);
                self.status = format!("Opened {}", path.display());
                self.status_color = egui::Color32::from_rgb(120, 180, 120);
                self.last_autosave_at = Instant::now();
                let mut suggested = PathBuf::from("build");
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                suggested.push(stem);
                suggested.set_extension(self.target.extension());
                self.out_file = suggested.display().to_string();
            }
            Err(err) => {
                self.status = format!("Open failed: {err}");
                self.status_color = egui::Color32::from_rgb(220, 80, 80);
            }
        }
    }

    fn new_file(&mut self) {
        self.open_tab_with_content(
            None,
            format!("untitled{}.dcrt", self.next_tab_id),
            "target portable\nentry main\n\nevent main:\n    exit 0\n".to_string(),
        );
        self.dirty = true;
        self.last_autosave_at = Instant::now();
        self.status = "New file".to_string();
        self.status_color = egui::Color32::from_rgb(120, 180, 120);
    }

    fn save_current_file(&mut self) {
        if let Some(path) = self.file_path.clone() {
            self.write_current_to_path(&path);
        } else {
            self.save_as();
        }
    }

    fn save_all_tabs(&mut self) {
        self.sync_buffer_to_active_tab();
        let active = self.active_tab;
        for idx in 0..self.tabs.len() {
            self.active_tab = idx;
            self.load_active_tab_to_buffer();
            if self.file_path.is_some() {
                self.save_current_file();
                self.sync_buffer_to_active_tab();
            }
        }
        self.active_tab = active.min(self.tabs.len().saturating_sub(1));
        self.load_active_tab_to_buffer();
        self.push_log("save", "save all complete");
    }

    fn save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("program.dcrt")
            .add_filter("Decretum source", &["dcrt"])
            .add_filter("All files", &["*"])
            .save_file()
        {
            self.write_current_to_path(&path);
            self.file_path = Some(path.clone());
            let _ = self.refresh_workspace();
        }
    }

    fn write_current_to_path(&mut self, path: &Path) {
        match fs::write(path, &self.code) {
            Ok(_) => {
                self.status = format!("Saved {}", path.display());
                self.status_color = egui::Color32::from_rgb(120, 180, 120);
                self.dirty = false;
                self.last_autosave_at = Instant::now();
                self.file_path = Some(path.to_path_buf());
                self.sync_buffer_to_active_tab();
            }
            Err(err) => {
                self.status = format!("Save failed: {err}");
                self.status_color = egui::Color32::from_rgb(220, 80, 80);
            }
        }
    }

    fn refresh_workspace(&mut self) -> Result<(), String> {
        let mut files = Vec::new();
        collect_all_files_recursive(&self.workspace_root, &mut files)?;
        files.sort();
        self.workspace_files = files;
        self.push_log(
            "workspace",
            &format!("indexed {} .dcrt files", self.workspace_files.len()),
        );
        Ok(())
    }

    fn validate_code(&mut self) {
        match Parser::parse(&self.code) {
            Ok(program) => {
                let msg = format!(
                    "Valid -- target={} entry={} data={} blocks={}",
                    program.target,
                    program.entry_event,
                    program.data.len(),
                    program.blocks.len()
                );
                self.status = msg.clone();
                self.status_color = egui::Color32::from_rgb(120, 180, 120);
                self.push_log("validate", &msg);
            }
            Err(err) => {
                self.status = format!("Parse Error: {err}");
                self.status_color = egui::Color32::from_rgb(230, 100, 100);
                self.push_log("validate", &format!("ERROR: {err}"));
            }
        }
    }

    fn compile_code(&mut self) {
        self.status = "Compiling...".to_string();
        self.status_color = egui::Color32::from_rgb(200, 200, 100);

        if let Err(err) = Parser::parse(&self.code).map_err(|e| e.to_string()) {
            self.status = format!("Parse Error: {err}");
            self.status_color = egui::Color32::from_rgb(230, 100, 100);
            self.last_error = err.clone();
            self.push_log("compile", &format!("PARSE ERROR: {err}"));
            self.parse_errors();
            return;
        }

        let out_path = normalize_out_path(&self.out_file, self.target);
        if let Some(parent) = out_path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            let msg = format!("Build Error: failed to create {}: {err}", parent.display());
            self.status = msg.clone();
            self.status_color = egui::Color32::from_rgb(230, 100, 100);
            self.push_log("compile", &msg);
            return;
        }

        let result = self.compile_via_rust_backend(&out_path);

        match result {
            Ok(msg) => {
                self.status = msg.clone();
                self.status_color = egui::Color32::from_rgb(120, 180, 120);
                self.last_artifact = Some(out_path.clone());
                self.out_file = out_path.display().to_string();
                self.push_log("compile", &format!("wrote {}", out_path.display()));
                self.log_build("single-file".to_string(), out_path, true, msg);
            }
            Err(err) => {
                self.status = format!("Compile Error: {err}");
                self.status_color = egui::Color32::from_rgb(230, 100, 100);
                self.push_log("compile", &format!("ERROR: {err}"));
                self.log_build(
                    "single-file".to_string(),
                    out_path,
                    false,
                    format!("compile error: {err}"),
                );
            }
        }
    }

    fn compile_via_rust_backend(&mut self, out_path: &Path) -> Result<String, String> {
        let program = Parser::parse(&self.code).map_err(|e| e.to_string())?;
        dispatch_compile(&program, self.target, out_path)
    }

    fn run_last_artifact(&mut self) {
        let Some(path) = self.last_artifact.clone() else {
            self.push_log("run", "no compiled artifact yet");
            return;
        };
        if !path.exists() {
            self.push_log("run", &format!("missing {}", path.display()));
            return;
        }
        self.run_virtual(&path);
    }

    fn run_virtual(&mut self, path: &Path) {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        self.show_run_output = true;
        self.run_output.clear();
        match ext {
            "exe" | "efi" => {
                let path_buf = path.to_path_buf();
                let output_arc = Arc::new(Mutex::new(String::new()));
                let out_clone = output_arc.clone();
                thread::spawn(move || {
                    match Command::new(&path_buf).output() {
                        Ok(output) => {
                            let mut buf = out_clone.lock().unwrap();
                            if !output.stdout.is_empty() {
                                buf.push_str(&String::from_utf8_lossy(&output.stdout));
                            }
                            if !output.stderr.is_empty() {
                                buf.push_str(&format!("\nstderr:\n{}", String::from_utf8_lossy(&output.stderr)));
                            }
                            if output.stdout.is_empty() && output.stderr.is_empty() {
                                buf.push_str(&format!("[exit code {}]\n", output.status.code().unwrap_or(-1)));
                            }
                        }
                        Err(e) => {
                            let mut buf = out_clone.lock().unwrap();
                            buf.push_str(&format!("launch failed: {e}"));
                        }
                    }
                });
                self.run_output_arc = Some(output_arc);
                self.push_log("run", &format!("launched {}", path.display()));
            }
            "dcb" => {
                match std::fs::read(path) {
                    Ok(data) => match BytecodeRuntime::from_bytes(&data) {
                        Ok(mut rt) => match rt.run_entry() {
                            Ok(code) => {
                                self.run_output = format!("bytecode exit code: {code}\n");
                                self.push_log("run", &format!("bytecode exit code: {code}"))
                            }
                            Err(e) => {
                                self.run_output = format!("bytecode error: {e}");
                                self.push_log("run", &format!("bytecode error: {e}"))
                            }
                        },
                        Err(e) => {
                            self.run_output = format!("bytecode load error: {e}");
                            self.push_log("run", &format!("bytecode load error: {e}"))
                        }
                    },
                    Err(e) => {
                        self.run_output = format!("read error: {e}");
                        self.push_log("run", &format!("read error: {e}"))
                    }
                }
            }
            "img" | "bin" => {
                match Command::new("qemu-system-x86_64")
                    .args(["-drive", &format!("file={},format=raw", path.display()), "-m", "16"])
                    .spawn()
                {
                    Ok(_child) => {
                        self.run_output = format!("Launched QEMU with {}\n", path.display());
                        self.push_log("run", &format!("launched qemu with {}", path.display()));
                    }
                    Err(_) => {
                        self.run_output = "QEMU not found. Install qemu-system-x86_64 to run BIOS images.".to_string();
                        self.push_log("run", "QEMU not found.");
                    }
                }
            }
            _ => {
                self.run_output = format!("no runner for .{} files", ext);
                self.push_log("run", &format!("no runner for .{} files", ext));
            }
        }
    }

    fn find_next(&mut self) -> Option<usize> {
        if self.find_text.is_empty() {
            return None;
        }
        let hay = if self.case_sensitive_search {
            self.code.clone()
        } else {
            self.code.to_ascii_lowercase()
        };
        let needle = if self.case_sensitive_search {
            self.find_text.clone()
        } else {
            self.find_text.to_ascii_lowercase()
        };
        let start = self
            .last_find_index
            .map(|i| i.saturating_add(needle.len()))
            .unwrap_or(0);
        let found = hay[start..]
            .find(&needle)
            .map(|offset| start + offset)
            .or_else(|| hay[..start].find(&needle));
        if let Some(idx) = found {
            self.last_find_index = Some(idx);
            let line = self.code[..idx].bytes().filter(|b| *b == b'\n').count() + 1;
            self.status = format!("Found at line {line}");
            self.status_color = egui::Color32::from_rgb(130, 170, 230);
            self.push_log("find", &format!("match at line {line}"));
        } else {
            self.status = "No match".to_string();
            self.status_color = egui::Color32::from_rgb(230, 180, 90);
            self.push_log("find", "no matches");
        }
        found
    }

    fn replace_next(&mut self) -> Option<usize> {
        let idx = self.find_next()?;
        let end = idx.saturating_add(self.find_text.len());
        self.code.replace_range(idx..end, &self.replace_text);
        self.dirty = true;
        self.last_find_index = Some(idx);
        self.push_log("replace", "replaced one occurrence");
        Some(idx)
    }

    fn replace_all(&mut self) -> usize {
        if self.find_text.is_empty() {
            return 0;
        }
        let count = if self.case_sensitive_search {
            self.code.matches(&self.find_text).count()
        } else {
            self.code
                .to_ascii_lowercase()
                .matches(&self.find_text.to_ascii_lowercase())
                .count()
        };
        if count == 0 {
            return 0;
        }
        if self.case_sensitive_search {
            self.code = self.code.replace(&self.find_text, &self.replace_text);
        } else {

            let old_find = self.find_text.clone();
            self.last_find_index = None;
            let mut replaced = 0usize;
            while self.find_next().is_some() {
                if self.replace_next().is_some() {
                    replaced += 1;
                } else {
                    break;
                }
                if replaced > 200_000 {
                    break;
                }
            }
            self.find_text = old_find;
            self.dirty = true;
            return replaced;
        }
        self.dirty = true;
        count
    }

    fn insert_snippet(&mut self, snippet: Snippet) {
        let text = match snippet {
            Snippet::PortableHello => {
                r#"target portable
entry main

event main:
    print "Hello, Decretum!\n"
    exit 0
"#
            }
            Snippet::BiosSkeleton => {
                r#"target bios16
entry start

data hello = "Booted from Decretum BIOS16\0"

event start:
    builtin.clear_screen
    builtin.print hello
    builtin.newline
    builtin.wait_key
    builtin.halt
    ret
"#
            }
            Snippet::ProcTemplate => {
                r#"proc do_work:
    ; your logic
    ret
"#
            }
            Snippet::RisCvCheri => {
                r#"target riscv_cheri
entry main

event main:
    li a0, 0
    li a1, 1
    li a2, 20
    while a2
        if a1
            add a0, a0, a1
        else
            slli a4, a1, 1
            add a0, a0, a4
        endif
        addi a1, a1, 1
        addi a2, a2, -1
    endwhile
    ret
"#
            }
            Snippet::ArmCm => {
                r#"target armcm
entry main

event main:
    mov r0, 10
    mov r1, 0
loop:
    sub r0, r0, 1
    cmp r0, 0
    bne main.loop
    ret
"#
            }
            Snippet::EightBit => {
                r#"target 6502
entry main

event main:
    lda a, 65
    ldx x, 2
    nop
    ret
"#
            }
            Snippet::PortLoop => {
                r#"target portable
entry main

event main:
    mov r0, 0
    mov r1, 100
loop:
    cmp r0, r1
    jge done
    add r0, r0, 1
    jmp loop
done:
    print "Done!\n"
    exit 0
"#
            }
        };
        self.code.push('\n');
        self.code.push_str(text);
        self.dirty = true;
        self.push_log("insert", "snippet inserted");
    }

    fn open_workspace_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.workspace_root = folder;
            self.workspace_content_results.clear();
            match self.refresh_workspace() {
                Ok(_) => {
                    self.status = format!("Workspace set to {}", self.workspace_root.display());
                    self.status_color = egui::Color32::from_rgb(120, 180, 120);
                }
                Err(err) => {
                    self.status = format!("Workspace load failed: {err}");
                    self.status_color = egui::Color32::from_rgb(230, 100, 100);
                }
            }
        }
    }

    fn search_workspace_contents(&mut self) {
        self.workspace_content_results.clear();
        let query = self.workspace_content_query.trim().to_string();
        if query.is_empty() {
            self.push_log("search", "empty query");
            return;
        }
        let mut hits = Vec::new();
        for path in &self.workspace_files {
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            for (line_idx, line) in content.lines().enumerate() {
                let matched = if self.case_sensitive_search {
                    line.contains(&query)
                } else {
                    line.to_ascii_lowercase()
                        .contains(&query.to_ascii_lowercase())
                };
                if matched {
                    hits.push(WorkspaceSearchResult {
                        path: path.clone(),
                        line: line_idx + 1,
                        text: line.to_string(),
                    });
                    if hits.len() >= 2500 {
                        break;
                    }
                }
            }
            if hits.len() >= 2500 {
                break;
            }
        }
        self.workspace_content_results = hits;
        self.push_log(
            "search",
            &format!(
                "found {} matches for '{}'",
                self.workspace_content_results.len(),
                query
            ),
        );
    }

    fn compile_workspace_project(&mut self) {
        let out_path = normalize_out_path(&self.out_file, self.target);
        if let Some(parent) = out_path.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            self.push_log(
                "compile-project",
                &format!("failed to create output dir: {err}"),
            );
            return;
        }
        let source = match load_project_source_from_root(&self.workspace_root) {
            Ok(s) => s,
            Err(err) => {
                self.push_log("compile-project", &format!("source load failed: {err}"));
                self.log_build(
                    "project".to_string(),
                    out_path,
                    false,
                    format!("source load failed: {err}"),
                );
                return;
            }
        };
        let program = match Parser::parse(&source) {
            Ok(p) => p,
            Err(err) => {
                let msg = format!("parse error: {err}");
                self.push_log("compile-project", &msg);
                self.log_build("project".to_string(), out_path, false, msg);
                return;
            }
        };

        let result = dispatch_compile(&program, self.target, &out_path);
        match result {
            Ok(msg) => {
                self.status = "Workspace project compile successful".to_string();
                self.status_color = egui::Color32::from_rgb(120, 180, 120);
                self.last_artifact = Some(out_path.clone());
                self.push_log("compile-project", &msg);
                self.log_build("project".to_string(), out_path, true, msg);
            }
            Err(err) => {
                let msg = format!("project compile failed: {err}");
                self.status = msg.clone();
                self.status_color = egui::Color32::from_rgb(230, 100, 100);
                self.push_log("compile-project", &msg);
                self.log_build("project".to_string(), out_path, false, msg);
            }
        }
    }

    fn run_external_command(&mut self) {
        let command = self.external_command.trim().to_string();
        if command.is_empty() {
            self.push_log("command", "command is empty");
            return;
        }
        let args = split_simple_args(self.external_args.trim());
        let output = Command::new(&command)
            .args(&args)
            .current_dir(&self.workspace_root)
            .output();
        match output {
            Ok(out) => {
                self.push_log("command", &format!("$ {} {}", command, args.join(" ")));
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stdout.trim().is_empty() {
                    self.push_log("command-stdout", stdout.trim());
                }
                if !stderr.trim().is_empty() {
                    self.push_log("command-stderr", stderr.trim());
                }
                self.push_log("command", &format!("exit status {}", out.status));
            }
            Err(err) => {
                self.push_log("command", &format!("failed: {err}"));
            }
        }
    }

    fn export_diagnostics_report(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("decretum_diagnostics.md")
            .save_file()
        else {
            return;
        };
        let mut report = String::new();
        report.push_str("# Decretum IDE Diagnostics\n\n");
        report.push_str(&format!(
            "- Workspace: `{}`\n",
            self.workspace_root.display()
        ));
        report.push_str(&format!(
            "- File: `{}`\n",
            self.file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<untitled>".to_string())
        ));
        report.push_str(&format!("- Target: `{:?}`\n", self.target));
        report.push_str(&format!("- Lines: `{}`\n", self.code.lines().count()));
        report.push_str(&format!("- Bytes: `{}`\n", self.code.len()));
        report.push_str("\n## Parse\n\n");
        match Parser::parse(&self.code) {
            Ok(program) => {
                report.push_str(&format!(
                    "Valid program. Entry `{}`, data `{}`, blocks `{}`.\n",
                    program.entry_event,
                    program.data.len(),
                    program.blocks.len()
                ));
            }
            Err(err) => {
                report.push_str(&format!("Parse error: `{err}`\n"));
            }
        }
        report.push_str("\n## Recent Build History\n\n");
        for rec in self.build_history.iter().rev().take(20) {
            report.push_str(&format!(
                "- [{}] mode=`{}` target=`{:?}` out=`{}` msg=`{}`\n",
                if rec.success { "ok" } else { "fail" },
                rec.mode,
                rec.target,
                rec.output.display(),
                rec.message.replace('\n', " ")
            ));
        }

        match fs::write(&path, report) {
            Ok(_) => self.push_log("diagnostics", &format!("wrote {}", path.display())),
            Err(err) => self.push_log("diagnostics", &format!("failed: {err}")),
        }
    }

    fn export_output_log(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("decretum_ide.log")
            .save_file()
        else {
            return;
        };
        match fs::write(&path, &self.output_log) {
            Ok(_) => self.push_log("log", &format!("wrote {}", path.display())),
            Err(err) => self.push_log("log", &format!("failed to export log: {err}")),
        }
    }

    fn open_output_folder(&mut self) {
        let path = PathBuf::from(&self.out_file);
        let folder = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.workspace_root.clone());
        let result = Command::new("explorer.exe").arg(folder.as_os_str()).spawn();
        if let Err(err) = result {
            self.push_log("open-folder", &format!("failed: {err}"));
        }
    }

    fn log_build(&mut self, mode: String, output: PathBuf, success: bool, message: String) {
        self.build_history.push(BuildRecord {
            when: Instant::now(),
            mode,
            target: self.target,
            output,
            success,
            message,
        });
        if self.build_history.len() > 200 {
            let drop_count = self.build_history.len() - 200;
            self.build_history.drain(0..drop_count);
        }
    }

    fn push_log(&mut self, tag: &str, msg: &str) {
        self.output_log.push_str(&format!("[{tag}] {msg}\n"));
    }
}

#[derive(Clone, Copy)]
enum Snippet {
    PortableHello,
    BiosSkeleton,
    ProcTemplate,
    RisCvCheri,
    ArmCm,
    EightBit,
    PortLoop,
}

fn dispatch_compile(program: &Program, target: CompileTarget, out_path: &Path) -> Result<String, String> {
    match target {
        CompileTarget::Pe => PortableBuilder::build_pe(program, out_path)
            .map(|o| format!("PE -> {} + {}", o.pe_path.display(), o.bytecode_path.display())),
        CompileTarget::Bytecode => PortableBuilder::build_bytecode(program, out_path)
            .map(|o| format!("Bytecode -> {}", o.bytecode_path.display())),
        CompileTarget::Bios16 => DirectBiosBuilder::build_boot_image(program, out_path)
            .map(|o| format!("BIOS16 -> {} ({} sectors)", o.image_path.display(), o.sectors_loaded)),
        CompileTarget::Uefi => DirectUefiBuilder::build_efi(program, out_path)
            .map(|o| format!("UEFI -> {}", o.efi_path.display())),
        CompileTarget::X8664 => DirectX86_64Builder::build_bin(program, out_path)
            .map(|o| format!("x86-64 -> {}", o.bin_path.display())),
        CompileTarget::Win64 => PortableBuilder::build_pe(program, out_path)
            .map(|o| format!("Win64 -> {}", o.pe_path.display())),
        CompileTarget::Win32 => DirectWin32Builder::build_pe(program, out_path)
            .map(|o| format!("Win32 -> {}", o.pe_path.display())),
        CompileTarget::Elf64 => DirectElfBuilder::build_elf(program, out_path)
            .map(|o| format!("ELF64 -> {}", o.elf_path.display())),
        CompileTarget::Elf32 => DirectElf32Builder::build_elf(program, out_path)
            .map(|o| format!("ELF32 -> {}", o.elf_path.display())),
        CompileTarget::I8086 => I8086Builder::build_bin(program, out_path)
            .map(|o| format!("i8086 -> {}", o.bin_path.display())),
        CompileTarget::I4004 => I4004Builder::build_bin(program, out_path)
            .map(|o| format!("I4004 -> {}", o.bin_path.display())),
        CompileTarget::I8008 => I8008Builder::build_bin(program, out_path)
            .map(|o| format!("I8008 -> {}", o.bin_path.display())),
        CompileTarget::I8080 => I8080Builder::build_bin(program, out_path)
            .map(|o| format!("I8080 -> {}", o.bin_path.display())),
        CompileTarget::ArmCm => DirectArmCmBuilder::build_bin(program, out_path)
            .map(|o| format!("ARM CM -> {} ({} bytes)", o.bin_path.display(), o.bin_size)),
        CompileTarget::Arm7tdmi => Arm7tdmiBuilder::build_bin(program, out_path)
            .map(|o| format!("ARM7TDMI -> {}", o.bin_path.display())),
        CompileTarget::Arm9 => Arm9Builder::build_bin(program, out_path)
            .map(|o| format!("ARM9 -> {}", o.bin_path.display())),
        CompileTarget::Aarch64 => DirectAarch64Builder::build_bin(program, out_path)
            .map(|o| format!("AArch64 -> {}", o.bin_path.display())),
        CompileTarget::Macho => DirectMachoBuilder::build_macho(program, out_path)
            .map(|o| format!("Mach-O -> {}", o.macho_path.display())),
        CompileTarget::Cheri => DirectCheriBuilder::build_bin(program, out_path)
            .map(|o| format!("CHERI -> {} ({} bytes)", o.bin_path.display(), o.bin_size)),
        CompileTarget::RiscV => DirectRiscvBuilder::build_bin(program, out_path)
            .map(|o| format!("RISC-V -> {} ({} bytes)", o.bin_path.display(), o.bin_size)),
        CompileTarget::RiscV64 => DirectRiscvBuilder::build_bin(program, out_path)
            .map(|o| format!("RISC-V 64 -> {} ({} bytes)", o.bin_path.display(), o.bin_size)),
        CompileTarget::RiscVCheri => DirectRisCvCheriBuilder::build_bin(program, out_path)
            .map(|o| format!("RISC-V CHERI -> {} ({} bytes)", o.bin_path.display(), o.bin_size)),
        CompileTarget::Mips => DirectMipsBuilder::build_bin(program, out_path)
            .map(|o| format!("MIPS -> {}", o.bin_path.display())),
        CompileTarget::Ppc => DirectPpcBuilder::build_bin(program, out_path)
            .map(|o| format!("PPC -> {}", o.bin_path.display())),
        CompileTarget::Ppc740 => Ppc740Builder::build_bin(program, out_path)
            .map(|o| format!("PPC740 -> {}", o.bin_path.display())),
        CompileTarget::Ppc970 => Ppc970Builder::build_bin(program, out_path)
            .map(|o| format!("PPC970 -> {}", o.bin_path.display())),
        CompileTarget::Sparc => DirectSparcBuilder::build_bin(program, out_path)
            .map(|o| format!("SPARC -> {}", o.bin_path.display())),
        CompileTarget::Alpha => DirectAlphaBuilder::build_bin(program, out_path)
            .map(|o| format!("Alpha -> {}", o.bin_path.display())),
        CompileTarget::Parisc => DirectPariscBuilder::build_bin(program, out_path)
            .map(|o| format!("PA-RISC -> {}", o.bin_path.display())),
        CompileTarget::OpenRisc => DirectOpenriscBuilder::build_bin(program, out_path)
            .map(|o| format!("OpenRISC -> {}", o.bin_path.display())),
        CompileTarget::Nios2 => DirectNios2Builder::build_bin(program, out_path)
            .map(|o| format!("Nios II -> {}", o.bin_path.display())),
        CompileTarget::Microblaze => DirectMicroblazeBuilder::build_bin(program, out_path)
            .map(|o| format!("MicroBlaze -> {}", o.bin_path.display())),
        CompileTarget::Mico32 => Mico32Builder::build_bin(program, out_path)
            .map(|o| format!("Mico32 -> {}", o.bin_path.display())),
        CompileTarget::Picoblaze => PicoblazeBuilder::build_bin(program, out_path)
            .map(|o| format!("PicoBlaze -> {}", o.bin_path.display())),
        CompileTarget::Mmix => MmixBuilder::build_bin(program, out_path)
            .map(|o| format!("MMIX -> {}", o.bin_path.display())),
        CompileTarget::Dlx => DlxBuilder::build_bin(program, out_path)
            .map(|o| format!("DLX -> {}", o.bin_path.display())),
        CompileTarget::Lc3 => Lc3Builder::build_bin(program, out_path)
            .map(|o| format!("LC-3 -> {}", o.bin_path.display())),
        CompileTarget::C6502 => Direct6502Builder::build_bin(program, out_path)
            .map(|o| format!("6502 -> {}", o.bin_path.display())),
        CompileTarget::Z80 => DirectZ80Builder::build_bin(program, out_path)
            .map(|o| format!("Z80 -> {}", o.bin_path.display())),
        CompileTarget::C6809 => Direct6809Builder::build_bin(program, out_path)
            .map(|o| format!("6809 -> {}", o.bin_path.display())),
        CompileTarget::M6800 => M6800Builder::build_bin(program, out_path)
            .map(|o| format!("M6800 -> {}", o.bin_path.display())),
        CompileTarget::Mos6501 => Mos6501Builder::build_bin(program, out_path)
            .map(|o| format!("MOS 6501 -> {}", o.bin_path.display())),
        CompileTarget::Pic => DirectPICBuilder::build_bin(program, out_path)
            .map(|o| format!("PIC -> {}", o.bin_path.display())),
        CompileTarget::Avr => DirectAvrBuilder::build_bin(program, out_path)
            .map(|o| format!("AVR -> {}", o.bin_path.display())),
        CompileTarget::Xc800 => Xc800Builder::build_bin(program, out_path)
            .map(|o| format!("XC800 -> {}", o.bin_path.display())),
        CompileTarget::Nec78k => Nec78kBuilder::build_bin(program, out_path)
            .map(|o| format!("NEC 78K -> {}", o.bin_path.display())),
        CompileTarget::R8c => R8cBuilder::build_bin(program, out_path)
            .map(|o| format!("R8C -> {}", o.bin_path.display())),
        CompileTarget::Msp430 => Msp430Builder::build_bin(program, out_path)
            .map(|o| format!("MSP430 -> {}", o.bin_path.display())),
        CompileTarget::C166 => C166Builder::build_bin(program, out_path)
            .map(|o| format!("C166 -> {}", o.bin_path.display())),
        CompileTarget::Rl78 => Rl78Builder::build_bin(program, out_path)
            .map(|o| format!("RL78 -> {}", o.bin_path.display())),
        CompileTarget::H8 => H8Builder::build_bin(program, out_path)
            .map(|o| format!("H8 -> {}", o.bin_path.display())),
        CompileTarget::M16c => M16cBuilder::build_bin(program, out_path)
            .map(|o| format!("M16C -> {}", o.bin_path.display())),
        CompileTarget::V20 => NecV20Builder::build_bin(program, out_path)
            .map(|o| format!("NEC V20 -> {}", o.bin_path.display())),
        CompileTarget::Rx => RxBuilder::build_bin(program, out_path)
            .map(|o| format!("RX -> {}", o.bin_path.display())),
        CompileTarget::Fr => FrBuilder::build_bin(program, out_path)
            .map(|o| format!("FR -> {}", o.bin_path.display())),
        CompileTarget::V810 => V810Builder::build_bin(program, out_path)
            .map(|o| format!("V810 -> {}", o.bin_path.display())),
        CompileTarget::Tms320 => Tms320Builder::build_bin(program, out_path)
            .map(|o| format!("TMS320 -> {}", o.bin_path.display())),
        CompileTarget::Blackfin => BlackfinBuilder::build_bin(program, out_path)
            .map(|o| format!("Blackfin -> {}", o.bin_path.display())),
        CompileTarget::Sharc => SharcBuilder::build_bin(program, out_path)
            .map(|o| format!("SHARC -> {}", o.bin_path.display())),
        CompileTarget::Sh2 => DirectSh2Builder::build_bin(program, out_path)
            .map(|o| format!("SH-2 -> {}", o.bin_path.display())),
        CompileTarget::Sh4 => DirectSh2Builder::build_bin(program, out_path)
            .map(|o| format!("SH-4 -> {}", o.bin_path.display())),
        CompileTarget::M68k => DirectM68kBuilder::build_bin(program, out_path)
            .map(|o| format!("M68k -> {}", o.bin_path.display())),
        CompileTarget::HuC6280 => HuC6280Builder::build_bin(program, out_path)
            .map(|o| format!("HuC6280 -> {}", o.bin_path.display())),
        CompileTarget::Pdp8 => Pdp8Builder::build_bin(program, out_path)
            .map(|o| format!("PDP-8 -> {}", o.bin_path.display())),
        CompileTarget::Pdp11 => Pdp11Builder::build_bin(program, out_path)
            .map(|o| format!("PDP-11 -> {}", o.bin_path.display())),
        CompileTarget::Vax => VaxBuilder::build_bin(program, out_path)
            .map(|o| format!("VAX -> {}", o.bin_path.display())),
        CompileTarget::Hp3000 => Hp3000Builder::build_bin(program, out_path)
            .map(|o| format!("HP 3000 -> {}", o.bin_path.display())),
        CompileTarget::S360 => S360Builder::build_bin(program, out_path)
            .map(|o| format!("S/360 -> {}", o.bin_path.display())),
        CompileTarget::Zarch => ZArchBuilder::build_bin(program, out_path)
            .map(|o| format!("z/Arch -> {}", o.bin_path.display())),
        CompileTarget::Univac => UnivacBuilder::build_bin(program, out_path)
            .map(|o| format!("UNIVAC -> {}", o.bin_path.display())),
        CompileTarget::Cdc6600 => Cdc6600Builder::build_bin(program, out_path)
            .map(|o| format!("CDC 6600 -> {}", o.bin_path.display())),
        CompileTarget::Vliw => DirectVliwBuilder::build_bin(program, out_path)
            .map(|o| format!("VLIW -> {}", o.bin_path.display())),
        CompileTarget::Elbrus => ElbrusBuilder::build_bin(program, out_path)
            .map(|o| format!("Elbrus -> {}", o.bin_path.display())),
        CompileTarget::Ia64 => DirectIa64Builder::build_bin(program, out_path)
            .map(|o| format!("IA-64 -> {}", o.bin_path.display())),
        CompileTarget::Mil1750a => Mil1750aBuilder::build_bin(program, out_path)
            .map(|o| format!("MIL-1750A -> {}", o.bin_path.display())),
        CompileTarget::Jovial => JovialBuilder::build_bin(program, out_path)
            .map(|o| format!("JOVIAL -> {}", o.bin_path.display())),
        CompileTarget::Ural => UralBuilder::build_bin(program, out_path)
            .map(|o| format!("Ural -> {}", o.bin_path.display())),
        CompileTarget::Besm => BesmBuilder::build_bin(program, out_path)
            .map(|o| format!("BESM -> {}", o.bin_path.display())),
        CompileTarget::Mir => MirBuilder::build_bin(program, out_path)
            .map(|o| format!("Mir -> {}", o.bin_path.display())),
        CompileTarget::Harvard => HarvardBuilder::build_bin(program, out_path)
            .map(|o| format!("Harvard -> {}", o.bin_path.display())),
        CompileTarget::Mill => MillBuilder::build_bin(program, out_path)
            .map(|o| format!("Mill -> {}", o.bin_path.display())),
        CompileTarget::Portable => PortableBuilder::build_bytecode(program, out_path)
            .map(|o| format!("Portable -> {}", o.bytecode_path.display())),
        CompileTarget::Vm => DirectVmBuilder::build_bytecode(program, out_path)
            .map(|o| format!("Stack VM -> {}", o.bytecode_path.display())),
        CompileTarget::Ternary => DirectTernaryBuilder::build_bin(program, out_path)
            .map(|o| format!("Ternary -> {}", o.bin_path.display())),
        CompileTarget::Quantum8 => DirectQuantum8Builder::build_bin(program, out_path)
            .map(|o| format!("Quantum 8 -> {}", o.bin_path.display())),
        CompileTarget::Quantum64 => DirectQuantum64Builder::build_bin(program, out_path)
            .map(|o| format!("Quantum 64 -> {}", o.bin_path.display())),
    }
}

fn normalize_out_path(input: &str, target: CompileTarget) -> PathBuf {
    let mut path = if input.trim().is_empty() {
        PathBuf::from("build/ide_output")
    } else {
        PathBuf::from(input.trim())
    };
    path.set_extension(target.extension());
    path
}

fn parse_error_line(text: &str) -> Option<usize> {
    let prefix = "line ";
    let start = text.find(prefix)? + prefix.len();
    let rest = &text[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<usize>().ok()
}

fn split_simple_args(text: &str) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split_whitespace().map(ToString::to_string).collect()
}

fn load_project_source_from_root(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_dcrt_files_recursive(root, &mut files)?;
    files.sort();
    if let Some(idx) = files.iter().position(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("main.dcrt"))
    }) {
        let first = files.remove(idx);
        files.insert(0, first);
    }
    if files.is_empty() {
        return Err(format!("no .dcrt files found under {}", root.display()));
    }
    let mut source = String::new();
    for file in files {
        let content = fs::read_to_string(&file)
            .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
        source.push_str(&content);
        source.push('\n');
    }
    Ok(source)
}

fn collect_dcrt_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if matches!(name, ".git" | "target" | "build") {
                continue;
            }
            collect_dcrt_files_recursive(&path, out)?;
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dcrt"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_all_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if matches!(name, ".git" | "target" | "build") {
                continue;
            }
            collect_all_files_recursive(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn highlight_decretum(source: &str, error_lines: &[usize]) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (idx, line) in source.split_inclusive('\n').enumerate() {
        let line_num = idx + 1;
        let is_error = error_lines.contains(&line_num);
        highlight_line(line, &mut job, is_error);
    }
    job
}

fn highlight_line(line: &str, job: &mut LayoutJob, is_error: bool) {
    let base = TextFormat {
        font_id: FontId::monospace(14.0),
        color: egui::Color32::from_rgb(210, 210, 225),
        ..Default::default()
    };
    let error_base = TextFormat {
        font_id: FontId::monospace(14.0),
        color: egui::Color32::from_rgb(255, 160, 160),
        background: egui::Color32::from_rgba_premultiplied(100, 20, 20, 80),
        ..Default::default()
    };
    let keyword_fmt = TextFormat {
        color: egui::Color32::from_rgb(196, 150, 255),
        ..base.clone()
    };
    let builtin_fmt = TextFormat {
        color: egui::Color32::from_rgb(120, 190, 255),
        ..base.clone()
    };
    let number_fmt = TextFormat {
        color: egui::Color32::from_rgb(250, 204, 140),
        ..base.clone()
    };
    let string_fmt = TextFormat {
        color: egui::Color32::from_rgb(155, 220, 150),
        ..base.clone()
    };
    let label_fmt = TextFormat {
        color: egui::Color32::from_rgb(255, 170, 110),
        ..base.clone()
    };
    let comment_fmt = TextFormat {
        color: egui::Color32::from_rgb(110, 118, 145),
        ..base.clone()
    };

    let (code, comment) = split_comment(line);
    let mut i = 0usize;
    while i < code.len() {
        let ch = code.as_bytes()[i] as char;
        if ch.is_ascii_whitespace() {
            let start = i;
            i += 1;
            while i < code.len() && (code.as_bytes()[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            job.append(&code[start..i], 0.0, if is_error { error_base.clone() } else { base.clone() });
            continue;
        }
        if ch == '"' {
            let start = i;
            i += 1;
            let mut escape = false;
            while i < code.len() {
                let c = code.as_bytes()[i] as char;
                i += 1;
                if escape {
                    escape = false;
                    continue;
                }
                if c == '\\' {
                    escape = true;
                    continue;
                }
                if c == '"' {
                    break;
                }
            }
            job.append(&code[start..i], 0.0, if is_error { error_base.clone() } else { string_fmt.clone() });
            continue;
        }

        let start = i;
        i += 1;
        while i < code.len() {
            let c = code.as_bytes()[i] as char;
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '[' || c == ']' {
                i += 1;
            } else {
                break;
            }
        }
        let token = &code[start..i];
        let lower = token.to_ascii_lowercase();
        let token_fmt = if is_error {
            error_base.clone()
        } else if token.ends_with(':') || lower.starts_with('.') {
            label_fmt.clone()
        } else if lower.starts_with("builtin.") {
            builtin_fmt.clone()
        } else if is_decretum_keyword(&lower) {
            keyword_fmt.clone()
        } else if is_register(&lower) {
            builtin_fmt.clone()
        } else if is_numeric_literal(&lower) {
            number_fmt.clone()
        } else {
            base.clone()
        };
        job.append(token, 0.0, token_fmt);
    }

    if !comment.is_empty() {
        job.append(comment, 0.0, comment_fmt);
    }
}

fn split_comment(line: &str) -> (&str, &str) {
    let mut in_str = false;
    let mut escape = false;
    for (idx, ch) in line.char_indices() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        if ch == '"' {
            in_str = true;
            continue;
        }
        if ch == ';' {
            return (&line[..idx], &line[idx..]);
        }
    }
    (line, "")
}

fn is_numeric_literal(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if token.starts_with("0x") {
        return token[2..].chars().all(|c| c.is_ascii_hexdigit());
    }
    if token.starts_with("-0x") {
        return token[3..].chars().all(|c| c.is_ascii_hexdigit());
    }
    token
        .chars()
        .enumerate()
        .all(|(idx, c)| c.is_ascii_digit() || (idx == 0 && c == '-'))
}

fn is_register(token: &str) -> bool {
    matches!(
        token,
        "r0" | "r1"
            | "r2"
            | "r3"
            | "r4"
            | "r5"
            | "r6"
            | "r7"
            | "r8"
            | "r9"
            | "r10"
            | "r11"
            | "r12"
            | "r13"
            | "r14"
            | "r15"
            | "rax"
            | "rbx"
            | "rcx"
            | "rdx"
            | "rsi"
            | "rdi"
            | "rbp"
            | "rsp"
            | "ax"
            | "bx"
            | "cx"
            | "dx"
            | "si"
            | "di"
            | "sp"
            | "bp"
            | "al"
            | "ah"
            | "bl"
            | "bh"
            | "cl"
            | "ch"
            | "dl"
            | "dh"
            | "es"
            | "cs"
            | "ss"
            | "ds"
    )
}

fn is_decretum_keyword(token: &str) -> bool {
    matches!(
        token,
        "target"
            | "entry"
            | "data"
            | "buffer"
            | "byte"
            | "word"
            | "dword"
            | "qword"
            | "event"
            | "proc"
            | "emit"
            | "call"
            | "ret"
            | "nop"
            | "mov"
            | "add"
            | "sub"
            | "mul"
            | "imul"
            | "div"
            | "mod"
            | "and"
            | "or"
            | "xor"
            | "shl"
            | "shr"
            | "not"
            | "inc"
            | "dec"
            | "cmp"
            | "jmp"
            | "je"
            | "jz"
            | "jne"
            | "jnz"
            | "jl"
            | "jle"
            | "jg"
            | "jge"
            | "jb"
            | "jbe"
            | "ja"
            | "jae"
            | "print"
            | "println"
            | "print_data"
            | "print_u64"
            | "print_var"
            | "wait_input"
            | "exit"
            | "input"
            | "input_str"
            | "sleep_ms"
            | "read_file"
            | "write_file"
            | "get_char"
            | "set_char"
            | "str_len"
            | "str_alloc"
            | "compile_decretum"
            | "builtin.print"
            | "builtin.println"
            | "builtin.newline"
            | "builtin.clear_screen"
            | "builtin.wait_key"
            | "builtin.get_key"
            | "builtin.beep"
            | "builtin.disk_reset"
            | "builtin.reboot"
            | "builtin.halt"
            | "builtin.print_u16"
            | "builtin.print_hex16"
            | "builtin.print_char"
            | "builtin.panic"
            | "builtin.set_text_attr"
            | "builtin.set_cursor"
            | "builtin.set_video_mode"
            | "builtin.get_cursor"
            | "builtin.get_mem_kb"
            | "builtin.memcpy"
            | "builtin.memset"
    )
}


// Returns the byte offset into `code` for the start of the given line index (0-based).
fn line_idx_to_byte(code: &str, line_idx: usize) -> usize {
    let mut pos = 0;
    for _ in 0..line_idx {
        if let Some(nl) = code[pos..].find('\n') {
            pos = pos + nl + 1;
        } else {
            break;
        }
    }
    pos.min(code.len())
}

// Returns the leading whitespace of the line at the given 0-based index.
fn leading_whitespace_of_line(code: &str, line_idx: usize) -> &str {
    let start = line_idx_to_byte(code, line_idx);
    let rest = &code[start..];
    let end = rest.find('\n').unwrap_or(rest.len());
    let line = &rest[..end];
    let ws_end = line.find(|c: char| !c.is_whitespace()).unwrap_or(line.len());
    &line[..ws_end]
}

// Find matching bracket in chars. Returns Some(byte_offset) for the matching bracket.
fn find_matching_bracket(chars: &[char], idx: usize, c: char) -> Option<usize> {
    let (open, close, _dir) = match c {
        '(' => ('(', ')', 1),
        ')' => ('(', ')', -1),
        '{' => ('{', '}', 1),
        '}' => ('{', '}', -1),
        '[' => ('[', ']', 1),
        ']' => ('[', ']', -1),
        _ => return None,
    };
    let dir = if open == c { 1 } else { -1 };
    let mut depth = 0i32;
    let mut i = if dir == 1 { idx + 1 } else { idx.wrapping_sub(1) };
    while i < chars.len() {
        if chars[i] == open { depth += 1; }
        if chars[i] == close { depth -= 1; }
        if depth == 0 {
            // Count bytes up to this character
            let byte_offset = chars[..i].iter().map(|ch| ch.len_utf8()).sum();
            return Some(byte_offset);
        }
        if dir == 1 { i += 1; } else if i == 0 { break; } else { i -= 1; }
    }
    None
}

// Add bracket pair highlight regions to a `LayoutJob`.
fn add_bracket_highlight(mut job: egui::text::LayoutJob, pairs: &[(usize, usize)], text: &str) -> egui::text::LayoutJob {
    for &(a, b) in pairs {
        for offset in [a, b] {
            if offset >= text.len() { continue; }
            let c = text[offset..].chars().next().unwrap_or(' ');
            let len = c.len_utf8();
            // Apply highlight to matching sections by adding a custom overlay color
            for section in &mut job.sections {
                let range = &section.byte_range;
                let sec_begin = range.start as usize;
                let sec_end = range.end as usize;
                if offset >= sec_begin && offset < sec_end {
                    if len >= (section.byte_range.end - section.byte_range.start) {
                        section.format.color = egui::Color32::from_rgb(255, 200, 100);
                        section.format.background = egui::Color32::from_rgb(60, 50, 20);
                    }
                    break;
                }
            }
        }
    }
    job
}

