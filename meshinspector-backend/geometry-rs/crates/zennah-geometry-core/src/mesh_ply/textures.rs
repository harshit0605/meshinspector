use std::path::Path;

use super::*;

pub(super) fn texture_files_from_comments(comments: &[String]) -> Vec<String> {
    comments
        .iter()
        .filter_map(|comment| {
            comment
                .strip_prefix("TextureFile")
                .map(|value| value.trim_start_matches([' ', '\t']))
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}

pub(super) fn texture_images_from_files(
    texture_files: &[String],
    texture_dir: &Path,
) -> Vec<MeshPlyTextureImage> {
    let mut images = Vec::new();
    for texture_file in texture_files {
        let texture_path = texture_dir.join(texture_file);
        if !texture_path.is_file() {
            continue;
        }
        if let Some(image) = load_texture_image(texture_file, &texture_path) {
            images.push(image);
        }
        break;
    }
    images
}

pub(super) fn load_texture_image(
    texture_file: &str,
    texture_path: &Path,
) -> Option<MeshPlyTextureImage> {
    let image = image::ImageReader::open(texture_path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let pixels_rgba = image
        .pixels()
        .map(|pixel| {
            let [red, green, blue, alpha] = pixel.0;
            [red, green, blue, alpha]
        })
        .collect();
    Some(MeshPlyTextureImage {
        file: texture_file.to_owned(),
        resolved_path: texture_path.to_string_lossy().into_owned(),
        width,
        height,
        pixels_rgba,
        filter: "Linear".to_string(),
        wrap: "Clamp".to_string(),
    })
}
