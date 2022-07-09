use crate::asm::{AsmCommand, PietAsm};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, One, Zero};

fn push(val: i32) -> AsmCommand {
    AsmCommand::Push(val.into())
}

const BIG_NUMBER: u32 = 100;

pub(super) fn optimize(mut asm: PietAsm) -> PietAsm {
    use AsmCommand::*;

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
