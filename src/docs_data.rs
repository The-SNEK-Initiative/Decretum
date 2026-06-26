pub const DOCS: &[(&str, &str)] = &[
    ("Overview", include_str!("../docs/index.html")),
    (
        "Getting Started",
        include_str!("../docs/getting-started/index.html"),
    ),
    ("Syntax Guide", include_str!("../docs/language/syntax.html")),
    (
        "Instruction Model",
        include_str!("../docs/language/expressions.html"),
    ),
    (
        "Helper Functions",
        include_str!("../docs/language/functions.html"),
    ),
    (
        "Execution Model",
        include_str!("../docs/runtime/execution-model.html"),
    ),
    (
        "Hardware Constraints",
        include_str!("../docs/runtime/constraints.html"),
    ),
    (
        "BIOS Builtins",
        include_str!("../docs/runtime/bios-builtins.html"),
    ),
    (
        "Compiler Backends",
        include_str!("../docs/compiler/backends.html"),
    ),
    (
        "PE Targets",
        include_str!("../docs/compiler/pe-target.html"),
    ),
    (
        "Integrated Environment",
        include_str!("../docs/tools/ide.html"),
    ),
    (
        "Example: Boot Kernel",
        include_str!("../docs/examples/full-kernel.html"),
    ),
    (
        "Example: Compute PE",
        include_str!("../docs/examples/compute-pe.html"),
    ),
    (
        "Example: Fibonacci",
        include_str!("../docs/examples/fibonacci.html"),
    ),
    (
        "Example: Random Numbers",
        include_str!("../docs/examples/random.html"),
    ),
    (
        "Example: String Ops",
        include_str!("../docs/examples/string-ops.html"),
    ),
];
