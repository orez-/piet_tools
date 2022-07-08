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

fn parse_int_arg(s: &str, lookup: &HashMap<String, BigInt>) -> Result<BigInt, ParseErrorType> {
    if let Some(arg) = s.strip_prefix("@") {
        let arg = parse_identifier(arg)?;
        let num = lookup.get(arg)
            .ok_or_else(|| ParseErrorType::UndefinedVarError(arg.to_string()))?;
        return Ok(num.clone());
    }
    parse_integer(s)
}

#[derive(Clone, Debug)]
enum Token {
    Var(String),
    Num(BigInt),
    Label(String),
}

impl Token {
    fn bind(&mut self, name: &str, value: &BigInt) {
        if let Token::Var(id) = self {
            if *id == name {
                *self = Token::Num(value.clone());
            }
        }
    }
}

impl TryFrom<&str> for Token {
    type Error = ParseErrorType;

    fn try_from(arg: &str) -> Result<Self, ParseErrorType> {
        Ok(match arg.strip_prefix('@') {
            Some(name) => Token::Var(name.to_string()),
            None => {
                match arg.parse() {
                    Ok(int) => Token::Num(int),
                    Err(_) => Token::Label(parse_identifier(arg)?.to_string()),
                }
            }
        })
    }
}

#[derive(Clone, Debug)]
struct Line<'a> {
    lineno: usize,
    stmt: Statement<'a>,
}

impl Line<'_> {
    fn bind(&mut self, name: &str, value: &BigInt) {
        self.stmt.bind(name, value);
    }
}

#[derive(Clone, Debug)]
enum Statement<'a> {
    Cmd {
        cmd: &'a str,
        args: Vec<Token>,
    },
    Label(&'a str),
}

impl Statement<'_> {
    fn bind(&mut self, name: &str, value: &BigInt) {
        if let Statement::Cmd { args, .. } = self {
            for arg in args.iter_mut() {
                arg.bind(name, value);
            }
        }
    }
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
    DuplicateLabel(String),
    UndefinedVarError(String),
    InvalidPragma(String),
    MissingEnd,
    ExtraEnd,
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

fn preprocess(lines: &[String]) -> Result<Vec<Line>, ParseError> {
    let mut lines = lines.iter().enumerate().filter_map(|(lineno, line)| {
        let lineno = lineno + 1;
        let line = line.trim().split('#').next().unwrap();
        (!line.is_empty()).then(|| (lineno, line))
    });
    let mut command_stack = Vec::new();
    let mut commands = Vec::new();
    while let Some((lineno, line)) = lines.next() {
        let pp_token = preprocess_line(line, lineno).map_err(|e| e.at(lineno))?;
        match pp_token {
            PreprocToken::Line(cmd) => { commands.push(cmd); }
            PreprocToken::Each(name, terms) => {
                command_stack.push((name, terms, commands, lineno));
                commands = Vec::new();
            }
            PreprocToken::End => {
                let (name, terms, mut restored_cmds, _) = command_stack.pop()
                    .ok_or(ParseErrorType::ExtraEnd.at(lineno))?;
                for term in terms {
                    let ccmds = commands.clone();
                    for mut cmd in ccmds {
                        cmd.bind(name, &term);
                        restored_cmds.push(cmd);
                    }
                }
                commands = restored_cmds;
            }
        }
    }
    if let Some((_, _, _, lineno)) = command_stack.pop() {
        return Err(ParseErrorType::MissingEnd.at(lineno));
    }
    Ok(commands)
}

