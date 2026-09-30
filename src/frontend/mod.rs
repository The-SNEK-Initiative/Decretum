pub mod parser;

pub use parser::{
    Block, BlockKind, DataDecl, ParseError, Parser, Program, ScalarWidth, collect_symbols,
};
