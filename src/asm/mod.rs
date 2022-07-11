use crate::{Command, PietCode};
use num_bigint::BigInt;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

mod generator;
mod optimizer;
mod parser;
mod preprocessor;

pub type LabelId = usize;

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
    Label(LabelId),
    Jump(LabelId),
    JumpIf(LabelId),
    Stop,
}

impl TryFrom<AsmCommand> for Command {
    type Error = ();

    fn try_from(cmd: AsmCommand) -> Result<Self, ()> {
        Ok(match cmd {
            AsmCommand::Push(_) => Command::Push,
            AsmCommand::Pop => Command::Pop,
            AsmCommand::Add => Command::Add,
            AsmCommand::Subtract => Command::Subtract,
            AsmCommand::Multiply => Command::Multiply,
            AsmCommand::Divide => Command::Divide,
            AsmCommand::Mod => Command::Mod,
            AsmCommand::Not => Command::Not,
            AsmCommand::Greater => Command::Greater,
            AsmCommand::Duplicate => Command::Duplicate,
            AsmCommand::Roll => Command::Roll,
            AsmCommand::InNum => Command::InNum,
            AsmCommand::InChar => Command::InChar,
            AsmCommand::OutNum => Command::OutNum,
            AsmCommand::OutChar => Command::OutChar,
            _ => { return Err(()); }
        })
    }
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

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error at {}: {}", self.lineno, self.error_type)
    }
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

impl fmt::Display for ParseErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseErrorType::*;

        match self {
            EmptyIdentifier => write!(f, "empty identifier"),
            InvalidIdentifierFormat(id) => write!(f, "invalid identifier '{id}'"),
            UnrecognizedCommand(cmd) => write!(f, "unrecognized command '{cmd}'"),
            WrongArgumentCount(count, min, None) => {
                write!(f, "expected at least {min} arguments, but found {count}")
            }
            WrongArgumentCount(count, min, Some(max)) => {
                write!(f, "expected between {min} and {max} arguments, but found {count}")
            }
            ExpectedInteger(code) => write!(f, "invalid integer literal '{code}'"),
            MissingLabel(label) => write!(f, "missing label '{label}'"),
            DuplicateLabel(label) => write!(f, "duplicate label '{label}'"),
            UnboundVarError(var) => write!(f, "unbound var '{var}'"),
            InvalidPragma(line) => write!(f, "invalid pragma: '{line}'"),
            MissingEnd => write!(f, "unclosed delimiter"),
            ExtraEnd => write!(f, "unexpected closing delimiter"),
            TypeError => write!(f, "type error"),
        }
    }
}

fn parse(lines: &[String]) -> Result<PietCode, ParseError> {
    let ast = preprocessor::preprocess(lines)?;
    let asm = parser::to_bytecode(ast)?;
    let asm = optimizer::optimize(asm);
    let asm = optimizer::sanitize(asm);
    let img = generator::generate(asm);
    Ok(img)
}

pub fn load(filename: &str) -> Result<PietCode, String> {
    let file = File::open(filename).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Result<Vec<_>, _> = reader.lines().collect();
    let lines = lines.map_err(|e| e.to_string())?;
    parse(&lines).map_err(|e| e.to_string())
}
