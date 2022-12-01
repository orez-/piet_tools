use crate::asm::{AsmCommand, LabelId, PietAsm};
use crate::{Color, Command, PietCode};
use indoc::indoc;
use num_traits::ToPrimitive;
use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use std::mem::{self, ManuallyDrop};

// const WIDTH: usize = 800;
const WIDTH: usize = 100;
const ROW_HEIGHT: usize = 10;
const ROW_FILL_HEIGHT: usize = 5;
const CONTROL_COLOR: Color = Color::Red;

macro_rules! draw {
    ($buffer: expr, $pattern: literal) => {{
        let pattern = DrawPattern::generate(indoc! {$pattern}, Vec::new());
        $buffer.draw(pattern)
    }};
    ($buffer: expr, $pattern: literal, $($tail:tt)*) => {{
        let pattern = DrawPattern::generate(indoc! {$pattern}, vec![$($tail)*]);
        $buffer.draw(pattern)
    }};
}

macro_rules! draw_here {
    ($buffer: expr, $pattern: literal) => {{
        let pattern = DrawPattern::generate(indoc! {$pattern}, Vec::new());
        $buffer.draw_here(pattern)
    }};
    ($buffer: expr, $pattern: literal, $($tail:tt)*) => {{
        let pattern = DrawPattern::generate(indoc! {$pattern}, vec![$($tail)*]);
        $buffer.draw_here(pattern)
    }};
}

struct DrawPattern {
    off_y: usize,
    step_x: usize,
    allocation_width: usize,
    last_color: Option<Color>,
    pixels: Vec<(usize, usize, Color)>,
}

impl DrawPattern {
    fn generate(pattern: &[u8], colors: Vec<Color>) -> Self {
        let mut x = 0;
        let mut y = 0;
        let mut pixels = Vec::new();
        let mut start_y = None;
        let mut end = None;
        for c in pattern {
            match c {
                b' ' => (),
                b'>' if x == 0 => {
                    if start_y.is_some() { panic!(); }
                    start_y = Some(y);
                }
                _ if x == 0 => { panic!(); }
                b'>' => {
                    if end.is_some() { panic!(); }
                    end = Some((x - 1, y));
                }
                b'.' => { pixels.push((x - 1, y, Color::White)); }
                b'#' => { pixels.push((x - 1, y, Color::Black)); }
                b'a'..=b'z' => {
                    let idx = (c - b'a') as usize;
                    pixels.push((x - 1, y, colors[idx]));
                }
                b'\n' => {
                    x = 0;
                    y += 1;
                    continue;
                }
                _ => { panic!(); }
            }
            x += 1;
        }
        // We want to allocate enough space to draw all the pixels we're drawing.
        // But if we're going to have to follow this up with another command
        // we'll need another pixel to ensure we don't write into a wall.
        let width = pixels.iter().map(|&(x, _, _)| x).max().unwrap() + 1;
        let allocation_width = width + end.is_some() as usize;
        let start_y = start_y.unwrap();
        assert_eq!(start_y, 1);  // ugh. for now.
        // let start_color = pixels.iter()
        //     .filter_map(|&(x, y, p)| ((x - 1, y) == start).then(|| p))
        //     .next().unwrap();
        let end_color = end.map(|end| {
            pixels.iter()
                .filter_map(|&(x, y, p)| ((x + 1, y) == end).then(|| p))
                .next().unwrap()
        });
        if let Some((x, y)) = end {
            // for now, at least.
            assert!(start_y == y);
            assert!(pixels.iter().all(|&(px, _, _)| px < x));
        }
        DrawPattern {
            off_y: start_y,
            step_x: width,
            last_color: end_color,
            allocation_width,
            pixels,
        }
    }
}

