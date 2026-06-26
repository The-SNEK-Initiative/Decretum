use std::fs;
use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};
use decretum::{
    Direct6502Builder,
    Direct6809Builder,
    DirectIa64Builder,
    DirectVliwBuilder,
    S360Builder,
    ZArchBuilder,
    UnivacBuilder,
    Cdc6600Builder,
    Pdp8Builder,
    Pdp11Builder,
    VaxBuilder,
    Hp3000Builder,
    I4004Builder,
    I8008Builder,
    I8080Builder,
    I8086Builder,
    M6800Builder,
    Mos6501Builder,
    Tms320Builder,
    BlackfinBuilder,
    SharcBuilder,
    C166Builder,
    Xc800Builder,
    Rl78Builder,
    RxBuilder,
    H8Builder,
    Msp430Builder,
    NecV20Builder,
    Nec78kBuilder,
    M16cBuilder,
    R8cBuilder,
    FrBuilder,
    Mico32Builder,
    PicoblazeBuilder,
    MmixBuilder,
    DlxBuilder,
    Lc3Builder,
    HuC6280Builder,
    V810Builder,
    Arm7tdmiBuilder,
    Arm9Builder,
    Ppc740Builder,
    Ppc970Builder,
    Mil1750aBuilder,
    JovialBuilder,
    UralBuilder,
    BesmBuilder,
    ElbrusBuilder,
    MirBuilder,
    HarvardBuilder,
    MillBuilder,
    DirectAarch64Builder,
    DirectAlphaBuilder,
    DirectArmCmBuilder,
    DirectAvrBuilder,
    DirectBiosBuilder,
    DirectCheriBuilder,
    DirectElf32Builder,
    DirectElfBuilder,
    DirectM68kBuilder,
    DirectMachoBuilder,
    DirectMicroblazeBuilder,
    DirectMipsBuilder,
    DirectNios2Builder,
    DirectOpenriscBuilder,
    DirectPICBuilder,
    DirectPariscBuilder,
    DirectPpcBuilder,
    DirectQuantum8Builder,
    DirectQuantum64Builder,
    DirectRisCvCheriBuilder,
    DirectRiscvBuilder,
    DirectSh2Builder,
    DirectSparcBuilder,
    DirectTernaryBuilder,
    DirectUefiBuilder,
    DirectVmBuilder,
    DirectWin32Builder,
    DirectX86_64Builder,
    DirectZ80Builder,
    NativeStackBuilder,
    Parser,
    PortableBuilder,
};

