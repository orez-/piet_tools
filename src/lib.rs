use std::cmp::Reverse;
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;
use image::{self, DynamicImage, GenericImageView, Rgb, Rgba};
use itertools::iproduct;
use num_bigint::BigInt;
use num_derive::FromPrimitive;
use num_integer::Integer;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};

pub trait GetAllEqualIterator<T>: Iterator<Item = T> {
    fn get_all_equal(&mut self) -> Option<T>
        where Self: Sized,
              Self::Item: PartialEq
    {
        let a = self.next()?;
        self.all(|x| a == x).then(|| a)
    }
}

impl<T, I: Iterator<Item = T>> GetAllEqualIterator<T> for I {}

type Coord = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hue {
    Red = 0,
    Yellow = 1,
    Green = 2,
    Cyan = 3,
    Blue = 4,
    Magenta = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
            Color::Color(h, l) => { (h, l) }
            Color::White => { return Command::Noop; }
            Color::Black => { panic!(); }
            Color::Other => { panic!(); }
        };
        let hue_step = (next_hue as i32 - hue as i32).rem_euclid(6);
        let light_step = (next_lightness as i32 - lightness as i32).rem_euclid(3);
        FromPrimitive::from_i32(light_step + hue_step * 3).unwrap()
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
            _ => Color::Other
        }
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

    pub fn execute(&self) -> PietRun<'_> {
        PietRun::new(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CodelChoice { Left, Right }

pub struct CodelRegion {
    pub(crate) color: Color,
    pub(crate) region: HashSet<Coord>,
}

impl CodelRegion {
    fn new(region: HashSet<Coord>, color: Color) -> Self {
        CodelRegion {
            color,
            region,
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

pub struct PietRun<'a> {
    instruction_pointer: InstructionPointer,
    pos: Coord,
    code: &'a PietCode,
    stack: Vec<BigInt>,
}

impl<'a> PietRun<'a> {
    fn new(code: &'a PietCode) -> Self {
        Self {
            code,
            instruction_pointer: InstructionPointer(Direction::Right, CodelChoice::Left),
            pos: (0, 0),
            stack: Vec::new(),
        }
    }

    // Fetch the next position to move to.
    fn walk_color(&mut self) -> Option<(CodelRegion, Coord, Color)> {
        let (x, y) = self.pos;
        let region = self.code.region_at(x, y).unwrap();

        for _ in 0..4 {
            let coord @ (x, y) = region.exit_to(self.instruction_pointer);
            match self.code.at(x, y) {
                None | Some(Color::Black) => (),
                Some(Color::Other) => { panic!(); }
                Some(color) => { return Some((region, coord, color)); }
            }
            self.instruction_pointer.flip();

            let coord @ (x, y) = region.exit_to(self.instruction_pointer);
            match self.code.at(x, y) {
                None | Some(Color::Black) => (),
                Some(Color::Other) => { panic!(); }
                Some(color) => { return Some((region, coord, color)); }
            }
            self.instruction_pointer.rotate();
        }
        None
    }

    fn walk_white(&mut self) -> Option<(Coord, Color)> {
        let mut seen = HashSet::new();
        let (mut x, mut y) = self.pos;
        let mut nx;
        let mut ny;
        while seen.insert((self.pos, self.instruction_pointer)) {
            let InstructionPointer(dir, _) = self.instruction_pointer;
            let (dx, dy) = dir.to_delta();
            while let Some(color) = {
                nx = x.wrapping_add(dx);
                ny = y.wrapping_add(dy);
                self.code.at(nx, ny)
            } {
                match color {
                    Color::Black => { break; }
                    Color::Other => { panic!(); }
                    Color::White => {
                        x = nx;
                        y = ny;
                    }
                    color => { return Some(((nx, ny), color)); }
                }
            }
            self.instruction_pointer.rotate();
        }
        None
    }

    fn pop2(&mut self) -> Option<(BigInt, BigInt)> {
        if self.stack.len() < 2 {
            return None;
        }
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        Some((a, b))
    }

    fn run_command(&mut self, command: Command, value: BigInt) -> Option<()> {
        match command {
            Command::Noop => {}
            Command::Push => {
                self.stack.push(value);
            }
            Command::Pop => { self.stack.pop()?; }
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
                let (a, b) = self.pop2()?;
                self.stack.push(a.div_floor(&b));
            }
            Command::Mod => {
                let (a, b) = self.pop2()?;
                self.stack.push(a.mod_floor(&b));
            }
            Command::Not => {
                let num = self.stack.pop()?;
                let zero = BigInt::zero();
                self.stack.push(if num == zero { BigInt::one() } else { zero });
            }
            Command::Greater => {
                let (a, b) = self.pop2()?;
                self.stack.push(if a > b { BigInt::one() } else { BigInt::zero() });
            }
            Command::Pointer => {
                let spin = self.stack.pop()?;
                let spin = spin.mod_floor(&(4.into())).to_u8().unwrap();
                for _ in 0..spin {
                    self.instruction_pointer.rotate();
                }
            }
            Command::Switch => {
                let swap = self.stack.pop()?;
                if swap % 2 != BigInt::zero() {
                    self.instruction_pointer.flip();
                }
            }
            Command::Duplicate => {
                let top = self.stack.last()?.clone();
                self.stack.push(top);
            }
            Command::Roll => {
                let (dive, roll) = self.pop2()?;
                if dive < BigInt::zero() { panic!(); }  // TODO: exit without popping
                let roll = roll.div_floor(&dive).to_usize().unwrap();
                let dive = dive.to_usize().unwrap();
                let start = self.stack.len() - dive;
                self.stack[start..].rotate_right(roll);
            }
            Command::InNum => { todo!(); }
            Command::InChar => { todo!(); }
            Command::OutNum => {
                let num = self.stack.pop()?;
                print!("{num}");
            }
            Command::OutChar => {
                let num = self.stack.pop()?;
                let chr = num.to_u8().unwrap() as char;  // TODO: 👀
                print!("{chr}");
            }
        }
        Some(())
    }

    // TODO: bool sucks
    pub fn step(&mut self) -> bool {
        let (x, y) = self.pos;
        let color = self.code.at(x, y).unwrap();
        match color {
            Color::White => {
                match self.walk_white() {
                    Some((coord, color)) => {
                        eprintln!("(White -> {color:?})");
                        self.pos = coord;
                        true
                    }
                    None => false,
                }
            }
            Color::Color(..) => {
                let (region, (x, y), next_color) = if let Some(v) = self.walk_color() { v }
                    else { return false; };
                let command = region.color.step_to(next_color);
                let value = region.value();
                eprintln!(
                    "({:?} ({}) -> {:?}) = {command:?}",
                    region.color,
                    value,
                    next_color,
                );
                self.run_command(command, value);
                self.pos = (x, y);
                true
            }
            Color::Other => { panic!(); }  // TODO
            Color::Black => { panic!(); }
        }
    }

    pub fn run(&mut self) {
        while self.step() {}
    }
}

pub fn load(filename: &str, codel_size: u32) -> Result<PietCode, String>  {
    let img = image::open(filename).map_err(|e| e.to_string())?;
    to_codels(img, codel_size)
}

fn to_codels(img: DynamicImage, codel_size: u32) -> Result<PietCode, String> {
    let (w, h) = img.dimensions();
    if w % codel_size != 0 || h % codel_size != 0 {
        return Err("invalid dimensions".to_string());
    }
    let width = w / codel_size;
    let height = h / codel_size;
    match img {
        DynamicImage::ImageRgb8(img) => {
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
                code
            })
        }
        // DynamicImage::ImageRgba8(img) => img,
        _ => { Err("unsupported image type".to_string()) }
    }
}
