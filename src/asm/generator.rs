use crate::asm::{AsmCommand, PietAsm};
use crate::{Color, Command, PietCode};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::iter::repeat;
use std::mem::{self, ManuallyDrop};

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

    // execution_direction: InstructionPointer,
    last_color: Option<Color>,
    x: usize,
    y: usize,
}

impl PietCodeBuffer {
    fn new(width: usize, height: usize) -> Self {
        PietCodeBuffer {
            width,
            height,
            code: vec![Color::Other; width * height],
            // TODO: i got the sense these don't really belong here, really we need
            // a layer atop the PCB to manage these. But this was getting to be a
            // daunting change, so for now here they be.
            // Working with this a bit more, these definitely don't belong here.
            // Make another layer.
            last_color: None,
            x: 0,
            y: 0,
        }
    }

    fn allocate(&mut self, width: usize) -> Result<PietCodeBufferEdit, DrawError> {
        let height = ROW_HEIGHT;
        if self.x + width >= WIDTH {
            self.reserve(height);
            let x = self.x;
            let y = self.y;
            PietCodeBufferEdit::new(self).draw_newline(x, y)?;
            self.x = 2;
            self.y += ROW_HEIGHT;
            self.last_color = None;
        }
        let area = Rect { x: self.x, y: self.y, width, height };
        Ok(PietCodeBufferEdit::new_slice(self, area))
    }

    /// Resize the buffer to accommodate `additional_height`
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

    fn clone_slice(&mut self, area: Rect) -> PietCodeBuffer {
        // TODO: bounds checking
        let Rect { x, y, width, height } = area;
        let mut code = Vec::with_capacity(width * height);
        for dy in y..y+height {
            for dx in x..x+width {
                let idx = dy * self.width + dx;
                code.push(self.code[idx]);
            }
        }
        PietCodeBuffer {
            code, width, height,
            last_color: None, x: 0, y: 0,
        }
    }

