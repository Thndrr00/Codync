//! A picture's own size. Ported from `OpenBot` 0.73.0: the model makes its nearest aspect ratio, and the
//! host crops the middle of that picture to the asked shape and scales it to the exact pixels, as a PNG.

use anyhow::{Result, bail};
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::FilterType};
use std::io::Cursor;

/// The smallest and largest side, as `OpenBot` allows.
const MIN: u32 = 64;
const MAX: u32 = 4096;

/// A width and a height a picture is cut and scaled to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// `1024x1280` (or `1024*1280`), within the limits.
pub fn parse(value: &str) -> Result<Size> {
    let trimmed = value.trim().to_ascii_lowercase();
    let (width, height) =
        trimmed.split_once(['x', '*']).ok_or_else(|| anyhow::anyhow!("a size is WIDTHxHEIGHT, such as `1024x1280`"))?;
    let side = |text: &str, name: &str| -> Result<u32> {
        let number: u32 =
            text.trim().parse().map_err(|_| anyhow::anyhow!("the size's {name} is a whole number of pixels"))?;
        if !(MIN..=MAX).contains(&number) {
            bail!("the size's {name} is {MIN} to {MAX} pixels");
        }
        Ok(number)
    };
    Ok(Size { width: side(width, "width")?, height: side(height, "height")? })
}

/// The `--size WxH` a media note asks for, if it asks for one.
pub fn in_note(note: &str) -> Option<Size> {
    let mut words = note.split_whitespace();
    while let Some(word) = words.next() {
        if word == "--size" {
            return words.next().and_then(|value| parse(value).ok());
        }
    }
    None
}

/// The rectangle of a `source` picture that holds the middle of the asked shape, in pixels.
pub fn crop(source: (u32, u32), target: Size) -> (u32, u32, u32, u32) {
    let (sw, sh) = source;
    if sw == 0 || sh == 0 {
        return (0, 0, sw.max(1), sh.max(1));
    }
    // The source's shape against the asked one, in whole pixels both ways: never a sliver, never a
    // stretch, always centred on what the model made.
    let by_width = (u64::from(sh) * u64::from(target.width) / u64::from(target.height)) as u32;
    let (cw, ch) = if by_width <= sw {
        (by_width, sh)
    } else {
        let by_height = (u64::from(sw) * u64::from(target.height) / u64::from(target.width)) as u32;
        (sw, by_height)
    };
    ((sw - cw) / 2, (sh - ch) / 2, cw.max(1), ch.max(1))
}

/// The picture cut to the asked shape and scaled to its pixels, as a PNG.
pub fn apply(bytes: &[u8], target: Size) -> Result<Vec<u8>> {
    let picture = image::load_from_memory(bytes).map_err(|error| anyhow::anyhow!("reading the picture: {error}"))?;
    let (x, y, width, height) = crop(picture.dimensions(), target);
    let cropped: DynamicImage = picture.crop_imm(x, y, width, height);
    let mut out = Vec::new();
    cropped
        .resize_exact(target.width, target.height, FilterType::Lanczos3)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|error| anyhow::anyhow!("writing the sized picture: {error}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_is_two_whole_sides_within_the_limits() {
        assert_eq!(parse("1024x1280").unwrap(), Size { width: 1024, height: 1280 });
        assert_eq!(parse(" 1024 X 1280 ").unwrap(), Size { width: 1024, height: 1280 });
        assert_eq!(parse("64x4096").unwrap(), Size { width: 64, height: 4096 });
        assert!(parse("1024").is_err(), "one side is not a size");
        assert!(parse("63x128").is_err(), "too small");
        assert!(parse("1024x4097").is_err(), "too large");
        assert!(parse("a x b").is_err(), "not numbers");
    }

    #[test]
    fn a_note_names_its_size_only_when_it_holds_one() {
        assert_eq!(in_note("Make one picture --size 1024x1280 of a fox."), Some(Size { width: 1024, height: 1280 }));
        assert_eq!(in_note("Make one picture of a fox."), None);
        assert_eq!(in_note("--size 10x10"), None, "a size out of range is not one");
    }

    #[test]
    fn the_crop_takes_the_middle_and_keeps_the_asked_shape() {
        // A square picture asked to a tall frame: the sides are cut, the height stays.
        assert_eq!(crop((1024, 1024), Size { width: 1024, height: 1280 }), (0, 0, 819, 1024));
        // A wide picture asked to a square: the top and bottom go.
        assert_eq!(crop((1600, 900), Size { width: 1000, height: 1000 }), (350, 0, 900, 900));
        // A picture already of the asked shape is not moved.
        assert_eq!(crop((1024, 1280), Size { width: 1024, height: 1280 }), (0, 0, 1024, 1280));
        // Degenerate input never asks for an empty rectangle.
        let (x, y, w, h) = crop((0, 0), Size { width: 100, height: 100 });
        assert_eq!((x, y), (0, 0));
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn a_picture_comes_out_at_exactly_the_asked_size() {
        let source = image::RgbaImage::from_pixel(1600, 900, image::Rgba([20, 120, 220, 255]));
        let mut png = Vec::new();
        DynamicImage::ImageRgba8(source)
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .expect("encoding the source");
        let sized = apply(&png, Size { width: 200, height: 300 }).expect("a sized picture");
        let out = image::load_from_memory(&sized).expect("decoding the result");
        assert_eq!(out.dimensions(), (200, 300));
        assert_eq!(out.color(), image::ColorType::Rgba8);
    }
}
