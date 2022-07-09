use crate::asm::preprocessor::{Line, Statement, Token};
use crate::asm::{AsmCommand, ParseError, ParseErrorType, PietAsm};
use std::collections::HashMap;

#[derive(Default)]
struct ParseContext {
    cmds: Vec<AsmCommand>,
    labels: HashMap<String, usize>,
    missing_labels: HashMap<String, usize>,
}

pub(super) fn to_bytecode(ast: Vec<Line>) -> Result<PietAsm, ParseError> {
    let mut context = ParseContext::default();
    for line in ast {
        let lineno = line.lineno;
        parse_line(line, &mut context).map_err(|e| e.at(lineno))?;
    }
    if let Some((label, lineno)) = context.missing_labels.into_iter().next() {
        // TODO: only grabs one here, not great.
        return Err(ParseErrorType::MissingLabel(label).at(lineno));
    }
    let ParseContext { cmds, .. } = context;
    Ok(PietAsm { cmds })
}

fn parse_line(line: Line, c: &mut ParseContext) -> Result<(), ParseErrorType> {
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
        Cmd { cmd: cmd @ ("JUMP" | "JUMPIF"), args } => {
            let mut labels: Vec<String> = validate_args(args, 1, Some(1))?;
            let label = labels.pop().unwrap();
            if !c.labels.contains_key(&label) {
                c.missing_labels.entry(label.clone()).or_insert(lineno);
            }
            match cmd {
                "JUMP" => { c.cmds.push(AsmCommand::Jump(label)); }
                "JUMPIF" => {
                    c.cmds.push(AsmCommand::Not);
                    c.cmds.push(AsmCommand::Not);
                    c.cmds.push(AsmCommand::JumpIf(label));
                }
                _ => unreachable!(),
            }
        }
        Cmd { cmd, .. } => {
            let cmd = cmd.to_string();
            return Err(ParseErrorType::UnrecognizedCommand(cmd));
        }
        Statement::Label(label) => {
            // XXX: i _believe_ we already ran `parse_identifier`,
            // but it'd sure be nice if that were enforced by the type system.
            if c.labels.insert(label.to_string(), lineno).is_some() {
                return Err(ParseErrorType::DuplicateLabel(label.to_string()));
            }
            c.missing_labels.remove(label);
            c.cmds.push(AsmCommand::Label(label.to_string()));
        }
    }
    Ok(())
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
