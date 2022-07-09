use crate::asm::{AsmCommand, PietAsm};
use crate::{Color, Command, PietCode};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::iter::repeat;
use std::mem::ManuallyDrop;

// const WIDTH: usize = 800;
const WIDTH: usize = 100;
const ROW_HEIGHT: usize = 10;
const ROW_FILL_HEIGHT: usize = 5;
const CONTROL_COLOR: Color = Color::Red;

#[derive(Debug)]
enum DrawError {
    OutOfBounds(usize, usize),
    ColorMismatch(Color, Color),
    Todo,
}

#[derive(Debug, Clone)]
struct PietCodeBuffer {
    width: usize,
    height: usize,
    code: Vec<Color>,
}

impl PietCodeBuffer {
    fn new(width: usize, height: usize) -> Self {
        PietCodeBuffer {
            width,
            height,
            code: vec![Color::Other; width * height],
        }
    }

    fn edit(&mut self) -> PietCodeBufferEdit {
        PietCodeBufferEdit::new(self)
    }

    fn reserve(&mut self, additional_height: usize) {
        self.height += additional_height;
        self.code.extend(repeat(Color::Other).take(self.width * additional_height));
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawError> {
        if x >= self.width || y >= self.height {
            // TODO: kind of spooky with our resizeable buffer. reconsider this.
            if matches!(color, Color::Black) { return Ok(()); }
            else { return Err(DrawError::OutOfBounds(x, y)); }
        }
        let idx = y * self.width + x;
        match &mut self.code[idx] {
            c @ Color::Other => { *c = color; }
            c if *c == color => (),
            c => { return Err(DrawError::ColorMismatch(color, *c)); }
        }
        Ok(())
    }

    fn draw_pixel_overwrite(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawError> {
        if x >= self.width || y >= self.height {
            // TODO: kind of spooky with our resizeable buffer. reconsider this.
            if matches!(color, Color::Black) { return Ok(()); }
            else { return Err(DrawError::OutOfBounds(x, y)); }
        }
        let idx = y * self.width + x;
        self.code[idx] = color;
        Ok(())
    }
}

/// Helper struct to group potentially destructive edits.
/// If any write command fails, the entire transaction is rolled back.
// TODO: mmmm not sure the full clone is the best way to express this,
// but let's do our best to encapsulate that decision within this struct
// so we can swap it out later if we want.
struct PietCodeBufferEdit<'a> {
    original: &'a mut PietCodeBuffer,
    edited: ManuallyDrop<PietCodeBuffer>,
    poisoned: bool,
}

impl<'a> PietCodeBufferEdit<'a> {
    fn new(pcb: &'a mut PietCodeBuffer) -> Self {
        PietCodeBufferEdit {
            edited: ManuallyDrop::new(pcb.clone()),
            original: pcb,
            poisoned: false,
        }
    }

    fn poison_on_err<T>(&mut self, res: Result<T, DrawError>) -> Result<T, DrawError> {
        match res {
            err @ Err(_) => {
                self.poisoned = true;
                err
            }
            ok => ok,
        }
    }

