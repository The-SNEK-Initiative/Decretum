use decretum::BytecodeRuntime;

fn main() {
    if let Err(err) = run() {
        eprintln!("runtime error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("failed to get exe: {e}"))?;
    let data = std::fs::read(&current_exe).map_err(|e| format!("failed to read exe: {e}"))?;
    let magic = decretum::portable::dcrt_embed_magic();
    if data.len() < 20 || data[data.len() - 16..] != magic {
        return Err("no embedded Decretum bytecode payload found".to_string());
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
    let mut runtime = BytecodeRuntime::from_bytes(bytecode)?;
    let code = runtime.run_entry()?;
    std::process::exit(code);
}
