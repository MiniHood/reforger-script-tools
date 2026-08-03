use png::{BitDepth, ColorType, Decoder, Transformations};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

const THUMBNAIL_FILE: &str = "thumbnail.png";
const MAX_THUMBNAIL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_THUMBNAIL_DIMENSION: u32 = 4_096;
const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

/// Returns the alpha-weighted arithmetic mean of the add-on thumbnail's RGB
/// channels. Invalid, oversized, missing, and fully transparent thumbnails do
/// not make the add-on index unavailable; they simply have no accent color.
pub(crate) fn addon_thumbnail_color(source_root: &Path) -> Option<String> {
    let path = source_root.join(THUMBNAIL_FILE);
    if fs::metadata(&path).ok()?.len() > MAX_THUMBNAIL_BYTES {
        return None;
    }

    let mut decoder = Decoder::new(BufReader::new(File::open(path).ok()?));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_THUMBNAIL_DIMENSION
        || info.height > MAX_THUMBNAIL_DIMENSION
        || reader.output_buffer_size() > MAX_DECODED_BYTES
    {
        return None;
    }

    let mut bytes = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut bytes).ok()?;
    if frame.bit_depth != BitDepth::Eight {
        return None;
    }
    let bytes = &bytes[..frame.buffer_size()];
    let mut sums = [0_u64; 3];
    let mut alpha_sum = 0_u64;

    match frame.color_type {
        ColorType::Rgb => {
            for pixel in bytes.chunks_exact(3) {
                add_pixel(&mut sums, &mut alpha_sum, pixel[0], pixel[1], pixel[2], 255);
            }
        }
        ColorType::Rgba => {
            for pixel in bytes.chunks_exact(4) {
                add_pixel(
                    &mut sums,
                    &mut alpha_sum,
                    pixel[0],
                    pixel[1],
                    pixel[2],
                    pixel[3],
                );
            }
        }
        ColorType::Grayscale => {
            for value in bytes {
                add_pixel(&mut sums, &mut alpha_sum, *value, *value, *value, 255);
            }
        }
        ColorType::GrayscaleAlpha => {
            for pixel in bytes.chunks_exact(2) {
                add_pixel(
                    &mut sums,
                    &mut alpha_sum,
                    pixel[0],
                    pixel[0],
                    pixel[0],
                    pixel[1],
                );
            }
        }
        ColorType::Indexed => return None,
    }

    if alpha_sum == 0 {
        return None;
    }
    let channel = |sum: u64| ((sum + alpha_sum / 2) / alpha_sum) as u8;
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        channel(sums[0]),
        channel(sums[1]),
        channel(sums[2])
    ))
}

fn add_pixel(sums: &mut [u64; 3], alpha_sum: &mut u64, red: u8, green: u8, blue: u8, alpha: u8) {
    let alpha = u64::from(alpha);
    sums[0] += u64::from(red) * alpha;
    sums[1] += u64::from(green) * alpha;
    sums[2] += u64::from(blue) * alpha;
    *alpha_sum += alpha;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn computes_an_alpha_weighted_average_and_ignores_transparent_pixels() {
        let root = temporary_root("average");
        fs::create_dir_all(&root).unwrap();
        write_rgba_png(
            &root.join(THUMBNAIL_FILE),
            3,
            1,
            &[255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 0],
        );
        assert_eq!(addon_thumbnail_color(&root).as_deref(), Some("#800080"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_and_fully_transparent_thumbnails_have_no_color() {
        let root = temporary_root("missing");
        fs::create_dir_all(&root).unwrap();
        assert_eq!(addon_thumbnail_color(&root), None);
        write_rgba_png(&root.join(THUMBNAIL_FILE), 1, 1, &[10, 20, 30, 0]);
        assert_eq!(addon_thumbnail_color(&root), None);
        fs::remove_dir_all(root).unwrap();
    }

    fn write_rgba_png(path: &Path, width: u32, height: u32, pixels: &[u8]) {
        let file = File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }

    fn temporary_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "reforger_addon_thumbnail_{label}_{}_{}",
            std::process::id(),
            nonce
        ))
    }
}
