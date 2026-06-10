use crate::storage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn attachments_dir() -> PathBuf {
    let p = storage::data_dir().join("attachments");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn timestamp_filename(ext: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("img-{}.{}", ts, ext)
}

/// Try to get an image from the clipboard and save it.
/// Returns Ok(path) on success, Err(message) for diagnostics, or Ok(None)
/// when the clipboard simply does not contain an image (this is the expected
/// path when the user is pasting plain text — the caller should NOT show an
/// error in that case).
pub fn paste_clipboard_image() -> Result<Option<PathBuf>, String> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| format!("clipboard open failed: {}", e))?;

    let img = match cb.get_image() {
        Ok(img) => img,
        // No image present is normal (text paste); signal with Ok(None).
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(e) => return Err(format!("clipboard read failed: {}", e)),
    };

    let width = img.width as u32;
    let height = img.height as u32;
    let bytes: Vec<u8> = img.bytes.into_owned();
    if width == 0 || height == 0 {
        return Err("clipboard image has zero dimensions".to_string());
    }
    let expected = (width * height * 4) as usize;
    if bytes.len() < expected {
        return Err(format!(
            "clipboard image is truncated ({} bytes, expected {})",
            bytes.len(),
            expected
        ));
    }

    let buffer = image::RgbaImage::from_raw(width, height, bytes)
        .ok_or_else(|| "failed to build RGBA buffer from clipboard data".to_string())?;
    let path = attachments_dir().join(timestamp_filename("png"));
    image::DynamicImage::ImageRgba8(buffer)
        .save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| format!("save PNG failed: {}", e))?;
    Ok(Some(path))
}

/// Build a markdown image link using a relative path if possible.
pub fn markdown_link_for(path: &Path) -> String {
    // Convert to forward slashes for cross-platform markdown links.
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20");
    format!("![]({})", s)
}