#[derive(Debug)]
enum DrawError {
    OutOfBounds(usize, usize),
    ColorMismatch(Color, Color, (usize, usize)),
    AllocationError,
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
    jump_xs: HashSet<usize>,
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
            jump_xs: HashSet::new(),
        }
    }

    fn draw(&mut self, pattern: DrawPattern) -> Result<(), DrawError> {
        // uh oh, that last_color thing.
        let (mut edit, _) = self.allocate(pattern.allocation_width)?;
        for (x, y, color) in pattern.pixels {
            edit.draw_pixel(x, y, color)?;
        }
        mem::drop(edit);
        self.last_color = pattern.last_color;
        self.x += pattern.step_x;
        Ok(())
    }

    fn draw_here(&mut self, pattern: DrawPattern) -> Result<(), DrawError> {
        let mut edit = self.allocate_here(pattern.allocation_width)?;
        for (x, y, color) in pattern.pixels {
            edit.draw_pixel(x, y, color)?;
        }
        mem::drop(edit);
        self.last_color = pattern.last_color;
        self.x += pattern.step_x;
        Ok(())
    }

    fn allocate_here(&mut self, width: usize) -> Result<PietCodeBufferEdit, DrawError> {
        let height = ROW_HEIGHT;
        let area = Rect { x: self.x, y: self.y, width, height };
        Ok(PietCodeBufferEdit::new_slice(self, area))
    }

    // TODO signature sucks, burn this place down
    fn allocate(&mut self, width: usize) -> Result<(PietCodeBufferEdit, Option<Color>), DrawError> {
        const ATTEMPTS: i32 = 10;
        let height = ROW_HEIGHT;
        let mut attempts = 0;
        while attempts < ATTEMPTS {
            if self.x + width >= WIDTH {
                self.reserve(height);
                let x = self.x;
                let y = self.y;
                PietCodeBufferEdit::new(self).draw_newline(x, y + 1)?;
                self.x = 2;
                self.y += height;
                self.last_color = Some(Color::White);
            }
            let idx = (0..width).rev().filter_map(|w| {
                let x = w + self.x;
                self.jump_xs.contains(&x).then(|| x)
            }).next();
            if let Some(idx) = idx {
                let x = self.x;
                let y = self.y;
                PietCodeBufferEdit::new(self)
                    .draw_rect(x, y + 1, idx - x + 1, 1, Color::White)?;
                self.x = idx + 1;
                // AUGHGHHH this never gets read,
                // since we're returning the PCBE at the end here.
                // TODO: hoist this metadata crap.
                self.last_color = Some(Color::White);
                println!("bumpin");
                attempts += 1;
                continue;
            }
            break;
        }
        if attempts >= ATTEMPTS {
            eprintln!("too many attempts");
            return Err(DrawError::AllocationError);
        }
        let area = Rect { x: self.x, y: self.y, width, height };
        let last_color = self.last_color;
        Ok((PietCodeBufferEdit::new_slice(self, area), last_color))
    }

    fn advance_to(&mut self, to_x: usize) -> Result<(), DrawError> {
        println!("advance to {to_x} (from {})", self.x);
        let do_draw = self.last_color.is_some();
        if to_x < self.x {  // passed already
            let height = ROW_HEIGHT;
            self.reserve(height);
            let x = self.x;
            let y = self.y;
            if do_draw {
                PietCodeBufferEdit::new(self).draw_newline(x, y + 1)?;
                self.last_color = Some(Color::White);
            }
            self.x = 2;
            self.y += height;
        }
        let x = self.x;
        let y = self.y;
        let dist = to_x - x;
        if do_draw {
            PietCodeBufferEdit::new(self).draw_rect(
                x, y + 1, dist, 1, Color::White,
            )?;
            self.last_color = Some(Color::White);
        }
        self.x += dist;
        Ok(())
    }

    fn draw_jump(&mut self, x: usize, y0: usize, y1: usize) -> Result<(), DrawError> {
        println!("draw_jump: {x} {y0} {y1}");
        assert!(y0 < y1);
        let mut edit = PietCodeBufferEdit::new(self);
        edit.draw_rect(x, y0, 1, y1 - y0, Color::White)
    }

    fn draw_command(&mut self, cmd: Command) -> Result<(), DrawError> {
        let mut x = 0;
        let (mut edit, last_color) = self.allocate(3)?;
        let color = match last_color {
            Some(Color::White) | None => {
                edit.draw_pixel(0, 1, CONTROL_COLOR)?;
                x += 1;
                CONTROL_COLOR
            }
            Some(color) => color,
        };
        let color = color.next_for_command(cmd);
        edit.draw_pixel(x, 1, color)?;
        mem::drop(edit);
        self.x += x + 1;
        self.last_color = Some(color);
        Ok(())
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
            c => { return Err(DrawError::ColorMismatch(color, *c, (x, y))); }
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
            jump_xs: HashSet::new(),
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

    // TODO: can these be the same thing?
    let mut labels: HashMap<LabelId, (usize, usize)> = HashMap::new();
    let mut unmatched_jumps: HashMap<LabelId, (usize, usize)> = HashMap::new();

    let PietAsm { cmds, mut jump_counts } = asm;

    // wow i suddenly get why Rust could use a `try` block.
    let res = (|| -> Result<(), DrawError> {
        let (mut edit, _) = buffer.allocate(3)?;
        edit.draw_pixel(0, 0, CONTROL_COLOR)?;
        edit.draw_pixel(0, 1, CONTROL_COLOR)?;
        edit.draw_pixel(1, 1, CONTROL_COLOR)?;
        mem::drop(edit);
        buffer.x += 2;
        buffer.last_color = Some(CONTROL_COLOR);

        for cmd in cmds {
            println!("{cmd:?}");
            match cmd {
                AsmCommand::Label(label) => {
                    if let Some(&(dest, y0)) = unmatched_jumps.get(&label) {
                        buffer.advance_to(dest - 2)?;
                        draw_here!(buffer, b"
                             #
                           >...>
                            #..
                              #
                        ")?;
                        buffer.draw_jump(dest, y0 + 2, buffer.y + 1)?;
                        buffer.jump_xs.remove(&(buffer.x - 1));
                        buffer.jump_xs.insert(buffer.x - 2);
                        labels.insert(label, (dest - 1, buffer.y + 1));
                    }
                    else {
                        draw!(buffer, b"
                             #
                           >...>
                            #..
                              #
                        ")?;
                        labels.insert(label, (buffer.x - 2, buffer.y + 1));
                        buffer.jump_xs.insert(buffer.x - 2);
                    }
                    match jump_counts[label].checked_sub(1) {
                        Some(num) => { jump_counts[label] = num; }
                        None => {
                            let x = buffer.x - 2;
                            buffer.jump_xs.remove(&x);
                        }
                    }
                }
                AsmCommand::Jump(label) => {
                    // Label already exists
                    if let Some(&(dest, y0)) = labels.get(&label) {
                        buffer.advance_to(dest - 1)?;
                        draw_here!(buffer, b"
                             .
                           >...#
                            #..
                              #
                        ")?;
                        buffer.draw_jump(dest, y0, buffer.y + 1)?;
                    }
                    else {
                        return Err(DrawError::Todo);
                    }
                    match jump_counts[label].checked_sub(1) {
                        Some(num) => { jump_counts[label] = num; }
                        None => {
                            let x = buffer.x - 3;
                            buffer.jump_xs.remove(&x);
                        }
                    }
                }
                AsmCommand::JumpIf(label) => {
                    // connecting to an existing label
                    if let Some(&(dest, y0)) = labels.get(&label) {
                        buffer.advance_to(dest - 1)?;
                        let a = CONTROL_COLOR;
                        let b = a.next_for_command(Command::Pointer);
                        draw_here!(buffer, b"
                             .
                           >..ab>
                            #...
                               #
                        ", a, b)?;
                        buffer.draw_jump(dest, y0, buffer.y + 1)?;
                    }
                    // connecting to an existing jump
                    else if let Some((dest, y0)) = unmatched_jumps.remove(&label) {
                        buffer.advance_to(dest - 2)?;
                        let a = CONTROL_COLOR;
                        let b = a.next_for_command(Command::Pointer);
                        draw_here!(buffer, b"
                              .
                           >.abb>
                              b
                        ", a, b)?;
                        buffer.draw_jump(dest, y0 + 2, buffer.y + 1)?;
                        unmatched_jumps.insert(label, (dest, buffer.y + 1));
                    }
                    // first of their name
                    else {
                        // TODO: there's gotta be a nicer api with `draw_command`
                        let mut x = 0;
                        let (mut edit, last_color) = buffer.allocate(4)?;
                        let color = match last_color {
                            Some(Color::White) | None => {
                                edit.draw_pixel(0, 1, CONTROL_COLOR)?;
                                x += 1;
                                CONTROL_COLOR
                            }
                            Some(color) => color,
                        };
                        let color = color.next_for_command(Command::Pointer);
                        edit.draw_pixel(x, 1, color)?;
                        edit.draw_pixel(x, 2, color)?;
                        edit.draw_pixel(x + 1, 1, color)?;
                        mem::drop(edit);
                        buffer.jump_xs.insert(buffer.x + x);
                        let key = (buffer.x + x, buffer.y + 1);
                        unmatched_jumps.insert(label, key);
                        buffer.x += x + 2;
                        buffer.last_color = Some(color);
                    }
                    match jump_counts[label].checked_sub(1) {
                        Some(num) => { jump_counts[label] = num; }
                        None => {
                            let x = buffer.x - 3;
                            buffer.jump_xs.remove(&x);
                        }
                    }
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
                    let (mut edit, _) = buffer.allocate(width + 5)?;
                    let mut x = 0;
                    if has_color {
                        // println!("drawin intro");
                        edit.draw_pixel(0, 1, Color::White)?;
                        x = 1;
                    }
                    edit.draw_rect(x, 1, width, ROW_FILL_HEIGHT, CONTROL_COLOR)?;
                    x += width;
                    if extra > 0 {
                        edit.draw_rect(x, 1, 1, extra, CONTROL_COLOR)?;
                        x += 1;
                    }
                    edit.draw_pixel(x, 1, CONTROL_COLOR)?;
                    let color = CONTROL_COLOR.next_for_command(Command::Push);
                    edit.draw_pixel(x + 1, 1, color)?;
                    mem::drop(edit);
                    buffer.x += x + 2;
                    buffer.last_color = Some(color);
                }
                AsmCommand::Pop | AsmCommand::Add | AsmCommand::Subtract | AsmCommand::Multiply |
                AsmCommand::Divide | AsmCommand::Mod | AsmCommand::Not | AsmCommand::Greater |
                AsmCommand::Duplicate | AsmCommand::Roll | AsmCommand::InNum | AsmCommand::InChar |
                AsmCommand::OutNum | AsmCommand::OutChar => {
                    let cmd: Command = cmd.try_into().unwrap();
                    buffer.draw_command(cmd)?;
                }
                AsmCommand::Stop => {
                    draw!(buffer, b"
                          #
                       >..a#
                        #aa#
                         ##
                    ", CONTROL_COLOR)?;
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
