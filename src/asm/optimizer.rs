use crate::asm::{AsmCommand, PietAsm};

pub(super) fn optimize(mut asm: PietAsm) -> PietAsm {
    use AsmCommand::*;

    fn push(val: i32) -> AsmCommand {
        Push(val.into())
    }

    // TODO: [dyad, POP] => [POP, POP]
    // let constant_patterns: [(Vec<AsmCommand>, Vec<AsmCommand>); _] = [
    let constant_patterns: [(Vec<AsmCommand>, Vec<AsmCommand>); 0] = [
        // XXX: these are all predicated on there being something on the stack!
        // (vec![push(1), Multiply], Vec::new()),
        // (vec![push(1), Divide], Vec::new()),
        // // push(0) needs to get replaced later anyway,
        // // so if we've got a pop handy, instead
        // (vec![Pop, push(0)], vec![push(1), Mod]),
    ];
    'progress: while {
        // [PUSH T, PUSH T] => [PUSH T, DUPLICATE]
        if let Some(idx) = asm.cmds
            .windows(2)
            .rposition(|w| matches!(w[0], Push(_)) && w[0] == w[1])
        {
            asm.cmds[idx + 1] = Duplicate;
            continue 'progress
        }

        // Run through all the constant patterns
        for (needle, replace_with) in &constant_patterns {
            let len = needle.len();
            if let Some(idx) = asm.cmds
                .windows(len)
                .position(|w| w == needle.as_slice())
            {
                asm.cmds.splice(idx..idx + len, replace_with.iter().cloned());
                continue 'progress;
            }
        }
        false
    } {}

    asm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::AsmCommand::*;

    fn push(val: i32) -> AsmCommand {
        Push(val.into())
    }

    #[test]
    fn test_dup_pushes() {
        let asm = PietAsm { cmds: vec![push(5), push(2), push(2), push(2), push(8), push(8)] };
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![push(5), push(2), Duplicate, Duplicate, push(8), Duplicate]);
    }

    #[test]
    fn test_stack_bump() {
        let asm = PietAsm { cmds: vec![push(1), Multiply] };
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![push(1), Multiply]);
    }
}
