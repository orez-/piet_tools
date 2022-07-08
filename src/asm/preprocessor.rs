use crate::asm::{ParseError, ParseErrorType};
use num_bigint::BigInt;

enum PreprocToken<'a> {
    Line(Line<'a>),
    Each(&'a str, Vec<BigInt>),
    End,
}

/// Prep the pasm file for processing.
/// This will:
/// - Annotate lines with their line numbers
/// - Strip comments + blank lines
/// - Expand macros
/// - Convert the code into an AST
pub(super) fn preprocess(lines: &[String]) -> Result<Vec<Line>, ParseError> {
    let lines = lines.iter().enumerate().filter_map(|(lineno, line)| {
        let lineno = lineno + 1;
        let line = line.split('#').next().unwrap().trim();
        (!line.is_empty()).then(|| (lineno, line))
    });
    let mut command_stack = Vec::new();
    let mut commands = Vec::new();
    for (lineno, line) in lines {
        let pp_token = preprocess_line(line, lineno).map_err(|e| e.at(lineno))?;
        match pp_token {
            PreprocToken::Line(cmd) => { commands.push(cmd); }
            PreprocToken::Each(name, terms) => {
                command_stack.push((name, terms, commands, lineno));
                commands = Vec::new();
            }
            PreprocToken::End => {
                let (name, terms, mut restored_cmds, _) = command_stack.pop()
                    .ok_or_else(|| ParseErrorType::ExtraEnd.at(lineno))?;
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

fn preprocess_line(line: &str, lineno: usize) -> Result<PreprocToken<'_>, ParseErrorType> {
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

// TODO: these structs and enums are returned from the preprocessor,
// but consumed by the parser. Does it make sense to define them somewhere else?
// `mod.rs`? A `models.rs`?

// Also all of these names suck and I hate them.

#[derive(Clone, Debug)]
pub(super) enum Token {
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
            None => match arg.parse() {
                Ok(int) => Token::Num(int),
                Err(_) => Token::Label(parse_identifier(arg)?.to_string()),
            },
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
pub(super) struct Line<'a> {
    pub(super) lineno: usize,
    pub(super) stmt: Statement<'a>,
}

impl Line<'_> {
    fn bind(&mut self, name: &str, value: &BigInt) {
        self.stmt.bind(name, value);
    }
}

#[derive(Clone, Debug)]
pub(super) enum Statement<'a> {
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
