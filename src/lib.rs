use std::cmp::Reverse;
use std::collections::{HashSet, VecDeque};
use std::fmt;
use image::{self, DynamicImage, GenericImageView, Rgb, Rgba};
use itertools::iproduct;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

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
type PietInt = i128;

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
        let (hue, lightness) = if let Color::Color(h, l) = self { (h, l) }
            else { return Command::Noop; };
        let (next_hue, next_lightness) = if let Color::Color(h, l) = next { (h, l) }
            else { return Command::Noop; };
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
        if color == Color::Black { return None; }
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

    pub fn execute<'a>(&'a self) -> PietRun<'a> {
        PietRun::new(self)
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Right,
    Down,
    Left,
    Up,
}

#[derive(Clone, Copy)]
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

    fn value(&self) -> PietInt {
        self.region.len() as PietInt
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

#[derive(Clone, Copy)]
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
    region: CodelRegion,
    code: &'a PietCode,
    stack: Vec<PietInt>,
}

impl<'a> PietRun<'a> {
    fn new(code: &'a PietCode) -> Self {
        let instruction_pointer = InstructionPointer(Direction::Right, CodelChoice::Left);
        let region = code.region_at(0, 0).unwrap();  // TODO: 👀
        Self { instruction_pointer, code, region, stack: Vec::new() }
    }

    fn next_region(&mut self) -> Option<CodelRegion> {
        for _ in 0..4 {
            let (x, y) = self.region.exit_to(self.instruction_pointer);
            if let Some(region) = self.code.region_at(x, y) {
                return Some(region);
            }
            self.instruction_pointer.flip();

            let (x, y) = self.region.exit_to(self.instruction_pointer);
            if let Some(region) = self.code.region_at(x, y) {
                return Some(region);
            }
            // TODO: unclear from the docs if we also flip, or not.
            // doublecheck with other implementations
            // self.instruction_pointer.flip();
            self.instruction_pointer.rotate();
        }
        None
    }

    fn pop2(&mut self) -> Option<(PietInt, PietInt)> {
        if self.stack.len() < 2 {
            return None;
        }
        let b = self.stack.pop()?;
        let a = self.stack.pop()?;
        Some((a, b))
    }

    fn run_command(&mut self, command: Command) -> Option<()> {
        match command {
            Command::Noop => {}
            Command::Push => {
                self.stack.push(self.region.value());
            }
            Command::Pop => { todo!(); }
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
            Command::Divide => { todo!(); }
            Command::Mod => { todo!(); }
            Command::Not => { todo!(); }
            Command::Greater => { todo!(); }
            Command::Pointer => { todo!(); }
            Command::Switch => { todo!(); }
            Command::Duplicate => {
                let top = *self.stack.last()?;
                self.stack.push(top);
            }
            Command::Roll => { todo!(); }
            Command::InNum => { todo!(); }
            Command::InChar => { todo!(); }
            Command::OutNum => { todo!(); }
            Command::OutChar => {
                let num = self.stack.pop()?;
                let chr = num as u8 as char;  // TODO: 👀
                print!("{chr}");
            }
        }
        Some(())
    }

    // TODO: bool sucks
    pub fn step(&mut self) -> bool {
        let next_region = if let Some(r) = self.next_region() { r }
            else { return false; };
        let command = self.region.color.step_to(next_region.color);
        eprintln!("({:?} ({}) -> {:?}) = {command:?}", self.region.color, self.region.value(), next_region.color);
        self.run_command(command);
        self.region = next_region;
        true
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
        _ => { return Err("unsupported image type".to_string()); }
    }
}
