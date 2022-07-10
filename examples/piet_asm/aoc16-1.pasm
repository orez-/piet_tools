# test
PUSH 0 0 1 1
:INPUT_LOOP
  # hex to int: c - (c // 58) * 7 - 48
  PUSH 0
  INCHAR
  DUP
  DUP
  NOT
  JUMPIF INPUT_LOOP_END  # [01]
  DIV 58
  MUL 7
  SUB
  SUB 48
  # destroy extra duplicate from eofcheck
  ADD

  # split into bits and add to bitstream
  @EACH MASK=[8 4 2 1]
    DUP
    # TODO
    PUSH @MASK
    # a = hexit // mask
    DIV
    ROLL 3 2
    # b = shft * a
    MUL
    ROLL 4 3
    # total += b
    ADD
    ROLL 3 2
    # shft *= 2
    MUL 2
    DUP
    ROLL 4 3
    # hexit %= mask
    MOD @MASK
  @END
  POP
JUMP INPUT_LOOP
:INPUT_LOOP_END
POP
POP
POP
POP

# And now we have encoded the input as a single integer,
# which we can start poppin bits off of.
:MAIN
  @EACH X=[4 2 1]
    DUP
    MOD 2
    MUL @X
    ROLL 3 2
    ADD
    ROLL 2 1
    DIV 2
  @END

  # Check packet type for literals
  DUP
  DIV 8
  ROLL 2 1
  MOD 8
  SUB 1
  NOT
  # if p != 4 start again from the top
  JUMPIF INTSEEK
    DUP
    MOD 2
    NOT
    JUMPIF DIV_TRUE
    DIV 16
    :DIV_TRUE
    DIV 4096
    JUMP MAIN
  # Otherwise we're seeking through an integer!
  :INTSEEK
    DUP
    DIV 32
    ROLL 2 1
    MOD 2
    # break unless repeat
    JUMPIF INTSEEK
  :INTSEEK_END

  DUP
  NOT
  # break if the stream is empty
  NOT
JUMPIF MAIN
POP
OUTNUM