enum PreprocToken<'a> {
    Line(Line<'a>),
    Each(&'a str, Vec<BigInt>),
    End,
}

fn preprocess_line<'a>(line: &'a str, lineno: usize) -> Result<PreprocToken<'a>, ParseErrorType> {
    if let Some(line) = line.strip_prefix('@') {
        let (cmd, rest) = line
            .split_once(|c: char| c.is_ascii_whitespace())
            .unwrap_or((line, ""));
        let rest = rest.trim();
        return match cmd {
            "EACH" => {
                let (name, set) = rest.split_once('=')
                    .ok_or_else(|| ParseErrorType::InvalidPragma(cmd.to_string()))?;
                let name = parse_identifier(name.trim())?;
                let terms = set.trim()
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(']'))
                    .ok_or_else(|| ParseErrorType::InvalidPragma(cmd.to_string()))?
                    .trim();
                let terms: Result<Vec<_>, _> = terms
                    .split_ascii_whitespace()
                    .map(parse_integer)
                    .collect();
                let terms = terms?;
                Ok(PreprocToken::Each(name, terms))
            }
            "END" if rest.is_empty() => Ok(PreprocToken::End),
            "END" => Err(ParseErrorType::InvalidPragma(line.to_string())),
            cmd => {
                let cmd = cmd.to_string();
                Err(ParseErrorType::InvalidPragma(cmd))
            }
        };
    }

    let stmt = if let Some(label) = line.strip_prefix(':') {
        let label = parse_identifier(label)?;
        Statement::Label(label)
    } else {
        let mut terms = line.split_ascii_whitespace();
        let cmd = terms.next().unwrap();
        let args: Result<Vec<_>, _> = terms.map(|t| t.try_into()).collect();
        Statement::Cmd { cmd, args: args? }
    };
    let line = Line { stmt, lineno };
    return Ok(PreprocToken::Line(line));
}

#[derive(Default)]
struct ParseContext {
    args: HashMap<String, BigInt>,
    labels: HashMap<String, usize>,
    missing_labels: HashMap<String, usize>,
}

fn parse(lines: &[String]) -> Result<PietAsm, ParseError> {
    let lines = preprocess(lines)?;
    for line in lines {
        println!("{line:?}");
    }
    todo!();
}

fn parse_line<'a>(line: &'a str, lineno: usize, c: &'a mut ParseContext) -> Result<(), ParseErrorType> {
    let mut cmds = Vec::new();
    let mut terms = line.split_ascii_whitespace();
    let cmd = terms.next().unwrap();
    match cmd {
        "PUSH" => {
            let args: Result<Vec<_>, _> = terms
                .map(|a| parse_int_arg(a, &c.args))
                .collect();
            let args = args?;
            validate_arg_count(args.len(), 1, None)?;
            for arg in args {
                cmds.push(AsmCommand::Push(arg));
            }
        }
        "POP" | "DUP" | "INNUM" | "INCHAR" => {
            let args: Vec<_> = terms.collect();
            validate_arg_count(args.len(), 0, Some(0))?;
            cmds.push(match cmd {
                "POP" => AsmCommand::Pop,
                "DUP" => AsmCommand::Duplicate,
                "INNUM" => AsmCommand::InNum,
                "INCHAR" => AsmCommand::InChar,
                _ => unreachable!(),
            });
        }
        "NOT" | "OUTNUM" | "OUTCHAR" => {
            let args: Result<Vec<_>, _> = terms.map(|a| parse_int_arg(a, &c.args)).collect();
            let args = args?;
            validate_arg_count(args.len(), 0, Some(1))?;
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
            let args: Result<Vec<_>, _> = terms.map(|a| parse_int_arg(a, &c.args)).collect();
            let args = args?;
            validate_arg_count(args.len(), 0, Some(2))?;
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
            let args = args?;
            validate_arg_count(args.len(), 1, Some(1))?;
            let label = args[0];
            if !c.labels.contains_key(label) {
                c.missing_labels.entry(label.to_string()).or_insert(lineno);
            }
        }
        cmd => {
            let cmd = cmd.to_string();
            return Err(ParseErrorType::UnrecognizedCommand(cmd));
        }
    }
    Ok(())
}

pub fn load(filename: &str) -> Result<PietAsm, String> {
    let file = File::open(filename).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Result<Vec<_>, _> = reader.lines().collect();
    let lines = lines.map_err(|e| e.to_string())?;
    parse(&lines).map_err(|e| format!("{e:?}"))
}
