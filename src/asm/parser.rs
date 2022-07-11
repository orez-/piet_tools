use crate::asm::preprocessor::{Line, Statement, Token};
use crate::asm::{AsmCommand, LabelId, ParseError, ParseErrorType, PietAsm};
use std::collections::HashMap;

type LineNo = usize;

struct Label {
    id: LabelId,
    label_lineno: Option<LineNo>,
    jump_lineno: Option<LineNo>,
}

impl Label {
    fn new(id: LabelId) -> Self {
        Label {
            id,
            label_lineno: None,
            jump_lineno: None,
        }
    }
}

#[derive(Default)]
struct ParseContext {
    cmds: Vec<AsmCommand>,
    global_label_id: LabelId,
    labels: HashMap<String, Label>,
}

impl ParseContext {
    fn get_label(&mut self, label_name: String) -> &mut Label {
        self.labels.entry(label_name)
            .or_insert_with(|| {
                self.global_label_id += 1;
                Label::new(self.global_label_id)
            })
    }
}

pub(super) fn to_bytecode(ast: Vec<Line>) -> Result<PietAsm, ParseError> {
    let mut context = ParseContext::default();
    for line in ast {
        let lineno = line.lineno;
        parse_line(line, &mut context).map_err(|e| e.at(lineno))?;
    }

    let mut missing_labels = context.labels.iter()
        .filter(|(_, label)| label.label_lineno.is_none());
    if let Some((name, label)) = missing_labels.next() {
        // TODO: only grabs one here, not great.
        let lineno = label.jump_lineno.unwrap();
        return Err(ParseErrorType::MissingLabel(name.to_string()).at(lineno));
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
        Cmd { cmd: cmd @ ("POP" | "DUP" | "INNUM" | "INCHAR" | "STOP"), args } => {
            validate_arg_count(args.len(), 0, Some(0))?;
            c.cmds.push(match cmd {
                "POP" => AsmCommand::Pop,
                "DUP" => AsmCommand::Duplicate,
                "INNUM" => AsmCommand::InNum,
                "INCHAR" => AsmCommand::InChar,
                "STOP" => AsmCommand::Stop,
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
            let label_name = labels.pop().unwrap();
            let label = c.get_label(label_name);
            label.jump_lineno.get_or_insert(lineno);
            let label_id = label.id;
            match cmd {
                "JUMP" => { c.cmds.push(AsmCommand::Jump(label_id)); }
                "JUMPIF" => {
                    c.cmds.push(AsmCommand::Not);
                    c.cmds.push(AsmCommand::Not);
                    c.cmds.push(AsmCommand::JumpIf(label_id));
                }
                _ => unreachable!(),
            }
        }
        Cmd { cmd, .. } => {
            let cmd = cmd.to_string();
            return Err(ParseErrorType::UnrecognizedCommand(cmd));
        }
        Statement::Label(label_name) => {
            // XXX: i _believe_ we already ran `parse_identifier`,
            // but it'd sure be nice if that were enforced by the type system.
            let label = c.get_label(label_name.to_string());
            if label.label_lineno.is_some() {
                return Err(ParseErrorType::DuplicateLabel(label_name.to_string()));
            }
            let label_id = label.id;
            label.label_lineno = Some(lineno);
            c.cmds.push(AsmCommand::Label(label_id));
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use crate::asm::preprocessor;

    #[test]
    fn test_jump_no_label() {
        let lines = vec!["JUMP NOPE".into()];
        let ast = preprocessor::preprocess(&lines).unwrap();

        assert_matches!(
            to_bytecode(ast),
            Err(ParseError { error_type: ParseErrorType::MissingLabel(s), .. })
                if s == "NOPE"
        )
    }

    #[test]
    fn test_double_label() {
        let lines = vec![
            ":TWIN".into(),
            ":TWIN".into(),
        ];
        let ast = preprocessor::preprocess(&lines).unwrap();

        assert_matches!(
            to_bytecode(ast),
            Err(ParseError { error_type: ParseErrorType::DuplicateLabel(s), .. })
                if s == "TWIN"
        )
    }
}
