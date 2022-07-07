use num_bigint::BigInt;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[allow(dead_code)]
#[derive(Debug)]
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
    Jump(String),
    JumpIf(String),
}

#[derive(Debug)]
pub struct PietAsm {
    cmds: Vec<AsmCommand>
}

fn parse_identifier(s: &str) -> Result<&str, ParseErrorType> {
    let mut chars = s.chars();
    let leader = chars.next().ok_or(ParseErrorType::EmptyIdentifier)?;
    if !matches!(leader, 'a'..='z' | 'A'..='Z' | '_') {
        return Err(ParseErrorType::InvalidIdentifierFormat(s.to_string()));
    }
    if !chars.all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9')) {
        return Err(ParseErrorType::InvalidIdentifierFormat(s.to_string()));
    }
    Ok(s)
}

fn parse_integer(s: &str) -> Result<BigInt, ParseErrorType> {
    s.parse().map_err(|_| { ParseErrorType::ExpectedInteger(s.to_string()) })
}

#[derive(Debug)]
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
}

impl ParseErrorType {
    fn at(self, lineno: usize) -> ParseError {
        ParseError {
            lineno,
            error_type: self,
        }
    }
}

fn validate_arg_count(count: usize, min: usize, max: Option<usize>) -> Result<(), ParseErrorType> {
    if min <= count && max.map_or(true, |mx| count <= mx) {
        return Ok(());
    }
    Err(ParseErrorType::WrongArgumentCount(count, min, max))
}

fn parse(lines: &[String]) -> Result<PietAsm, ParseError> {
    let mut labels = HashMap::new();
    let mut missing_labels = HashMap::new();
    let mut cmds = Vec::new();
    for (lineno, line) in lines.iter().enumerate() {
        let lineno = lineno + 1;
        let line = line.trim().split('#').next().unwrap();
        if line.is_empty() { continue; }
        if line.starts_with(':') {
            let label = parse_identifier(&line[1..])
                .map_err(|e| e.at(lineno))?;
            labels.insert(label, lineno);
            missing_labels.remove(label);
            continue;
        }
        let mut terms = line.split_ascii_whitespace();
        let cmd = terms.next().unwrap();
        match cmd {
            "PUSH" => {
                let args: Result<Vec<_>, _> = terms.map(parse_integer).collect();
                let args = args.map_err(|e| e.at(lineno))?;
                validate_arg_count(args.len(), 1, None).map_err(|e| e.at(lineno))?;
                for arg in args {
                    cmds.push(AsmCommand::Push(arg));
                }
            }
            "POP" | "DUP" | "INNUM" | "INCHAR" => {
                let args: Result<Vec<_>, _> = terms.map(parse_integer).collect();
                let args = args.map_err(|e| e.at(lineno))?;
                validate_arg_count(args.len(), 0, Some(0)).map_err(|e| e.at(lineno))?;
                cmds.push(match cmd {
                    "POP" => AsmCommand::Pop,
                    "DUP" => AsmCommand::Duplicate,
                    "INNUM" => AsmCommand::InNum,
                    "INCHAR" => AsmCommand::InChar,
                    _ => unreachable!(),
                });
            }
            "NOT" | "OUTNUM" | "OUTCHAR" => {
                let args: Result<Vec<_>, _> = terms.map(parse_integer).collect();
                let args = args.map_err(|e| e.at(lineno))?;
                validate_arg_count(args.len(), 0, Some(1)).map_err(|e| e.at(lineno))?;
                for arg in args {
                    cmds.push(AsmCommand::Push(arg));
                }
                cmds.push(match cmd {
                    "NOT" => AsmCommand::Not,
                    "OUTNUM" => AsmCommand::OutNum,
                    "OUTCHAR" => AsmCommand::OutChar,
                    _ => unreachable!(),
                });
            }
            "ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "GREATER" | "ROLL" => {
                let args: Result<Vec<_>, _> = terms.map(parse_integer).collect();
                let args = args.map_err(|e| e.at(lineno))?;
                validate_arg_count(args.len(), 0, Some(2)).map_err(|e| e.at(lineno))?;
                for arg in args {
                    cmds.push(AsmCommand::Push(arg));
                }
                cmds.push(match cmd {
                    "ADD" => AsmCommand::Add,
                    "SUB" => AsmCommand::Subtract,
                    "MUL" => AsmCommand::Multiply,
                    "DIV" => AsmCommand::Divide,
                    "MOD" => AsmCommand::Mod,
                    "GREATER" => AsmCommand::Greater,
                    "ROLL" => AsmCommand::Roll,
                    _ => unreachable!(),
                });
            }
            "JUMP" | "JUMPIF" => {
                let args: Result<Vec<_>, _> = terms.map(parse_identifier).collect();
                let args = args.map_err(|e| e.at(lineno))?;
                validate_arg_count(args.len(), 1, Some(1)).map_err(|e| e.at(lineno))?;
                let label = args[0];
                if !labels.contains_key(label) {
                    missing_labels.entry(label).or_insert(lineno);
                }
            }
            // "POINTER" => (),
            // "SWITCH" => (),
            cmd => {
                let cmd = cmd.to_string();
                return Err(ParseErrorType::UnrecognizedCommand(cmd).at(lineno));
            }
        }
    }
    if let Some((label, lineno)) = missing_labels.into_iter().next() {
        // TODO: only grabs one here, not great.
        return Err(ParseErrorType::MissingLabel(label.to_string()).at(lineno));
    }
    Ok(PietAsm { cmds })
}

pub fn load(filename: &str) -> Result<PietAsm, String> {
    let file = File::open(filename).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Result<Vec<_>, _> = reader.lines().collect();
    let lines = lines.map_err(|e| e.to_string())?;
    parse(&lines).map_err(|e| format!("{e:?}"))
}