    fn reserve(&mut self, additional_height: usize) {
        self.edited.reserve(additional_height);
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawError> {
        let result = self.edited.draw_pixel(x, y, color);
        self.poison_on_err(result)
    }

    fn draw_pixel_overwrite(&mut self, x: usize, y: usize, color: Color) -> Result<(), DrawError> {
        let result = self.edited.draw_pixel_overwrite(x, y, color);
        self.poison_on_err(result)
    }

    fn draw_rect(&mut self, left: usize, top: usize, width: usize, height: usize, color: Color) -> Result<(), DrawError> {
        for x in left..left + width {
            for y in top..top + height {
                let res = self.draw_pixel(x, y, color);
                self.poison_on_err(res)?;
            }
        }
        Ok(())
    }

    fn draw_horiz(&mut self, y: usize) -> Result<(), DrawError> {
        for x in 0..self.edited.width {
            let res = self.draw_pixel(x, y, Color::White);
            self.poison_on_err(res)?;
        }
        Ok(())
    }

    fn draw_newline(&mut self, x: usize, y: usize) -> Result<(), DrawError> {
        self.reserve(ROW_HEIGHT);
        self.draw_rect(x, y, 1, ROW_HEIGHT - 2, Color::White)?;
        self.draw_horiz(y + ROW_HEIGHT - 2)?;
        self.draw_pixel(x + 1, y, Color::Black)?;
        self.draw_pixel(x, y + ROW_HEIGHT - 1, Color::Black)?;
        self.draw_pixel(0, y + ROW_HEIGHT - 4, Color::Black)?;
        self.draw_pixel(2, y + ROW_HEIGHT - 3, Color::Black)?;
        self.draw_pixel(1, y + ROW_HEIGHT + 2, Color::Black)?;
        self.draw_rect(0, y + ROW_HEIGHT - 3, 2, 5, Color::White)?;
        self.draw_pixel_overwrite(0, y + ROW_HEIGHT - 1, Color::Black)
    }
}

impl Drop for PietCodeBufferEdit<'_> {
    fn drop(&mut self) {
        // SAFETY - it is unsafe to use `self.edited` after this,
        // but since we're immediately dropping this whole struct
        // I _think_ there's no chance of that.
        if !self.poisoned {
            *self.original = unsafe { ManuallyDrop::take(&mut self.edited) };
        }
    }
}

impl From<PietCodeBuffer> for PietCode {
    fn from(this: PietCodeBuffer) -> PietCode {
        let PietCodeBuffer { width, height, code } = this;
        PietCode { width, height, code }
    }
}

pub(super) fn generate(asm: PietAsm) -> PietCode {
    // let mut execution_direction = InstructionPointer::default();
    let mut last_color: Option<Color> = None;
    let mut buffer = PietCodeBuffer::new(WIDTH, ROW_HEIGHT);
    let mut x = 0;
    let mut y = 0;

    let mut labels: HashMap<String, (usize, usize)> = HashMap::new();

    // wow i suddenly get why Rust could use a `try` block.
    let res = (|| -> Result<(), DrawError> {
        for cmd in asm.cmds {
            println!("{cmd:?}");
            match cmd {
                AsmCommand::Label(s) => {
                    if x + 4 >= WIDTH {
                        buffer.edit().draw_newline(x, y)?;
                        x = 2;
                        y += ROW_HEIGHT;
                        last_color = None;
                    }
                    let mut edit = buffer.edit();
                    edit.draw_pixel(x, y, Color::White)?;
                    edit.draw_rect(x + 1, y, 2, 2, Color::White)?;
                    edit.draw_pixel(x + 1, y - 1, Color::Black)?;
                    edit.draw_pixel(x, y + 1, Color::Black)?;
                    edit.draw_pixel(x + 2, y + 2, Color::Black)?;

                    labels.insert(s, (x + 1, y));
                    x += 3;
                    last_color = None;
                }
                AsmCommand::Jump(_) | AsmCommand::JumpIf(_) => {
                    eprintln!("Skipping {cmd:?} for a sec! Sorry!");
                }
                AsmCommand::Push(num) => {
                    // TODO: push is hard.. as a first pass we're unconditionally
                    // ensuring a white intro, but we could try being more
                    // clever here.
                    let num = num.to_usize().expect("larger constants are unsupported");
                    let sans_dangle = num - 1;
                    let width = sans_dangle / ROW_FILL_HEIGHT;
                    let extra = sans_dangle % ROW_FILL_HEIGHT;

                    if x + width + 5 >= WIDTH {
                        buffer.edit().draw_newline(x, y)?;
                        x = 2;
                        y += ROW_HEIGHT;
                        last_color = None;
                    }

                    let mut edit = buffer.edit();
                    if last_color.is_some() {
                        edit.draw_pixel(x, y, Color::White)?;
                        x += 1;
                    }
                    edit.draw_rect(x, y, width, ROW_FILL_HEIGHT, CONTROL_COLOR)?;
                    x += width;
                    if extra > 0 {
                        edit.draw_rect(x, y, 1, extra, CONTROL_COLOR)?;
                        x += 1;
                    }
                    edit.draw_pixel(x, y, CONTROL_COLOR)?;
                    x += 1;
                    let color = CONTROL_COLOR.next_for_command(Command::Push);
                    edit.draw_pixel(x, y, color)?;
                    x += 1;
                    last_color = Some(color);
                }
                AsmCommand::Pop | AsmCommand::Add | AsmCommand::Subtract | AsmCommand::Multiply |
                AsmCommand::Divide | AsmCommand::Mod | AsmCommand::Not | AsmCommand::Greater |
                AsmCommand::Duplicate | AsmCommand::Roll | AsmCommand::InNum | AsmCommand::InChar |
                AsmCommand::OutNum | AsmCommand::OutChar => {
                    if x + 3 >= WIDTH {
                        buffer.edit().draw_newline(x, y)?;
                        x = 2;
                        y += ROW_HEIGHT;
                        last_color = None;
                    }
                    let cmd: Command = cmd.try_into().unwrap();
                    let mut edit = buffer.edit();
                    let color = match last_color {
                        Some(color) => color,
                        None => {
                            edit.draw_pixel(x, y, CONTROL_COLOR)?;
                            x += 1;
                            CONTROL_COLOR
                        }
                    };
                    let color = color.next_for_command(cmd);
                    edit.draw_pixel(x, y, color)?;
                    x += 1;
                    last_color = Some(color);
                }
                AsmCommand::Stop => {
                    if x + 4 >= WIDTH {
                        buffer.edit().draw_newline(x, y)?;
                        x = 2;
                        y += ROW_HEIGHT;
                        last_color = None;
                    }
                    let mut edit = buffer.edit();
                    edit.draw_rect(x, y - 1, 4, 4, Color::Black)?;
                    edit.draw_pixel_overwrite(x, y, Color::White)?;
                    edit.draw_pixel_overwrite(x + 1, y, Color::White)?;
                    edit.draw_pixel_overwrite(x + 2, y, CONTROL_COLOR)?;
                    edit.draw_pixel_overwrite(x + 2, y + 1, CONTROL_COLOR)?;
                    edit.draw_pixel_overwrite(x + 1, y + 1, CONTROL_COLOR)?;
                    x += 4;
                }
            }
        }
        Ok(())
    })();
    match res {
        Ok(_) => (),
        Err(e) => {
            println!("error: {e:?}");
        }
    }
    buffer.into()
}
