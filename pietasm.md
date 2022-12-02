# The PietASM Datasheet

PietASM is a small, textual, assembly-ish language, used for compiling into [Piet](https://www.dangermouse.net/esoteric/piet.html) code files.
Largely, commands map one-to-one with Piet commands, although there are exceptions.

Leading whitespace is ignored.

## Comments
Any text following a `#` on a line is ignored as a comment.

```
# this text is ignored
PUSH 5  # the push is run, but this text is ignored!
```

## Command arguments
If a command corresponds to a Piet command which pops arguments from the stack, the arguments may instead be passed as literals.

eg, these three code examples are equivalent:
```asm
PUSH 5
PUSH 3
ADD
```

```asm
PUSH 5
ADD 3
```

```asm
ADD 5 3
```

## Commands
### Math + IO
These commands are largely unchanged from their Piet definitions.
- `PUSH *num` - push `num` onto the stack.
  Any number of constant arguments may be passed, to be pushed onto the stack in order.
- `POP` - pop and discard the top of the stack
- `DUP` - duplicate the top element of the stack
- `ADD` - add the top two elements of the stack
- `SUB` - subtract the top two elements of the stack
- `MUL` - multiply the top two elements of the stack
- `DIV` - divide the top two elements of the stack
- `MOD` - modulo the top two elements of the stack
- `NOT` - replace the top of the stack with 0 if it is nonzero, and 1 if it is zero
- `GREATER` - pop the top two elements of the stack.
  Push 1 if the second-top is larger than the top, 0 otherwise.
- `ROLL` - pop the top two elements of the stack.
  Roll the stack at a depth of `second-top` by `top` positions.
- `INNUM` - read a number from stdin and put it on the stack
- `INCHAR` - read a character from stdin and put its ascii value on the stack
- `OUTNUM` - pop the top element of the stack and print it as a number
- `OUTCHAR` - pop the top element of the stack and print it as an ascii character

### Control Flow
- `STOP` - end execution
- `:FOO` - an identifier prefixed with a colon is a **label**.
  Labels may be jumped to (see below).
  All labels in a file must be unique.
- `JUMP label` - jump to the specified `label` (no colon)
- `JUMPIF label` - pop the top of the stack, and if it is nonzero jump to the specified `label` (no colon)

Note that there are no commands which correspond directly to Piet's `switch` and `pointer` commands, since the details of the Piet image are left to the PietASM compiler.

## Preprocessor Pragma
```asm
@EACH FOO=[1 2 3]
DUP
PUSH @FOO
@END
```

The `@EACH` pragma can be used to duplicate a section of code.
The above code is equivalent to:

```asm
DUP
PUSH 1
DUP
PUSH 2
DUP
PUSH 3
```

The code between the `@EACH` and `@END` lines is added to the file once for each element between the square brackets.
The bracketed values are assigned to the metavariable defined before the `=`, and can be used in place of constants by using the `@` prefix.
