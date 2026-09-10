use decretum::{BytecodeRuntime, DirectBiosBuilder, Parser, PortableBuilder};
use std::fs;

const PORTABLE_SAMPLE: &str = r#"
target portable
entry boot

qword a = 21
qword b = 13
qword result = 0

event boot:
    mov [result], [a]
    add [result], [b]
    mul [result], 8
    exit 0
"#;

#[test]
fn parser_accepts_portable_program() {
    let program = Parser::parse(PORTABLE_SAMPLE).expect("parse");
    assert_eq!(program.target, "portable");
    assert_eq!(program.entry_event, "boot");
    assert_eq!(program.blocks.len(), 1);
}

#[test]
fn portable_bytecode_runtime_executes() {
    let program = Parser::parse(PORTABLE_SAMPLE).expect("parse");
    let bytes = PortableBuilder::compile_to_bytes(&program).expect("compile bytecode");
    let mut runtime = BytecodeRuntime::from_bytes(&bytes).expect("decode bytecode");
    let code = runtime.run_entry().expect("run");
    assert_eq!(code, 0);
    assert_eq!(runtime.memory_value("result"), Some(272));
}

#[test]
fn portable_bytecode_file_builds() {
    let program = Parser::parse(PORTABLE_SAMPLE).expect("parse");
    let out = std::env::temp_dir().join("decretum_test_program.dcb");
    let output = PortableBuilder::build_bytecode(&program, &out).expect("build bytecode");
    let bytes = fs::read(&output.bytecode_path).expect("read bytecode");
    assert!(!bytes.is_empty());
    let _ = fs::remove_file(output.bytecode_path);
}

#[test]
fn direct_bios_boot_image_builds() {
    let source = r#"
target bios16
entry boot

data banner = "decretum boot\0"

event boot:
    builtin.clear_screen
    builtin.print banner
    builtin.newline
    cli
.hang:
    hlt
    jmp .hang
"#;

    let program = Parser::parse(source).expect("parse");
    let tmp = std::env::temp_dir().join("decretum_test_boot.img");
    let output = DirectBiosBuilder::build_boot_image(&program, &tmp).expect("build boot image");
    let image = fs::read(&output.image_path).expect("read boot image");
    let kernel = fs::read(&output.kernel_path).expect("read kernel payload");

    assert!(image.len() >= 1024);
    assert_eq!(image[510], 0x55);
    assert_eq!(image[511], 0xAA);
    assert!(!kernel.is_empty());
    assert!(output.sectors_loaded >= 1);

    let _ = fs::remove_file(output.image_path);
    let _ = fs::remove_file(output.kernel_path);
}

#[test]
fn direct_bios_extended_ops_build() {
    let source = r#"
target bios16
entry boot

data seed = "core\0"
word mem_kb = 0
buffer copy 16

event boot:
    builtin.get_mem_kb
    mov [mem_kb], ax
    builtin.memset copy, 0, 16
    builtin.memcpy copy, seed, 5
    test ax, ax
    jz .done
    or ax, bx
    and ax, bx
    add ax, bx
    sub ax, bx
.done:
    cli
    hlt
"#;

    let program = Parser::parse(source).expect("parse");
    let tmp = std::env::temp_dir().join("decretum_test_extended_boot.img");
    let output = DirectBiosBuilder::build_boot_image(&program, &tmp).expect("build boot image");
    let image = fs::read(&output.image_path).expect("read boot image");
    assert!(image.len() >= 1024);
    let _ = fs::remove_file(output.image_path);
    let _ = fs::remove_file(output.kernel_path);
}
