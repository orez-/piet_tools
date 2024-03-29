use crate::asm::{AsmCommand, PietAsm};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, One, Zero};

fn push(val: i32) -> AsmCommand {
    AsmCommand::Push(val.into())
}

const BIG_NUMBER: u32 = 100;

pub(super) fn optimize(mut asm: PietAsm) -> PietAsm {
    use AsmCommand::*;

    // Remove labels with no jumps
    asm.cmds.retain(|cmd| {
        !matches!(cmd, AsmCommand::Label(id)
            if asm.jump_counts[*id] == 0
        )
    });

    // Jumps immediately preceding their label
    while let Some((idx, id)) = asm.cmds
            .windows(2)
            .enumerate()
            .filter_map(|(i, w)| match w {
                [Jump(a), Label(b)] if a == b => Some((i, *a)),
                _ => None,
            })
            .next() {
        asm.cmds.remove(idx);
        asm.jump_counts[id] -= 1;
    }

    // TODO: [dyad, POP] => [POP, POP]
    // let constant_patterns: [(Vec<AsmCommand>, Vec<AsmCommand>); _] = [
    let constant_patterns: [(Vec<AsmCommand>, Vec<AsmCommand>); 1] = [
        // XXX: these are all predicated on there being something on the stack!
        // (vec![push(1), Multiply], Vec::new()),
        // (vec![push(1), Divide], Vec::new()),
        // // push(0) needs to get replaced later anyway,
        // // so if we've got a pop handy, instead
        // (vec![Pop, push(0)], vec![push(1), Mod]),
        (vec![Not, Not, Not], vec![Not]),
    ];
    'progress: while {
        // [PUSH T, PUSH T] => [PUSH T, DUPLICATE]
        if let Some(idx) = asm.cmds
            .windows(2)
            .rposition(|w| matches!(w[0], Push(_)) && w[0] == w[1])
        {
            asm.cmds[idx + 1] = Duplicate;
            continue 'progress;
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

pub(super) fn sanitize(mut asm: PietAsm) -> PietAsm {
    use AsmCommand::*;

    // Factor out negative constants
    while let Some((idx, num)) = {
        asm.cmds.iter().enumerate().filter_map(|(i, e)| match e {
            Push(n) if n <= &BigInt::zero() => Some((i, n)),
            _ => None,
        }).next()
    }
    {
        let replace = match num.to_u32() {
            Some(0) => vec![push(1), Not],
            _ => vec![push(1), Push(num + BigInt::one()), Subtract],
        };
        asm.cmds.splice(idx..idx + 1, replace);
    }

    // Factor out large constants
    while let Some((idx, replace)) = {
        asm.cmds.iter().enumerate().filter_map(|(i, e)| match e {
            Push(n) => factor_big_number(n).map(|v| (i, v)),
            _ => None,
        }).next()
    }
    {
        asm.cmds.splice(idx..idx + 1, replace);
    }

    // End on an "STOP"
    if !matches!(asm.cmds.last(), Some(Stop | Jump(_))) {
        asm.cmds.push(Stop);
    }
    asm
}

// TODO: this is hard.
fn factor_big_number(num: &BigInt) -> Option<Vec<AsmCommand>> {
    use AsmCommand::*;

    num.to_u32().map_or(true, |n| n >= BIG_NUMBER).then(|| {
        let sqrt = num.sqrt();
        let diff = num - (&sqrt * &sqrt);
        let mut result = vec![Push(sqrt), Duplicate, Multiply];
        if diff != BigInt::zero() {
            result.push(Push(diff));
            result.push(Add);
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::AsmCommand::*;

    fn to_piet_asm(cmds: Vec<AsmCommand>) -> PietAsm {
        let count = cmds.iter().filter(|c| matches!(c, Label(_))).count();
        let mut jump_counts = vec![0; count];
        for cmd in &cmds {
            match cmd {
                Jump(id) | JumpIf(id) => {
                    jump_counts[*id] += 1;
                }
                _ => (),
            }
        }
        PietAsm { cmds, jump_counts }
    }

    #[test]
    fn test_dup_pushes() {
        let asm = to_piet_asm(vec![push(5), push(2), push(2), push(2), push(8), push(8)]);
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![push(5), push(2), Duplicate, Duplicate, push(8), Duplicate]);
    }

    #[test]
    fn test_stack_bump() {
        let asm = to_piet_asm(vec![push(1), Multiply]);
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![push(1), Multiply]);
    }

    #[test]
    fn test_rm_unused_labels() {
        let asm = to_piet_asm(vec![Label(0), push(1), Label(1), push(2), Label(2), Jump(1)]);
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![push(1), Label(1), push(2), Jump(1)]);
    }

    #[test]
    fn test_rm_unnecessary_jump() {
        let asm = to_piet_asm(vec![Jump(0), Label(0), Jump(0)]);
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![Label(0), Jump(0)]);
    }

    #[test]
    fn test_rm_unnecessary_jump_and_label() {
        let asm = to_piet_asm(vec![Jump(0), Label(0)]);
        let PietAsm { cmds, .. } = optimize(asm);
        assert_eq!(cmds, vec![]);
    }
}
