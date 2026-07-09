use crate::backend::{BackendError, BackendResult};
use crc32fast::Hasher;
use flate2::read::DeflateDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const GENERATED_ICON_PREFIX: &str = "SuperSideloaderIcon";

pub(crate) const IPHONE_ICON_NAMES: &[&str] = &[
    "SuperSideloaderIcon20",
    "SuperSideloaderIcon29",
    "SuperSideloaderIcon40",
    "SuperSideloaderIcon60",
];
pub(crate) const IPAD_ICON_NAMES: &[&str] = &[
    "SuperSideloaderIcon20",
    "SuperSideloaderIcon29",
    "SuperSideloaderIcon40",
    "SuperSideloaderIcon76",
    "SuperSideloaderIcon83_5",
];
pub(crate) const IPHONE_PRIMARY_ICON_NAME: &str = "SuperSideloaderIcon60";
pub(crate) const IPAD_PRIMARY_ICON_NAME: &str = "SuperSideloaderIcon76";

struct IconVariant {
    file_name: &'static str,
    pixels: u32,
}

const ICON_VARIANTS: &[IconVariant] = &[
    IconVariant {
        file_name: "SuperSideloaderIcon20.png",
        pixels: 20,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon20@2x.png",
        pixels: 40,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon20@3x.png",
        pixels: 60,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon29.png",
        pixels: 29,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon29@2x.png",
        pixels: 58,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon29@3x.png",
        pixels: 87,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon40.png",
        pixels: 40,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon40@2x.png",
        pixels: 80,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon40@3x.png",
        pixels: 120,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon60.png",
        pixels: 60,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon60@2x.png",
        pixels: 120,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon60@3x.png",
        pixels: 180,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon76.png",
        pixels: 76,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon76@2x.png",
        pixels: 152,
    },
    IconVariant {
        file_name: "SuperSideloaderIcon83_5@2x.png",
        pixels: 167,
    },
];

pub(crate) fn normalize_png_for_display(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let image = decode_png(bytes)?;
    encode_png(&image)
}

pub(crate) fn write_icon_variants(app_path: &Path, source_path: &Path) -> BackendResult<()> {
    let source_bytes = fs::read(source_path).map_err(|source| BackendError::Io {
        action: "Read app icon override",
        path: source_path.to_path_buf(),
        source,
    })?;
    let source = decode_png(&source_bytes).map_err(|error| {
        BackendError::Message(format!(
            "Failed to decode app icon override {}: {error}",
            source_path.display()
        ))
    })?;
    if source.width() != source.height() {
        return Err(BackendError::Message(format!(
            "The app icon override must be square, but it is {}x{} pixels.",
            source.width(),
            source.height()
        )));
    }

    remove_previous_variants(app_path)?;
    for variant in ICON_VARIANTS {
        let destination = app_path.join(variant.file_name);
        source
            .resize_exact(variant.pixels, variant.pixels, FilterType::Lanczos3)
            .save_with_format(&destination, ImageFormat::Png)
            .map_err(|error| {
                BackendError::Message(format!(
                    "Failed to write app icon variant {}: {error}",
                    destination.display()
                ))
            })?;
    }
    Ok(())
}

fn decode_png(bytes: &[u8]) -> Result<DynamicImage, String> {
    let chunks = parse_png_chunks(bytes)?;
    if chunks.iter().any(|chunk| chunk.kind == *b"CgBI") {
        decode_cgbi_png(&chunks)
    } else {
        image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .map_err(|error| format!("invalid PNG: {error}"))
    }
}

fn decode_cgbi_png(chunks: &[PngChunk<'_>]) -> Result<DynamicImage, String> {
    let ihdr = chunks
        .iter()
        .find(|chunk| chunk.kind == *b"IHDR")
        .ok_or_else(|| "CgBI PNG has no IHDR chunk".to_string())?;
    if ihdr.data.len() != 13 || ihdr.data[8] != 8 || ihdr.data[9] != 6 {
        return Err("CgBI PNG must use 8-bit RGBA pixels".to_string());
    }

    let compressed = chunks
        .iter()
        .filter(|chunk| chunk.kind == *b"IDAT")
        .flat_map(|chunk| chunk.data.iter().copied())
        .collect::<Vec<_>>();
    if compressed.is_empty() {
        return Err("CgBI PNG has no IDAT data".to_string());
    }

    let mut filtered_pixels = Vec::new();
    DeflateDecoder::new(compressed.as_slice())
        .read_to_end(&mut filtered_pixels)
        .map_err(|error| format!("failed to inflate CgBI pixel data: {error}"))?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&filtered_pixels)
        .map_err(|error| format!("failed to recompress CgBI pixel data: {error}"))?;
    let standard_idat = encoder
        .finish()
        .map_err(|error| format!("failed to finish CgBI pixel data: {error}"))?;

    let mut standard_png = PNG_SIGNATURE.to_vec();
    let mut wrote_idat = false;
    for chunk in chunks {
        match &chunk.kind {
            b"CgBI" => {}
            b"IDAT" if !wrote_idat => {
                write_png_chunk(&mut standard_png, *b"IDAT", &standard_idat)?;
                wrote_idat = true;
            }
            b"IDAT" => {}
            _ => write_png_chunk(&mut standard_png, chunk.kind, chunk.data)?,
        }
    }

    let mut image = image::load_from_memory_with_format(&standard_png, ImageFormat::Png)
        .map_err(|error| format!("failed to decode normalized CgBI PNG: {error}"))?
        .to_rgba8();
    for pixel in image.pixels_mut() {
        let [blue, green, red, alpha] = pixel.0;
        *pixel = Rgba([
            unpremultiply(red, alpha),
            unpremultiply(green, alpha),
            unpremultiply(blue, alpha),
            alpha,
        ]);
    }
    Ok(DynamicImage::ImageRgba8(image))
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|error| format!("failed to encode PNG: {error}"))?;
    Ok(output.into_inner())
}

