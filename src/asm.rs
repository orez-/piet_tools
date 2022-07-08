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
#[allow(dead_code)]
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

impl TryFrom<Token> for BigInt {
    type Error = ParseErrorType;

    fn try_from(token: Token) -> Result<BigInt, ParseErrorType> {
        match token {
            Token::Var(var) => Err(ParseErrorType::UnboundVarError(var)),
            Token::Num(int) => Ok(int),
            Token::Label(_) => Err(ParseErrorType::TypeError),
        }
    }
}

// TODO: this super sucks. Make a dedicated Label type
impl TryFrom<Token> for String {
    type Error = ParseErrorType;

    fn try_from(token: Token) -> Result<String, ParseErrorType> {
        match token {
            Token::Var(var) => Err(ParseErrorType::UnboundVarError(var)),
            Token::Num(_) => Err(ParseErrorType::TypeError),
            Token::Label(label) => Ok(label),
        }
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
    TypeError,  // TODO: any metadata.
}

impl ParseErrorType {
    fn at(self, lineno: usize) -> ParseError {
        ParseError {
            lineno,
            error_type: self,
        }
    }
}

fn validate_args<T>(args: Vec<Token>, min: usize, max: Option<usize>) -> Result<Vec<T>, ParseErrorType>
where T: TryFrom<Token, Error = ParseErrorType> {
    validate_arg_count(args.len(), min, max)?;
    args.into_iter().map(|t| t.try_into()).collect()
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
    cmds: Vec<AsmCommand>,
    labels: HashMap<String, usize>,
    missing_labels: HashMap<String, usize>,
}

fn parse(lines: &[String]) -> Result<PietAsm, ParseError> {
    let lines = preprocess(lines)?;
    let mut context = ParseContext::default();
    for line in lines {
        let lineno = line.lineno;
        parse_line(line, &mut context)
            .map_err(|e| e.at(lineno))?;
    }
    if let Some((label, lineno)) = context.missing_labels.into_iter().next() {
        // TODO: only grabs one here, not great.
        return Err(ParseErrorType::MissingLabel(label.to_string()).at(lineno));
    }
    let ParseContext { cmds, .. } = context;
    Ok(PietAsm { cmds })
}

fn parse_line<'a>(line: Line, c: &'a mut ParseContext) -> Result<(), ParseErrorType> {
    use Statement::Cmd;

    let lineno = line.lineno;

    match line.stmt {
        Cmd { cmd: "PUSH", args } => {
            let args = validate_args(args, 1, None)?;
            for arg in args {
                c.cmds.push(AsmCommand::Push(arg));
            }
        }
        Cmd { cmd: cmd @ ("POP" | "DUP" | "INNUM" | "INCHAR"), args } => {
            validate_arg_count(args.len(), 0, Some(0))?;
            c.cmds.push(match cmd {
                "POP" => AsmCommand::Pop,
                "DUP" => AsmCommand::Duplicate,
                "INNUM" => AsmCommand::InNum,
                "INCHAR" => AsmCommand::InChar,
                _ => unreachable!(),
            });
        }
        Cmd { cmd: cmd @ ("NOT" | "OUTNUM" | "OUTCHAR"), args } => {
            let args = validate_args(args, 0, Some(1))?;
            for arg in args {
                c.cmds.push(AsmCommand::Push(arg));
            }
            c.cmds.push(match cmd {
                "NOT" => AsmCommand::Not,
                "OUTNUM" => AsmCommand::OutNum,
                "OUTCHAR" => AsmCommand::OutChar,
                _ => unreachable!(),
            });
        }
        Cmd { cmd: cmd @ ("ADD" | "SUB" | "MUL" | "DIV" | "MOD" | "GREATER" | "ROLL"), args } => {
            let args = validate_args(args, 0, Some(2))?;
            for arg in args {
                c.cmds.push(AsmCommand::Push(arg));
            }
            c.cmds.push(match cmd {
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
        Cmd { cmd: "JUMP" | "JUMPIF", args } => {
            let mut labels: Vec<String> = validate_args(args, 1, Some(1))?;
            let label = labels.pop().unwrap();
            if !c.labels.contains_key(&label) {
                c.missing_labels.entry(label).or_insert(lineno);
            }
        }
        Cmd { cmd, .. } => {
            let cmd = cmd.to_string();
            return Err(ParseErrorType::UnrecognizedCommand(cmd));
        }
        Statement::Label(_) => todo!()
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
