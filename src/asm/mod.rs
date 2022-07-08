use num_bigint::BigInt;
use std::fs::File;
use std::io::{BufRead, BufReader};

mod optimizer;
mod parser;
mod preprocessor;

#[derive(Debug, PartialEq, Eq, Clone)]
enum AsmCommand {
    Push(BigInt),
    Pop,
    Add,
    Subtract,
    Multiply,
    Divide,
    Mod,
    Not,
    Greater,
    // Pointer,
    // Switch,
    Duplicate,
    Roll,
    InNum,
    InChar,
    OutNum,
    OutChar,
    // --
    Label(String),
    Jump(String),
    JumpIf(String),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PietAsm {
    cmds: Vec<AsmCommand>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ParseError {
    lineno: usize,
    error_type: ParseErrorType,
}

#[derive(Debug)]
enum ParseErrorType {
    EmptyIdentifier,
    InvalidIdentifierFormat(String),
    UnrecognizedCommand(String),
    WrongArgumentCount(usize, usize, Option<usize>),
    ExpectedInteger(String),
    MissingLabel(String),
    DuplicateLabel(String),
    UnboundVarError(String),
    InvalidPragma(String),
    MissingEnd,
    ExtraEnd,
    TypeError, // TODO: any metadata.
}

impl ParseErrorType {
    fn at(self, lineno: usize) -> ParseError {
        ParseError {
            lineno,
            error_type: self,
        }
    }
}

fn parse(lines: &[String]) -> Result<PietAsm, ParseError> {
    let ast = preprocessor::preprocess(lines)?;
    let asm = parser::to_bytecode(ast)?;
    let asm = optimizer::optimize(asm);
    Ok(asm)
}

pub fn load(filename: &str) -> Result<PietAsm, String> {
    let file = File::open(filename).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Result<Vec<_>, _> = reader.lines().collect();
    let lines = lines.map_err(|e| e.to_string())?;
    parse(&lines).map_err(|e| format!("{e:?}"))
}