fn unpremultiply(value: u8, alpha: u8) -> u8 {
    if alpha == 0 {
        return 0;
    }
    ((u32::from(value) * 255 + u32::from(alpha) / 2) / u32::from(alpha)).min(255) as u8
}

struct PngChunk<'a> {
    kind: [u8; 4],
    data: &'a [u8],
}

fn parse_png_chunks(bytes: &[u8]) -> Result<Vec<PngChunk<'_>>, String> {
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("file is not a PNG".to_string());
    }

    let mut chunks = Vec::new();
    let mut offset = PNG_SIGNATURE.len();
    while offset < bytes.len() {
        let header_end = offset
            .checked_add(8)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| "PNG has a truncated chunk header".to_string())?;
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = bytes[offset + 4..header_end].try_into().unwrap();
        let data_start = header_end;
        let data_end = data_start
            .checked_add(length)
            .filter(|end| {
                end.checked_add(4)
                    .is_some_and(|crc_end| crc_end <= bytes.len())
            })
            .ok_or_else(|| "PNG has a truncated chunk".to_string())?;
        chunks.push(PngChunk {
            kind,
            data: &bytes[data_start..data_end],
        });
        offset = data_end + 4;
        if kind == *b"IEND" {
            break;
        }
    }

    if !chunks.iter().any(|chunk| chunk.kind == *b"IHDR") {
        return Err("PNG has no IHDR chunk".to_string());
    }
    if !chunks.iter().any(|chunk| chunk.kind == *b"IEND") {
        return Err("PNG has no IEND chunk".to_string());
    }
    Ok(chunks)
}

fn write_png_chunk(output: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) -> Result<(), String> {
    let length = u32::try_from(data.len()).map_err(|_| "PNG chunk is too large".to_string())?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(&kind);
    output.extend_from_slice(data);
    let mut crc = Hasher::new();
    crc.update(&kind);
    crc.update(data);
    output.extend_from_slice(&crc.finalize().to_be_bytes());
    Ok(())
}

fn remove_previous_variants(app_path: &Path) -> BackendResult<()> {
    let entries = fs::read_dir(app_path).map_err(|source| BackendError::Io {
        action: "Read app bundle while replacing icons",
        path: app_path.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| BackendError::Io {
            action: "Read app bundle entry while replacing icons",
            path: app_path.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let generated_png = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.starts_with(GENERATED_ICON_PREFIX)
                    && name.to_ascii_lowercase().ends_with(".png")
            });
        if generated_png {
            fs::remove_file(&path).map_err(|source| BackendError::Io {
                action: "Remove previous app icon variant",
                path,
                source,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use image::{GenericImageView, RgbaImage};

    #[test]
    fn cgbi_png_is_normalized_to_standard_rgba_png() {
        let cgbi = cgbi_test_png();

        let normalized = normalize_png_for_display(&cgbi).unwrap();
        assert!(!parse_png_chunks(&normalized)
            .unwrap()
            .iter()
            .any(|chunk| chunk.kind == *b"CgBI"));

        let image = image::load_from_memory_with_format(&normalized, ImageFormat::Png).unwrap();
        assert_eq!(image.dimensions(), (2, 1));
        assert_eq!(image.to_rgba8().get_pixel(0, 0).0, [128, 64, 32, 128]);
        assert_eq!(image.to_rgba8().get_pixel(1, 0).0, [20, 40, 80, 255]);
    }

    #[test]
    fn icon_variants_have_the_expected_pixel_sizes() {
        let temp = tempfile::tempdir().unwrap();
        let app_path = temp.path().join("Example.app");
        fs::create_dir(&app_path).unwrap();
        let source_path = temp.path().join("source.png");
        RgbaImage::from_pixel(180, 180, Rgba([20, 40, 80, 255]))
            .save(&source_path)
            .unwrap();

        write_icon_variants(&app_path, &source_path).unwrap();

        for variant in ICON_VARIANTS {
            let image = image::open(app_path.join(variant.file_name)).unwrap();
            assert_eq!(
                image.dimensions(),
                (variant.pixels, variant.pixels),
                "{}",
                variant.file_name
            );
        }
    }

    #[test]
    fn icon_override_must_be_square() {
        let temp = tempfile::tempdir().unwrap();
        let app_path = temp.path().join("Example.app");
        fs::create_dir(&app_path).unwrap();
        let source_path = temp.path().join("source.png");
        RgbaImage::new(180, 120).save(&source_path).unwrap();

        let error = write_icon_variants(&app_path, &source_path).unwrap_err();
        assert!(error.user_message().contains("must be square"));
    }

    fn cgbi_test_png() -> Vec<u8> {
        let mut raw_pixels = vec![0];
        raw_pixels.extend_from_slice(&[16, 32, 64, 128]);
        raw_pixels.extend_from_slice(&[80, 40, 20, 255]);
        let mut deflater = DeflateEncoder::new(Vec::new(), Compression::default());
        deflater.write_all(&raw_pixels).unwrap();
        let idat = deflater.finish().unwrap();

        let mut png = PNG_SIGNATURE.to_vec();
        write_png_chunk(&mut png, *b"CgBI", &[0x40, 0, 0, 0]).unwrap();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&2u32.to_be_bytes());
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        write_png_chunk(&mut png, *b"IHDR", &ihdr).unwrap();
        write_png_chunk(&mut png, *b"IDAT", &idat).unwrap();
        write_png_chunk(&mut png, *b"IEND", &[]).unwrap();
        png
    }
}