    fn blit(&mut self, source: PietCodeBuffer, dest: Rect) {
        let Rect { x, y, width, height } = dest;
        let mut src = 0;
        for dy in y..y+height {
            for dx in x..x+width {
                self.draw_pixel_overwrite(dx, dy, source.code[src]).unwrap();
                src += 1;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
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
    area: Rect,
}

impl<'a> PietCodeBufferEdit<'a> {
    fn new(pcb: &'a mut PietCodeBuffer) -> Self {
        let area = Rect {
            x: 0,
            y: 0,
            width: pcb.width,
            height: pcb.height,
        };
        Self::new_slice(pcb, area)
    }

    fn new_slice(pcb: &'a mut PietCodeBuffer, area: Rect) -> Self {
        let slice = pcb.clone_slice(area);
        PietCodeBufferEdit {
            edited: ManuallyDrop::new(slice),
            original: pcb,
            poisoned: false,
            area,
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
        self.draw_rect(x, y, 1, ROW_HEIGHT - 2, Color::White)?;
        self.draw_horiz(y + ROW_HEIGHT - 2)?;
        self.draw_pixel(x + 1, y, Color::Black)?;
        self.draw_pixel(x, y + ROW_HEIGHT - 1, Color::Black)?;
        self.draw_pixel(0, y + ROW_HEIGHT - 4, Color::Black)?;
        self.draw_pixel(2, y + ROW_HEIGHT - 3, Color::Black)?;
        self.draw_pixel(1, y + ROW_HEIGHT + 2, Color::Black)?;
        self.draw_rect(0, y + ROW_HEIGHT - 3, 2, 5, Color::White)?;
        self.draw_pixel_overwrite(0, y + ROW_HEIGHT - 1, Color::Black)?;
        Ok(())
    }
}

impl Drop for PietCodeBufferEdit<'_> {
    fn drop(&mut self) {
        // SAFETY - it is unsafe to use `self.edited` after this,
        // but since we're immediately dropping this whole struct
        // I _think_ there's no chance of that.
        if !self.poisoned {
            let code = unsafe { ManuallyDrop::take(&mut self.edited) };
            self.original.blit(code, self.area);
        }
    }
}

impl From<PietCodeBuffer> for PietCode {
    fn from(this: PietCodeBuffer) -> PietCode {
        let PietCodeBuffer { width, height, code, .. } = this;
        PietCode { width, height, code }
    }
}

pub(super) fn generate(asm: PietAsm) -> PietCode {
    let mut buffer = PietCodeBuffer::new(WIDTH, ROW_HEIGHT);

    // let mut labels: HashMap<String, (usize, usize)> = HashMap::new();
    // let mut unmatched_jumps: HashMap<String, (usize, usize)> = HashMap::new();

    // wow i suddenly get why Rust could use a `try` block.
    let res = (|| -> Result<(), DrawError> {
        for cmd in asm.cmds {
            println!("{cmd:?}");
            match cmd {
                AsmCommand::Label(s) => {
                    let mut edit = buffer.allocate(4)?;
                    edit.draw_pixel(0, 0, Color::White)?;
                    edit.draw_rect(1, 0, 2, 2, Color::White)?;
                    // edit.draw_pixel(1, -1, Color::Black)?;  // TODO: fix outta bounds
                    edit.draw_pixel(0, 1, Color::Black)?;
                    edit.draw_pixel(2, 2, Color::Black)?;
                    mem::drop(edit);
                    // labels.insert(s, (x + 1, y));
                    buffer.x += 3;
                    buffer.last_color = None;
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

                    let has_color = buffer.last_color.is_some();
                    let mut edit = buffer.allocate(width + 5)?;
                    let mut x = 0;
                    if has_color {
                        edit.draw_pixel(0, 0, Color::White)?;
                        x = 1;
                    }
                    edit.draw_rect(x, 0, width, ROW_FILL_HEIGHT, CONTROL_COLOR)?;
                    x += width;
                    if extra > 0 {
                        edit.draw_rect(x, 0, 1, extra, CONTROL_COLOR)?;
                        x += 1;
                    }
                    edit.draw_pixel(x, 0, CONTROL_COLOR)?;
                    let color = CONTROL_COLOR.next_for_command(Command::Push);
                    edit.draw_pixel(x + 1, 0, color)?;
                    mem::drop(edit);
                    buffer.x += x + 2;
                    buffer.last_color = Some(color);
                }
                AsmCommand::Pop | AsmCommand::Add | AsmCommand::Subtract | AsmCommand::Multiply |
                AsmCommand::Divide | AsmCommand::Mod | AsmCommand::Not | AsmCommand::Greater |
                AsmCommand::Duplicate | AsmCommand::Roll | AsmCommand::InNum | AsmCommand::InChar |
                AsmCommand::OutNum | AsmCommand::OutChar => {
                    let cmd: Command = cmd.try_into().unwrap();
                    let mut x = 0;
                    let last_color = buffer.last_color;
                    let mut edit = buffer.allocate(3)?;
                    let color = match last_color {
                        Some(color) => color,
                        None => {
                            edit.draw_pixel(0, 0, CONTROL_COLOR)?;
                            x += 1;
                            CONTROL_COLOR
                        }
                    };
                    let color = color.next_for_command(cmd);
                    edit.draw_pixel(x, 0, color)?;
                    mem::drop(edit);
                    buffer.x += x + 1;
                    buffer.last_color = Some(color);
                }
                AsmCommand::Stop => {
                    let mut edit = buffer.allocate(4)?;
                    // edit.draw_rect(0, -1, 4, 4, Color::Black)?;  // TODO: fix outta boundddds..
                    edit.draw_pixel_overwrite(0, 0, Color::White)?;
                    edit.draw_pixel_overwrite(1, 0, Color::White)?;
                    edit.draw_pixel_overwrite(2, 0, CONTROL_COLOR)?;
                    edit.draw_pixel_overwrite(2, 1, CONTROL_COLOR)?;
                    edit.draw_pixel_overwrite(1, 1, CONTROL_COLOR)?;
                    mem::drop(edit);
                    buffer.x += 4;
                    buffer.last_color = None;  // TODO: is this right?
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
