use std::fs;
use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};
use decretum::{
    DirectAarch64Builder, DirectArmCmBuilder, DirectBiosBuilder, DirectCheriBuilder,
    DirectElf32Builder, DirectElfBuilder, DirectMachoBuilder, DirectRisCvCheriBuilder,
    DirectRiscvBuilder, DirectUefiBuilder, DirectVmBuilder, DirectWin32Builder, DirectX86_64Builder,
    NativeStackBuilder, Parser, PortableBuilder, TargetRegistry,
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
            let target = TargetRegistry::get(&program.target).ok_or_else(|| {
                format!(
                    "compile-arch doesn't support target '{}'",
                    program.target
                )
            })?;
            let path = target.build(&program, &out_file)?;
            println!("wrote {}", path.display());
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
