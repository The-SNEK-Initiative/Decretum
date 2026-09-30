# Decretum compiler architecture

## Why a target abstraction exists

Target selection used to live in scattered string matches (`match program.target.as_str()`
in the CLI, one arm per backend). Adding a target meant editing dispatch code, and nothing
enumerated the supported set in one place. The `Target` trait plus `TargetRegistry` fix
only that: selection is centralized behind `TargetRegistry::get(name)`, so callers map a
target identifier to a backend entry point without knowing any backend.

## How registry dispatch works

Each backend family ships a `target.rs` next to its lowering code. It defines one unit
struct per target (e.g. `X86_64Target`) implementing the `Target` trait: canonical `name`,
`output_kind` metadata (`Binary`, `Bytecode`, `DiskImage`, `Executable`), and a `build`
method that delegates to the existing builder (`DirectX86_64Builder::build_bin`, etc.)
and returns the primary written path. `backend::registry` holds one static table of
`(name, &dyn Target)` pairs, including aliases that share an implementation (`riscv64`
reuses the RISC-V builder, `sh4` reuses the SH-2 builder). Lookup is a linear scan over
a static slice: no dynamic plugins, no runtime loading, no external modules.

## Why backends remain independent

The trait exposes selection metadata only. It says nothing about CPUs, instructions, or
encoding, and there is no shared lowering pipeline: each `build` call enters a backend
that parses its own instruction text, selects its own instructions, and emits its own
machine code exactly as before. Per-backend `program.target` guards stay in place as
validation. Registry-to-wrapper references are the only new coupling, and they point
one way (selection layer toward backends); backends never call the registry, formats,
or each other, except for the two pre-existing deliberate reuses (ELF/x86-64 via the
UEFI assembler, ELF32/Win32 via the i386 encoder).

## Why no universal IR exists

Decretum lowers directly from the frontend `Program` (target, entry event, data
declarations, blocks of raw instruction lines) to target machine code. The seventy-plus
targets span 4-bit micros, mainframes, DSPs, ternary machines, and quantum circuit
emitters; a common instruction model would either collapse to meaninglessness or force
every backend through lossy translation. The portable bytecode `Op` enum is the closest
thing to an IR and stays private to the portable backend on purpose. The registry
preserves this philosophy: it centralizes *which* backend runs, never *how* it compiles.
