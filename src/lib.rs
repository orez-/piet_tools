use image::{self, DynamicImage, GenericImageView, ImageResult, Rgb, Rgba, RgbImage};
use itertools::iproduct;
use num_bigint::BigInt;
use num_derive::FromPrimitive;
use num_integer::Integer;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use std::cmp::Reverse;
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;

pub mod asm;

pub trait GetAllEqualIterator<T>: Iterator<Item = T> {
    fn get_all_equal(&mut self) -> Option<T>
    where
        Self: Sized,
        Self::Item: PartialEq,
    {
        let a = self.next()?;
        self.all(|x| a == x).then(|| a)
    }
}

impl<T, I: Iterator<Item = T>> GetAllEqualIterator<T> for I {}

type Coord = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(FromPrimitive)]
enum Hue {
    Red = 0,
    Yellow = 1,
    Green = 2,
    Cyan = 3,
    Blue = 4,
    Magenta = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(FromPrimitive)]
enum Lightness {
    Light = 0,
    Normal = 1,
    Dark = 2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Color(Hue, Lightness),
    Black,
    White,
    Other,
}

#[allow(non_upper_case_globals)]
impl Color {
    const LightRed: Color = Color::Color(Hue::Red, Lightness::Light);
    const LightYellow: Color = Color::Color(Hue::Yellow, Lightness::Light);
    const LightGreen: Color = Color::Color(Hue::Green, Lightness::Light);
    const LightCyan: Color = Color::Color(Hue::Cyan, Lightness::Light);
    const LightBlue: Color = Color::Color(Hue::Blue, Lightness::Light);
    const LightMagenta: Color = Color::Color(Hue::Magenta, Lightness::Light);

    const Red: Color = Color::Color(Hue::Red, Lightness::Normal);
    const Yellow: Color = Color::Color(Hue::Yellow, Lightness::Normal);
    const Green: Color = Color::Color(Hue::Green, Lightness::Normal);
    const Cyan: Color = Color::Color(Hue::Cyan, Lightness::Normal);
    const Blue: Color = Color::Color(Hue::Blue, Lightness::Normal);
    const Magenta: Color = Color::Color(Hue::Magenta, Lightness::Normal);

    const DarkRed: Color = Color::Color(Hue::Red, Lightness::Dark);
    const DarkYellow: Color = Color::Color(Hue::Yellow, Lightness::Dark);
    const DarkGreen: Color = Color::Color(Hue::Green, Lightness::Dark);
    const DarkCyan: Color = Color::Color(Hue::Cyan, Lightness::Dark);
    const DarkBlue: Color = Color::Color(Hue::Blue, Lightness::Dark);
    const DarkMagenta: Color = Color::Color(Hue::Magenta, Lightness::Dark);
}

impl Color {
    fn step_to(self, next: Color) -> Command {
        let (hue, lightness) = match self {
            Color::Color(h, l) => (h, l),
            Color::White => { return Command::Noop; }
            Color::Black => { panic!(); }
            Color::Other => { panic!(); }
        };
        let (next_hue, next_lightness) = match next {
            Color::Color(h, l) => (h, l),
            Color::White => { return Command::Noop; }
            Color::Black => { panic!(); }
            Color::Other => { panic!(); }
        };
        let hue_step = (next_hue as i32 - hue as i32).rem_euclid(6);
        let light_step = (next_lightness as i32 - lightness as i32).rem_euclid(3);
        FromPrimitive::from_i32(light_step + hue_step * 3).unwrap()
    }

    /// Reverse of `step_to`.
    fn next_for_command(self, command: Command) -> Color {
        let (hue, lightness) = match self {
            Color::Color(h, l) => (h as i32, l as i32),
            _ => { panic!(); }
        };
        let command = command as i32;
        let dlight = command % 3;
        let dhue = command / 3;
        let hue = FromPrimitive::from_i32((hue + dhue) % 6).unwrap();
        let lightness = FromPrimitive::from_i32((lightness + dlight) % 3).unwrap();
        Color::Color(hue, lightness)
    }
}

#[derive(FromPrimitive, Debug)]
enum Command {
    Noop = 0,
    Push = 1,
    Pop = 2,
    Add = 3,
    Subtract = 4,
    Multiply = 5,
    Divide = 6,
    Mod = 7,
    Not = 8,
    Greater = 9,
    Pointer = 10,
    Switch = 11,
    Duplicate = 12,
    Roll = 13,
    InNum = 14,
    InChar = 15,
    OutNum = 16,
    OutChar = 17,
}

