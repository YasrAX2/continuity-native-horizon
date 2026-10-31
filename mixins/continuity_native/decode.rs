// Adapted from Horizon sprites.rs (MIT); copyright Horizon contributors.
// Full upstream license is in THIRD_PARTY_NOTICES.md.
use super::*;
    pub(super) fn decode(root: &Path, name: &str) -> Result<crate::sprite_cache::Decoded> {
        let (namespace, path) = name.split_once(':').ok_or("invalid sprite identifier")?;
        let path = root.join(format!("assets/{namespace}/{path}.png"));
        let metadata = match fs::read_to_string(path.with_extension("png.mcmeta")) {
            Ok(text) => Some(serde_json::from_str::<
                BTreeMap<String, Box<serde_json::value::RawValue>>,
            >(&text)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let animation_text = metadata
            .as_ref()
            .and_then(|v| v.get("animation"))
            .map(|v| v.get().to_owned());
        let animation = animation_text
            .as_deref()
            .map(AnimationMetadata::from_json)
            .transpose()?;
        let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![
            0;
            reader
                .output_buffer_size()
                .ok_or("sprite exceeds decoder limits")?
        ];
        let info = reader.next_frame(&mut bytes)?;
        let (width, height) = if let Some(animation) = &animation {
            let [width, height] =
                animation.frame_size(i32::try_from(info.width)?, i32::try_from(info.height)?);
            (u32::try_from(width)?, u32::try_from(height)?)
        } else {
            (info.width, info.height)
        };
        if width == 0 || height == 0 || info.width % width != 0 || info.height % height != 0 {
            return Err(format!("sprite {name} has invalid frame dimensions").into());
        }
        let channels = info.color_type.samples();
        let mut pixels = Vec::with_capacity(info.width as usize * info.height as usize * 4);
        for pixel in bytes[..info.buffer_size()].chunks_exact(channels) {
            let rgba = match info.color_type {
                png::ColorType::Rgba => [pixel[0], pixel[1], pixel[2], pixel[3]],
                png::ColorType::Rgb => [pixel[0], pixel[1], pixel[2], 255],
                png::ColorType::Grayscale => [pixel[0], pixel[0], pixel[0], 255],
                png::ColorType::GrayscaleAlpha => [pixel[0], pixel[0], pixel[0], pixel[1]],
                _ => return Err("unsupported expanded sprite format".into()),
            };
            pixels.extend(rgba);
        }
        let columns = info.width / width;
        let count = columns * (info.height / height);
        // Layers are square; a frame of another shape is nearest-resampled to
        // its longer side so UVs (normalized per layer) are unchanged.
        let side = width.max(height);
        let mut frames = Vec::new();
        for index in 0..count {
            let x = index % columns * width;
            let y = index / columns * height;
            let mut frame = Vec::with_capacity((side * side * 4) as usize);
            for row in 0..side {
                let source_row = y + row * height / side;
                for column in 0..side {
                    let offset =
                        ((source_row * info.width + x + column * width / side) * 4) as usize;
                    frame.extend_from_slice(&pixels[offset..offset + 4]);
                }
            }
            frames.push(frame);
        }
        let texture = texture_section(metadata.as_ref());
        Ok(crate::sprite_cache::Decoded {
            side,
            frames,
            animation: animation_text,
            strategy: texture
                .as_ref()
                .and_then(|t| t["mipmap_strategy"].as_str())
                .and_then(crate::mipmap::Strategy::parse)
                .unwrap_or_default(),
            alpha_cutoff_bias: texture
                .as_ref()
                .and_then(|t| t["alpha_cutoff_bias"].as_f64())
                .unwrap_or(0.0) as f32,
            has_transparent: pixels.as_chunks::<4>().0.iter().any(|p| p[3] == 0),
            opaque: pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255),
            translucent: pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[3] != 0 && p[3] != 255),
        })
    }

pub(crate) fn insert(sprites: &mut Sprites, root: &Path, name: &str) -> Result<u32> {
    if let Some(&layer) = sprites.layers.get(name) { return Ok(layer); }
    sprites.register(name, decode(root, name)?)
}
