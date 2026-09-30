use std::fs;
use std::path::{Path, PathBuf};

use crate::{DirectBiosBuilder, Parser, PortableBuilder};

#[derive(Debug, Clone)]
pub struct NativeStackModule {
    pub name: String,
    pub source_path: PathBuf,
    pub target: String,
    pub bytecode_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct NativeStackBuildOutput {
    pub manifest_path: PathBuf,
    pub modules: Vec<NativeStackModule>,
    pub boot_image_path: Option<PathBuf>,
    pub boot_kernel_path: Option<PathBuf>,
}

pub struct NativeStackBuilder;

impl NativeStackBuilder {
    pub fn build(source_root: &Path, out_root: &Path) -> Result<NativeStackBuildOutput, String> {
        if !source_root.exists() {
            return Err(format!(
                "native stack source root does not exist: {}",
                source_root.display()
            ));
        }

        fs::create_dir_all(out_root)
            .map_err(|e| format!("failed to create {}: {e}", out_root.display()))?;

        let mut modules = Vec::new();
        for group in ["handoff", "services", "runtime", "toolchain"] {
            let dir = source_root.join(group);
            if !dir.exists() {
                continue;
            }
            for file in list_dcrt_files(&dir)? {
                let source = fs::read_to_string(&file)
                    .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
                let program = Parser::parse(&source).map_err(|e| e.to_string())?;
                let name = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| format!("invalid module file name: {}", file.display()))?
                    .to_string();

                let module_out_dir = out_root.join(group);
                fs::create_dir_all(&module_out_dir)
                    .map_err(|e| format!("failed to create {}: {e}", module_out_dir.display()))?;

                let bytecode_path = if program.target == "portable" || program.target == "win64" {
                    let bytes = PortableBuilder::compile_to_bytes(&program)?;
                    let out_path = module_out_dir.join(format!("{name}.dcb"));
                    fs::write(&out_path, bytes)
                        .map_err(|e| format!("failed to write {}: {e}", out_path.display()))?;
                    Some(out_path)
                } else {
                    None
                };

                modules.push(NativeStackModule {
                    name,
                    source_path: file,
                    target: program.target,
                    bytecode_path,
                });
            }
        }

        let (boot_image_path, boot_kernel_path) = compile_boot_profile(source_root, out_root)?;
        let manifest_path = out_root.join("native_stack_manifest.json");
        let manifest_text = build_manifest_json(
            &modules,
            boot_image_path.as_ref(),
            boot_kernel_path.as_ref(),
        );
        fs::write(&manifest_path, manifest_text)
            .map_err(|e| format!("failed to write {}: {e}", manifest_path.display()))?;

        Ok(NativeStackBuildOutput {
            manifest_path,
            modules,
            boot_image_path,
            boot_kernel_path,
        })
    }
}

fn compile_boot_profile(
    source_root: &Path,
    out_root: &Path,
) -> Result<(Option<PathBuf>, Option<PathBuf>), String> {
    let boot_source = source_root.join("boot").join("boot_kernel.dcrt");
    if !boot_source.exists() {
        return Ok((None, None));
    }
    let source = fs::read_to_string(&boot_source)
        .map_err(|e| format!("failed to read {}: {e}", boot_source.display()))?;
    let program = Parser::parse(&source).map_err(|e| e.to_string())?;
    if program.target != "bios16" {
        return Err(format!(
            "boot profile must use target bios16, got '{}'",
            program.target
        ));
    }
    let boot_out = out_root.join("boot_kernel.img");
    let output = DirectBiosBuilder::build_boot_image(&program, &boot_out)?;
    Ok((Some(output.image_path), Some(output.kernel_path)))
}

fn list_dcrt_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("dcrt"))
                .unwrap_or(false)
        {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn build_manifest_json(
    modules: &[NativeStackModule],
    boot_image: Option<&PathBuf>,
    boot_kernel: Option<&PathBuf>,
) -> String {
    let mut text = String::new();
    text.push_str("{\n");
    text.push_str("  \"schema\": \"decretum-native-stack-v1\",\n");
    text.push_str("  \"modules\": [\n");
    for (idx, module) in modules.iter().enumerate() {
        if idx > 0 {
            text.push_str(",\n");
        }
        text.push_str("    {\n");
        text.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&module.name)
        ));
        text.push_str(&format!(
            "      \"target\": \"{}\",\n",
            json_escape(&module.target)
        ));
        text.push_str(&format!(
            "      \"source\": \"{}\",\n",
            json_escape(&module.source_path.display().to_string())
        ));
        if let Some(bytecode) = &module.bytecode_path {
            text.push_str(&format!(
                "      \"bytecode\": \"{}\"\n",
                json_escape(&bytecode.display().to_string())
            ));
        } else {
            text.push_str("      \"bytecode\": null\n");
        }
        text.push_str("    }");
    }
    text.push_str("\n  ],\n");
    if let Some(image) = boot_image {
        text.push_str(&format!(
            "  \"boot_image\": \"{}\",\n",
            json_escape(&image.display().to_string())
        ));
    } else {
        text.push_str("  \"boot_image\": null,\n");
    }
    if let Some(kernel) = boot_kernel {
        text.push_str(&format!(
            "  \"boot_kernel\": \"{}\"\n",
            json_escape(&kernel.display().to_string())
        ));
    } else {
        text.push_str("  \"boot_kernel\": null\n");
    }
    text.push_str("}\n");
    text
}