#[derive(ClapParser, Debug)]
#[command(
    name = "decretumc",
    about = "Decretum compiler (portable bytecode + direct BIOS machine backend)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse and validate source.
    Validate {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
    },
    /// Compile to portable Decretum bytecode (.dcb).
    #[command(alias = "compile-bin")]
    CompileBytecode {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.dcb")]
        out_file: PathBuf,
    },
    /// Compile to native Windows PE executable via portable bytecode runtime wrapper.
    CompilePe {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.exe")]
        out_file: PathBuf,
    },
    /// Compile to bootable BIOS disk image (.img) with direct machine-code backend.
    CompileBootimg {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/kernel.img")]
        out_file: PathBuf,
    },
    /// Compile to UEFI application (.efi) with direct x86-64 machine-code backend.
    #[command(name = "compile-uefi")]
    CompileUefi {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/boot.efi")]
        out_file: PathBuf,
    },
    /// Compile to ARM Cortex-M raw binary (.bin) with Thumb machine-code backend.
    #[command(name = "compile-armcm")]
    CompileArmCm {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to RISC-V raw binary (.bin) with RV32I machine-code backend.
    #[command(name = "compile-riscv")]
    CompileRiscV {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to standalone x86-64 raw binary (.bin) with x86-64 machine-code backend.
    #[command(name = "compile-x86-64")]
    CompileX86_64 {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/kernel.bin")]
        out_file: PathBuf,
    },
    /// Compile to RISC-V 64-bit raw binary (.bin) with RV64I machine-code backend.
    #[command(name = "compile-riscv64")]
    CompileRiscV64 {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to AArch64 raw binary (.bin) with ARM64 machine-code backend.
    #[command(name = "compile-aarch64")]
    CompileAarch64 {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to stack-based VM bytecode (.vbc).
    #[command(name = "compile-vm")]
    CompileVm {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.vbc")]
        out_file: PathBuf,
    },
    /// Compile to macOS Mach-O executable via AArch64/x86-64 machine code.
    #[command(name = "compile-macho")]
    CompileMacho {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.macho")]
        out_file: PathBuf,
    },
    /// Compile to Linux ELF64 executable via x86-64 machine code.
    #[command(name = "compile-elf")]
    CompileElf {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.elf")]
        out_file: PathBuf,
    },
    /// Compile to CHERI capability binary (.bin) with Morello-like backend.
    #[command(name = "compile-cheri")]
    CompileCheri {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to RISC-V CHERI capability binary (.bin).
    #[command(name = "compile-riscv-cheri")]
    CompileRisCvCheri {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/firmware.bin")]
        out_file: PathBuf,
    },
    /// Compile to Windows 32-bit PE executable (.exe).
    #[command(name = "compile-win32")]
    CompileWin32 {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.exe")]
        out_file: PathBuf,
    },
    /// Compile to Linux 32-bit ELF executable (.elf).
    #[command(name = "compile-elf32")]
    CompileElf32 {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/program.elf")]
        out_file: PathBuf,
    },
    /// Compile to any architecture (.bin) by detecting target from source.
    #[command(name = "compile-arch")]
    CompileArch {
        #[arg(value_name = "FILES", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "OUT", default_value = "build/out.bin")]
        out_file: PathBuf,
    },
    /// Build the Decretum native stack profile and emit a build manifest.
    BuildNativeStack {
        #[arg(long, value_name = "SRC", default_value = "kernel/native_stack")]
        source_root: PathBuf,
        #[arg(long, value_name = "OUT", default_value = "build/native_stack")]
        out_root: PathBuf,
    },
    /// Compile a Decretum project folder (recursively) using one command.
    CompileProject {
        #[arg(long, value_name = "ROOT", default_value = "pure_decretum_compiler")]
        root: PathBuf,
        #[arg(long, value_name = "OUT", default_value = "build/project.exe")]
        out_file: PathBuf,
        #[arg(long, value_name = "MODE", default_value = "pe")]
        mode: String,
    },
}

fn main() {
    if let Err(error) = run_main() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run_main() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Validate { inputs } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            println!(
                "ok: target={} entry={} data={} blocks={}",
                program.target,
                program.entry_event,
                program.data.len(),
                program.blocks.len()
            );
        }
        Commands::CompileBytecode { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = PortableBuilder::build_bytecode(&program, &out_file)?;
            println!("wrote {}", output.bytecode_path.display());
        }
        Commands::CompilePe { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            if program.target == "bios16" {
                return Err(
                    "compile-pe does not support target bios16; use compile-bootimg for BIOS kernels"
                        .to_string(),
                );
            }
            let output = PortableBuilder::build_pe(&program, &out_file)?;
            println!("wrote {}", output.bytecode_path.display());
            println!("wrote {}", output.pe_path.display());
            println!("wrapper build dir {}", output.project_dir.display());
        }
        Commands::CompileBootimg { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectBiosBuilder::build_boot_image(&program, &out_file)?;
            println!("wrote {}", output.image_path.display());
            println!("wrote {}", output.kernel_path.display());
            println!("boot loader sectors: {}", output.sectors_loaded);
        }
        Commands::CompileUefi { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectUefiBuilder::build_efi(&program, &out_file)?;
            println!("wrote {}", output.efi_path.display());
        }
        Commands::CompileArmCm { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectArmCmBuilder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileRiscV { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectRiscvBuilder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileX86_64 { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectX86_64Builder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileRiscV64 { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectRiscvBuilder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileAarch64 { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectAarch64Builder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileVm { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectVmBuilder::build_bytecode(&program, &out_file)?;
            println!("wrote {}", output.bytecode_path.display());
            println!("ops: {}", output.op_count);
        }
        Commands::CompileMacho { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectMachoBuilder::build_macho(&program, &out_file)?;
            println!("wrote {}", output.macho_path.display());
        }
        Commands::CompileElf { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectElfBuilder::build_elf(&program, &out_file)?;
            println!("wrote {}", output.elf_path.display());
        }
        Commands::CompileCheri { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectCheriBuilder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileRisCvCheri { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectRisCvCheriBuilder::build_bin(&program, &out_file)?;
            println!("wrote {}", output.bin_path.display());
        }
        Commands::CompileWin32 { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectWin32Builder::build_pe(&program, &out_file)?;
            println!("wrote {}", output.pe_path.display());
        }
        Commands::CompileElf32 { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            let output = DirectElf32Builder::build_elf(&program, &out_file)?;
            println!("wrote {}", output.elf_path.display());
        }
        Commands::CompileArch { inputs, out_file } => {
            let source = load_source_from_inputs(&inputs)?;
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            match program.target.as_str() {
                "mips" => {
                    let o = DirectMipsBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "ppc" => {
                    let o = DirectPpcBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "sparc" => {
                    let o = DirectSparcBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "riscv" => {
                    let o = DirectRiscvBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "riscv64" => {
                    let o = DirectRiscvBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "riscv_cheri" => {
                    let o = DirectRisCvCheriBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "ia64" => {
                    let o = DirectIa64Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "vliw" => {
                    let o = DirectVliwBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "s360" => { let o = S360Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "zarch" => { let o = ZArchBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "univac" => { let o = UnivacBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "cdc6600" => { let o = Cdc6600Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "pdp8" => { let o = Pdp8Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "pdp11" => { let o = Pdp11Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "vax" => { let o = VaxBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "hp3000" => { let o = Hp3000Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "i4004" => { let o = I4004Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "i8008" => { let o = I8008Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "i8080" => { let o = I8080Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "i8086" => { let o = I8086Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "m6800" => { let o = M6800Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mos6501" => { let o = Mos6501Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "tms320" => { let o = Tms320Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "blackfin" => { let o = BlackfinBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "sharc" => { let o = SharcBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "c166" => { let o = C166Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "xc800" => { let o = Xc800Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "rl78" => { let o = Rl78Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "rx" => { let o = RxBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "h8" => { let o = H8Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "msp430" => { let o = Msp430Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "v20" => { let o = NecV20Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "nec78k" => { let o = Nec78kBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "m16c" => { let o = M16cBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "r8c" => { let o = R8cBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "fr" => { let o = FrBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mico32" => { let o = Mico32Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "picoblaze" => { let o = PicoblazeBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mmix" => { let o = MmixBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "dlx" => { let o = DlxBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "lc3" => { let o = Lc3Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "huc6280" => { let o = HuC6280Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "v810" => { let o = V810Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "arm9" => { let o = Arm9Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "armcm" => { let o = DirectArmCmBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "arm7tdmi" => { let o = Arm7tdmiBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "ppc740" => { let o = Ppc740Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "ppc970" => { let o = Ppc970Builder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mil1750a" => { let o = Mil1750aBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "jovial" => { let o = JovialBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "ural" => { let o = UralBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "besm" => { let o = BesmBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "elbrus" => { let o = ElbrusBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mir" => { let o = MirBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "harvard" => { let o = HarvardBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "mill" => { let o = MillBuilder::build_bin(&program, &out_file)?; println!("wrote {}", o.bin_path.display()); }
                "alpha" => {
                    let o = DirectAlphaBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "parisc" => {
                    let o = DirectPariscBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "openrisc" => {
                    let o = DirectOpenriscBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "nios2" => {
                    let o = DirectNios2Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "microblaze" => {
                    let o = DirectMicroblazeBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "6502" => {
                    let o = Direct6502Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "z80" => {
                    let o = DirectZ80Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "6809" => {
                    let o = Direct6809Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "pic" => {
                    let o = DirectPICBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "avr" => {
                    let o = DirectAvrBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "sh2" | "sh4" => {
                    let o = DirectSh2Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "m68k" => {
                    let o = DirectM68kBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "ternary" => {
                    let o = DirectTernaryBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "quantum8" => {
                    let o = DirectQuantum8Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                "quantum64" => {
                    let o = DirectQuantum64Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", o.bin_path.display());
                }
                _ => {
                    return Err(format!(
                        "compile-arch doesn't support target '{}'",
                        program.target
                    ));
                }
            }
        }
        Commands::BuildNativeStack {
            source_root,
            out_root,
        } => {
            let output = NativeStackBuilder::build(&source_root, &out_root)?;
            println!("wrote manifest {}", output.manifest_path.display());
            println!("modules: {}", output.modules.len());
            if let Some(image) = output.boot_image_path {
                println!("boot image {}", image.display());
            }
            if let Some(kernel) = output.boot_kernel_path {
                println!("boot kernel {}", kernel.display());
            }
        }
        Commands::CompileProject {
            root,
            out_file,
            mode,
        } => {
            let files = collect_project_sources(&root)?;
            if files.is_empty() {
                return Err(format!(
                    "no .dcrt files found in project root {}",
                    root.display()
                ));
            }
            let mut source = String::new();
            for file in &files {
                let content = fs::read_to_string(file)
                    .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
                source.push_str(&content);
                source.push('\n');
            }
            let program = Parser::parse(&source).map_err(|e| e.to_string())?;
            match mode.to_ascii_lowercase().as_str() {
                "pe" => {
                    let output = PortableBuilder::build_pe(&program, &out_file)?;
                    println!("wrote {}", output.bytecode_path.display());
                    println!("wrote {}", output.pe_path.display());
                }
                "bytecode" | "dcb" => {
                    let output = PortableBuilder::build_bytecode(&program, &out_file)?;
                    println!("wrote {}", output.bytecode_path.display());
                }
                "bootimg" | "bios16" => {
                    let output = DirectBiosBuilder::build_boot_image(&program, &out_file)?;
                    println!("wrote {}", output.image_path.display());
                    println!("wrote {}", output.kernel_path.display());
                }
                "uefi" => {
                    let output = DirectUefiBuilder::build_efi(&program, &out_file)?;
                    println!("wrote {}", output.efi_path.display());
                }
                "armcm" => {
                    let output = DirectArmCmBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "riscv" => {
                    let output = DirectRiscvBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "x86_64" => {
                    let output = DirectX86_64Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "riscv64" => {
                    let output = DirectRiscvBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "aarch64" => {
                    let output = DirectAarch64Builder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "vm" => {
                    let output = DirectVmBuilder::build_bytecode(&program, &out_file)?;
                    println!("wrote {}", output.bytecode_path.display());
                }
                "macho" => {
                    let output = DirectMachoBuilder::build_macho(&program, &out_file)?;
                    println!("wrote {}", output.macho_path.display());
                }
                "elf" | "elf64" => {
                    let output = DirectElfBuilder::build_elf(&program, &out_file)?;
                    println!("wrote {}", output.elf_path.display());
                }
                "cheri" => {
                    let output = DirectCheriBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "riscv_cheri" => {
                    let output = DirectRisCvCheriBuilder::build_bin(&program, &out_file)?;
                    println!("wrote {}", output.bin_path.display());
                }
                "win32" => {
                    let output = DirectWin32Builder::build_pe(&program, &out_file)?;
                    println!("wrote {}", output.pe_path.display());
                }
                "elf32" => {
                    let output = DirectElf32Builder::build_elf(&program, &out_file)?;
                    println!("wrote {}", output.elf_path.display());
                }
                other => {
                    return Err(format!(
                        "unsupported compile-project mode '{other}' (use pe|bytecode|bootimg|uefi|armcm|riscv|x86_64|riscv64|aarch64|vm|macho|elf|cheri|riscv_cheri|win32|elf32)"
                    ));
                }
            }
            println!("project root {}", root.display());
            println!("source files {}", files.len());
        }
    }

    Ok(())
}

fn collect_project_sources(root: &PathBuf) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Err(format!("project root does not exist: {}", root.display()));
    }
    let mut files = Vec::<PathBuf>::new();
    collect_project_sources_recursive(root, &mut files)?;
    files.sort();

    // Place main.dcrt first when present so target/entry are seen early.
    if let Some(idx) = files.iter().position(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("main.dcrt"))
            .unwrap_or(false)
    }) {
        let main = files.remove(idx);
        files.insert(0, main);
    }
    Ok(files)
}

fn collect_project_sources_recursive(dir: &PathBuf, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_project_sources_recursive(&path, out)?;
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("dcrt"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn load_source_from_inputs(inputs: &[PathBuf]) -> Result<String, String> {
    let files = collect_source_files(inputs)?;
    if files.is_empty() {
        return Err("no .dcrt files found in provided inputs".to_string());
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

fn collect_source_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for input in inputs {
        if input.is_file() {
            if is_dcrt_file(input) {
                out.push(input.clone());
            }
            continue;
        }
        if input.is_dir() {
            collect_source_files_from_dir(input, &mut out)?;
            continue;
        }
        return Err(format!("input path does not exist: {}", input.display()));
    }
    out.sort();
    Ok(out)
}

fn collect_source_files_from_dir(dir: &PathBuf, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_files_from_dir(&path, out)?;
        } else if path.is_file() && is_dcrt_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_dcrt_file(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("dcrt"))
        .unwrap_or(false)
}