impl fmt::Debug for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match *self {
            Color::LightRed => "LightRed",
            Color::LightYellow => "LightYellow",
            Color::LightGreen => "LightGreen",
            Color::LightCyan => "LightCyan",
            Color::LightBlue => "LightBlue",
            Color::LightMagenta => "LightMagenta",
            Color::Red => "Red",
            Color::Yellow => "Yellow",
            Color::Green => "Green",
            Color::Cyan => "Cyan",
            Color::Blue => "Blue",
            Color::Magenta => "Magenta",
            Color::DarkRed => "DarkRed",
            Color::DarkYellow => "DarkYellow",
            Color::DarkGreen => "DarkGreen",
            Color::DarkCyan => "DarkCyan",
            Color::DarkBlue => "DarkBlue",
            Color::DarkMagenta => "DarkMagenta",
            Color::Black => "Black",
            Color::White => "White",
            Color::Other => "Other",
        })
    }
}

impl From<Rgb<u8>> for Color {
    fn from(pixel: Rgb<u8>) -> Color {
        match pixel {
            Rgb([0xFF, 0xFF, 0xFF]) => Color::White,
            Rgb([0x00, 0x00, 0x00]) => Color::Black,
            Rgb([0xFF, 0xC0, 0xC0]) => Color::LightRed,
            Rgb([0xFF, 0x00, 0x00]) => Color::Red,
            Rgb([0xC0, 0x00, 0x00]) => Color::DarkRed,
            Rgb([0xFF, 0xFF, 0xC0]) => Color::LightYellow,
            Rgb([0xFF, 0xFF, 0x00]) => Color::Yellow,
            Rgb([0xC0, 0xC0, 0x00]) => Color::DarkYellow,
            Rgb([0xC0, 0xFF, 0xC0]) => Color::LightGreen,
            Rgb([0x00, 0xFF, 0x00]) => Color::Green,
            Rgb([0x00, 0xC0, 0x00]) => Color::DarkGreen,
            Rgb([0xC0, 0xFF, 0xFF]) => Color::LightCyan,
            Rgb([0x00, 0xFF, 0xFF]) => Color::Cyan,
            Rgb([0x00, 0xC0, 0xC0]) => Color::DarkCyan,
            Rgb([0xC0, 0xC0, 0xFF]) => Color::LightBlue,
            Rgb([0x00, 0x00, 0xFF]) => Color::Blue,
            Rgb([0x00, 0x00, 0xC0]) => Color::DarkBlue,
            Rgb([0xFF, 0xC0, 0xFF]) => Color::LightMagenta,
            Rgb([0xFF, 0x00, 0xFF]) => Color::Magenta,
            Rgb([0xC0, 0x00, 0xC0]) => Color::DarkMagenta,
            _ => Color::Other,
        }
    }
}

impl TryFrom<Color> for Rgb<u8> {
    type Error = ();

    fn try_from(pixel: Color) -> Result<Rgb<u8>, ()> {
        Ok(match pixel {
            Color::White => Rgb([0xFF, 0xFF, 0xFF]),
            Color::Black => Rgb([0x00, 0x00, 0x00]),
            Color::LightRed => Rgb([0xFF, 0xC0, 0xC0]),
            Color::Red => Rgb([0xFF, 0x00, 0x00]),
            Color::DarkRed => Rgb([0xC0, 0x00, 0x00]),
            Color::LightYellow => Rgb([0xFF, 0xFF, 0xC0]),
            Color::Yellow => Rgb([0xFF, 0xFF, 0x00]),
            Color::DarkYellow => Rgb([0xC0, 0xC0, 0x00]),
            Color::LightGreen => Rgb([0xC0, 0xFF, 0xC0]),
            Color::Green => Rgb([0x00, 0xFF, 0x00]),
            Color::DarkGreen => Rgb([0x00, 0xC0, 0x00]),
            Color::LightCyan => Rgb([0xC0, 0xFF, 0xFF]),
            Color::Cyan => Rgb([0x00, 0xFF, 0xFF]),
            Color::DarkCyan => Rgb([0x00, 0xC0, 0xC0]),
            Color::LightBlue => Rgb([0xC0, 0xC0, 0xFF]),
            Color::Blue => Rgb([0x00, 0x00, 0xFF]),
            Color::DarkBlue => Rgb([0x00, 0x00, 0xC0]),
            Color::LightMagenta => Rgb([0xFF, 0xC0, 0xFF]),
            Color::Magenta => Rgb([0xFF, 0x00, 0xFF]),
            Color::DarkMagenta => Rgb([0xC0, 0x00, 0xC0]),
            Color::Other => { return Err(()); }
        })
    }
}

