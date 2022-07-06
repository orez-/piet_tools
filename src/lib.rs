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
        FromPrimitive::from_i32(light_step * 6 + hue_step).unwrap()
    }
}

#[derive(FromPrimitive)]
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

pub struct PietCode {
    width: usize,
    height: usize,
    code: Vec<Color>,
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
            let code = iproduct!(0..width, 0..height)
                .map(|(x, y)| {
                    img.view(x, y, codel_size, codel_size)
                        .pixels()
                        .map(|(_, _, px)| px)
                        .get_all_equal()
                        // TODO: error on None
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