impl From<Rgba<u8>> for Color {
    fn from(pixel: Rgba<u8>) -> Color {
        let Rgba([r, g, b, a]) = pixel;
        if a != 0xFF {
            return Color::Other;
        }
        Rgb([r, g, b]).into()
    }
}

#[derive(Debug)]
pub struct PietCode {
    width: usize,
    height: usize,
    code: Vec<Color>,
}

impl PietCode {
    fn codels(&self) -> impl Iterator<Item = (usize, usize, Color)> + '_ {
        self.code.iter().enumerate().map(|(i, c)| {
            let x = i % self.width;
            let y = i / self.width;
            (x, y, *c)
        })
    }

    fn at(&self, x: usize, y: usize) -> Option<Color> {
        if x >= self.width || y >= self.height { return None; }
        Some(self.code[x + y * self.width])
    }

    fn region_at(&self, x: usize, y: usize) -> Option<CodelRegion> {
        let color = self.at(x, y)?;
        let mut seen = HashSet::new();
        seen.insert((x, y));
        let mut queue = VecDeque::new();
        queue.push_back((x, y));
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in [(0, 1), (1, 0), (0, usize::MAX), (usize::MAX, 0)] {
                let nx = x.wrapping_add(dx);
                let ny = y.wrapping_add(dy);
                if self.at(nx, ny).map_or(true, |n| n != color) { continue; }
                if !seen.insert((nx, ny)) { continue; }
                queue.push_back((nx, ny));
            }
        }
        Some(CodelRegion::new(seen, color))
    }

    pub fn execute(&self) -> PietRunner<'_> {
        PietRunner::new(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Direction {
    Right,
    Down,
    Left,
    Up,
}

impl Direction {
    fn to_delta(self) -> Coord {
        match self {
            Direction::Right => (1, 0),
            Direction::Down => (0, 1),
            Direction::Left => (usize::MAX, 0),
            Direction::Up => (0, usize::MAX),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum CodelChoice { Left, Right }

pub struct CodelRegion {
    pub(crate) color: Color,
    pub(crate) region: HashSet<Coord>,
}

impl CodelRegion {
    fn new(region: HashSet<Coord>, color: Color) -> Self {
        CodelRegion { color, region }
    }

    fn value(&self) -> BigInt {
        BigInt::from(self.region.len())
    }

    fn exit_to(&self, ip: InstructionPointer) -> Coord {
        let InstructionPointer(dp, cc) = ip;
        match (dp, cc) {
            (Direction::Right, CodelChoice::Left) => {
                let (x, y) = *self.region.iter().max_by_key(|(x, y)| (x, Reverse(y))).unwrap();
                (x + 1, y)
            }
            (Direction::Right, CodelChoice::Right) => {
                let (x, y) = *self.region.iter().max_by_key(|(x, y)| (x, y)).unwrap();
                (x + 1, y)
            }
            (Direction::Down, CodelChoice::Left) => {
                let (x, y) = *self.region.iter().max_by_key(|(x, y)| (y, x)).unwrap();
                (x, y + 1)
            }
            (Direction::Down, CodelChoice::Right) => {
                let (x, y) = *self.region.iter().max_by_key(|(x, y)| (y, Reverse(x))).unwrap();
                (x, y + 1)
            }
            (Direction::Left, CodelChoice::Left) => {
                let (x, y) = *self.region.iter().min_by_key(|(x, y)| (x, Reverse(y))).unwrap();
                (x.wrapping_sub(1), y)
            }
            (Direction::Left, CodelChoice::Right) => {
                let (x, y) = *self.region.iter().min_by_key(|(x, y)| (x, y)).unwrap();
                (x.wrapping_sub(1), y)
            }
            (Direction::Up, CodelChoice::Left) => {
                let (x, y) = *self.region.iter().min_by_key(|(x, y)| (y, x)).unwrap();
                (x, y.wrapping_sub(1))
            }
            (Direction::Up, CodelChoice::Right) => {
                let (x, y) = *self.region.iter().min_by_key(|(x, y)| (y, Reverse(x))).unwrap();
                (x, y.wrapping_sub(1))
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct InstructionPointer(Direction, CodelChoice);

impl InstructionPointer {
    fn flip(&mut self) {
        self.1 = match self.1 {
            CodelChoice::Left => CodelChoice::Right,
            CodelChoice::Right => CodelChoice::Left,
        }
    }

    fn rotate(&mut self) {
        self.0 = match self.0 {
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
            Direction::Up => Direction::Right,
        };
    }
}

impl Default for InstructionPointer {
    fn default() -> Self {
        InstructionPointer(Direction::Right, CodelChoice::Left)
    }
}

#[derive(Debug)]
enum ExecutionError {
    NotEnoughStack(usize, usize),
    NegativeRoll(BigInt),
    IntegerOverflow,
    DivisionByZero,
    IoError(std::io::Error),
    EncodeError(BigInt),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ExecutionError::*;

        match self {
            NotEnoughStack(requested, stack_len) => {
                write!(f, "insufficient stack length ({stack_len}); expected at least {requested}")
            }
            NegativeRoll(num) => write!(f, "expected positive roll depth, not {num}"),
            IntegerOverflow => write!(f, "integer overflow"),
            IoError(e) => write!(f, "IO error: {e}"),
            DivisionByZero => write!(f, "division by zero"),
            EncodeError(num) => write!(f, "can't encode integer '{num}' as character"),
        }
    }
}

#[derive(Default)]
pub struct PietVM {
    instruction_pointer: InstructionPointer,
    pos: Coord,
    stack: Vec<BigInt>,
}

impl PietVM {
    fn new() -> Self {
        Self::default()
    }

    // Fetch the next position to move to.
    fn walk_color(&mut self, code: &PietCode) -> Option<(CodelRegion, Coord, Color)> {
        let (x, y) = self.pos;
        let region = code.region_at(x, y).unwrap();

        for _ in 0..4 {
            let coord @ (x, y) = region.exit_to(self.instruction_pointer);
            match code.at(x, y) {
                None | Some(Color::Black) => (),
                Some(Color::Other) => { panic!(); }
                Some(color) => { return Some((region, coord, color)); }
            }
            self.instruction_pointer.flip();

            let coord @ (x, y) = region.exit_to(self.instruction_pointer);
            match code.at(x, y) {
                None | Some(Color::Black) => (),
                Some(Color::Other) => { panic!(); }
                Some(color) => { return Some((region, coord, color)); }
            }
            self.instruction_pointer.rotate();
        }
        None
    }

    fn walk_white(&mut self, code: &PietCode) -> Option<(Coord, Color)> {
        let mut seen = HashSet::new();
        let mut nx;
        let mut ny;
        while seen.insert((self.pos, self.instruction_pointer)) {
            let InstructionPointer(dir, _) = self.instruction_pointer;
            let (dx, dy) = dir.to_delta();
            while let Some(color) = {
                let (x, y) = self.pos;
                nx = x.wrapping_add(dx);
                ny = y.wrapping_add(dy);
                code.at(nx, ny)
            } {
                match color {
                    Color::Black => { break; }
                    Color::Other => { panic!("invalid color while sliding"); }
                    Color::White => { self.pos = (nx, ny); }
                    color => { return Some(((nx, ny), color)); }
                }
            }
            self.instruction_pointer.flip();
            self.instruction_pointer.rotate();
        }
        None
    }

    fn pop1(&mut self) -> Result<BigInt, ExecutionError> {
        self.stack.pop()
            .ok_or_else(|| ExecutionError::NotEnoughStack(1, 0))
    }

    fn pop2(&mut self) -> Result<(BigInt, BigInt), ExecutionError> {
        if self.stack.len() < 2 {
            return Err(ExecutionError::NotEnoughStack(2, self.stack.len()));
        }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        Ok((a, b))
    }

    fn last1(&self) -> Result<&BigInt, ExecutionError> {
        self.stack.last()
            .ok_or_else(|| ExecutionError::NotEnoughStack(1, 0))
    }

    fn last2(&self) -> Result<(&BigInt, &BigInt), ExecutionError> {
        let len = self.stack.len();
        if len < 2 { return Err(ExecutionError::NotEnoughStack(2, self.stack.len())); }
        if let [d, r] = &self.stack[len - 2..] { Ok((d, r)) }
            else { unreachable!(); }  // rust you dingus
    }

    fn run_command(&mut self, command: Command, value: BigInt) -> Result<(), ExecutionError> {
        match command {
            Command::Noop => {}
            Command::Push => {
                self.stack.push(value);
            }
            Command::Pop => { self.pop1()?; }
            Command::Add => {
                let (a, b) = self.pop2()?;
                self.stack.push(a + b);
            }
            Command::Subtract => {
                let (a, b) = self.pop2()?;
                self.stack.push(a - b);
            }
            Command::Multiply => {
                let (a, b) = self.pop2()?;
                self.stack.push(a * b);
            }
            Command::Divide => {
                let (_, b) = self.last2()?;
                if b == &BigInt::zero() {
                    return Err(ExecutionError::DivisionByZero);
                }
                let (a, b) = self.pop2()?;
                self.stack.push(a.div_floor(&b));
            }
            Command::Mod => {
                let (_, b) = self.last2()?;
                if b == &BigInt::zero() {
                    return Err(ExecutionError::DivisionByZero);
                }
                let (a, b) = self.pop2()?;
                self.stack.push(a.mod_floor(&b));
            }
            Command::Not => {
                let num = self.pop1()?;
                let zero = BigInt::zero();
                self.stack.push(if num == zero { BigInt::one() } else { zero });
            }
            Command::Greater => {
                let (a, b) = self.pop2()?;
                self.stack.push(if a > b { BigInt::one() } else { BigInt::zero() });
            }
            Command::Pointer => {
                let spin = self.pop1()?;
                let spin = spin.mod_floor(&(4.into())).to_u8().unwrap();
                for _ in 0..spin {
                    self.instruction_pointer.rotate();
                }
            }
            Command::Switch => {
                let swap = self.pop1()?;
                if swap % 2 != BigInt::zero() {
                    self.instruction_pointer.flip();
                }
            }
            Command::Duplicate => {
                let top = self.last1()?.clone();
                self.stack.push(top);
            }
            Command::Roll => {
                let (dive, roll) = self.last2()?;
                if dive <= &BigInt::zero() {
                    return Err(ExecutionError::NegativeRoll(dive.clone()));
                }
                let roll = roll.mod_floor(&dive).to_usize()
                    .ok_or(ExecutionError::IntegerOverflow)?;
                let dive = dive.to_usize()
                    .ok_or(ExecutionError::IntegerOverflow)?;
                let len = self.stack.len() - 2;
                let start = len.checked_sub(dive)
                    .ok_or_else(|| ExecutionError::NotEnoughStack(len, dive))?;
                self.pop2()?;
                self.stack[start..].rotate_right(roll);
            }
            Command::InNum => { todo!(); }
            Command::InChar => {
                // TODO: don't make this so stdin specific
                use std::io::{self, Read};

                let stdin = io::stdin();
                let buf: &mut [u8] = &mut [0];
                stdin.lock().read_exact(buf).map_err(|e| ExecutionError::IoError(e))?;
                self.stack.push(BigInt::from(buf[0]));
            }
            Command::OutNum => {
                let num = self.pop1()?;
                print!("{num}");
            }
            Command::OutChar => {
                let num = self.pop1()?;
                let chr = num.to_u8() // TODO: non-ascii? 👀
                    .ok_or_else(|| ExecutionError::EncodeError(num))?
                    as char;
                print!("{chr}");
            }
        }
        Ok(())
    }

    // TODO: bool sucks
    pub fn step(&mut self, code: &PietCode) -> bool {
        let (x, y) = self.pos;
        let color = code.at(x, y).unwrap();
        eprintln!("{:?}", self.stack);
        match color {
            Color::White => match self.walk_white(code) {
                Some((coord, color)) => {
                    eprintln!("(White -> {color:?}) [{coord:?}]");
                    self.pos = coord;
                    true
                }
                None => false,
            },
            Color::Color(..) => {
                let (region, coord, next_color) = if let Some(v) = self.walk_color(code) { v }
                    else { return false; };
                let command = region.color.step_to(next_color);
                let value = region.value();
                eprintln!(
                    "({:?} ({}) -> {:?}) [{coord:?}] = {command:?}",
                    region.color, value, next_color,
                );
                if let Err(err) = self.run_command(command, value) {
                    eprintln!("Skipping command: {err}");
                }
                self.pos = coord;
                true
            }
            Color::Other => { panic!(); }  // TODO
            Color::Black => { panic!(); }
        }
    }
}

pub struct PietRunner<'a> {
    code: &'a PietCode,
    vm: PietVM,
}

impl<'a> PietRunner<'a> {
    fn new(code: &'a PietCode) -> Self {
        PietRunner {
            vm: PietVM::new(),
            code,
        }
    }

    pub fn step(&mut self) -> bool {
        self.vm.step(self.code)
    }

    pub fn run(&mut self) {
        while self.step() {}
    }
}

pub fn load(filename: &str, codel_size: u32) -> Result<PietCode, String> {
    let img = image::open(filename).map_err(|e| e.to_string())?;
    to_codels(img, codel_size)
}

pub fn save(code: &PietCode, filename: &str, codel_size: u32) -> ImageResult<()> {
    let img = to_image(code, codel_size);
    img.save(filename)
}

fn to_codels(img: DynamicImage, codel_size: u32) -> Result<PietCode, String> {
    let (w, h) = img.dimensions();
    if w % codel_size != 0 || h % codel_size != 0 {
        return Err("invalid dimensions".to_string());
    }
    let width = w / codel_size;
    let height = h / codel_size;
    let img = img.into_rgb8();
    let code = iproduct!(0..height, 0..width)
        .map(|(y, x)| {
            img.view(x * codel_size, y * codel_size, codel_size, codel_size)
                .pixels()
                .map(|(_, _, px)| px)
                .get_all_equal()
                // TODO: options to:
                // - error on None
                // - error on Other
                // - black on Other
                .map_or(Color::Other, |px| px.into())
        })
        .collect();
    Ok(PietCode {
        width: width as usize,
        height: height as usize,
        code,
    })
}

fn to_image(code: &PietCode, codel_size: u32) -> RgbImage {
    // TODO: options to handle Other pixels.
    // Currently hardcoded to a nice purple
    const OTHER_COLOR: Rgb<u8> = Rgb([0x73, 0x26, 0xb1]);
    let PietCode { width, height, .. } = code;
    let mut img = RgbImage::new(
        *width as u32 * codel_size,
        *height as u32 * codel_size,
    );
    for (x, y, codel) in code.codels() {
        let img_x = x as u32 * codel_size;
        let img_y = y as u32 * codel_size;
        let color = codel.try_into().unwrap_or(OTHER_COLOR);

        for dx in 0..codel_size {
            for dy in 0..codel_size {
                img.put_pixel(img_x + dx, img_y + dy, color);
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_stack(nums: &[i32]) -> Vec<BigInt> {
        nums.into_iter().map(|e| (*e).into()).collect()
    }

    #[test]
    fn test_roll() {
        let mut vm = PietVM { stack: to_stack(&[4, 5, 6, 7, 8, 9, 3, 2]), ..Default::default() };
        vm.run_command(Command::Roll, BigInt::zero()).unwrap();
        assert_eq!(vm.stack, to_stack(&[4, 5, 6, 8, 9, 7]));
    }

    #[test]
    fn test_div_zero() {
        let mut vm = PietVM { stack: to_stack(&[4, 0]), ..Default::default() };
        let result = vm.run_command(Command::Divide, BigInt::zero());
        assert!(matches!(result, Err(ExecutionError::DivisionByZero)));
        assert_eq!(vm.stack, to_stack(&[4, 0]));
    }

    /// If we're going to divide by zero but have too few arguments on the stack,
    /// prefer the "too few arguments" message
    #[test]
    fn test_div_zero_too_few() {
        let mut vm = PietVM { stack: to_stack(&[0]), ..Default::default() };
        let result = vm.run_command(Command::Divide, BigInt::zero());
        assert!(matches!(result, Err(ExecutionError::NotEnoughStack(2, 1))));
        assert_eq!(vm.stack, to_stack(&[0]));
    }

    #[test]
    fn test_mod_zero() {
        let mut vm = PietVM { stack: to_stack(&[4, 0]), ..Default::default() };
        let result = vm.run_command(Command::Mod, BigInt::zero());
        assert!(matches!(result, Err(ExecutionError::DivisionByZero)));
        assert_eq!(vm.stack, to_stack(&[4, 0]));
    }

    /// If we're going to modulo by zero but have too few arguments on the stack,
    /// prefer the "too few arguments" message
    #[test]
    fn test_mod_zero_too_few() {
        let mut vm = PietVM { stack: to_stack(&[0]), ..Default::default() };
        let result = vm.run_command(Command::Mod, BigInt::zero());
        assert!(matches!(result, Err(ExecutionError::NotEnoughStack(2, 1))));
        assert_eq!(vm.stack, to_stack(&[0]));
    }

    /// Exercises sliding, slide cycle detection, and slide CC maintenance
    #[test]
    fn test_slide() {
        let code = load("test_imgs/test_slide.png", 1).unwrap();
        let mut runner = code.execute();
        runner.run();
        assert_eq!(runner.vm.stack, to_stack(&[8]));
    }
}
