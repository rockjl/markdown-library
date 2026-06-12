//! Clipboard image paste support.

use crate::storage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Path to the attachments directory, created on demand.
pub fn attachments_dir() -> PathBuf {
    let p = storage::data_dir().join("attachments");
    let _ = std::fs::create_dir_all(&p);
    p
}

/// Generate a unique filename based on the current millisecond timestamp.
///
/// # Parameters
/// * `ext` - File extension (e.g. "png")
///
/// # Returns
/// A filename like `img-1712345678000.png`
fn timestamp_filename(ext: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("img-{}.{}", ts, ext)
}

/// Try to get an image from the clipboard and save it to the attachments directory.
///
/// # Returns
/// - `Ok(Some(path))` if an image was pasted and saved
/// - `Ok(None)` if the clipboard does not contain an image (normal for text paste)
/// - `Err(message)` on failure
pub fn paste_clipboard_image() -> Result<Option<PathBuf>, String> {
    let mut cb = arboard::Clipboard::new()
        .map_err(|e| format!("clipboard open failed: {}", e))?;

    let img = match cb.get_image() {
        Ok(img) => img,
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

/// Build a markdown image link from a file path.
///
/// Converts backslashes to forward slashes and encodes spaces as `%20`.
///
/// # Returns
/// A string like `![](path/to/image.png)`
pub fn markdown_link_for(path: &Path) -> String {
    let s = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20");
    format!("![]({})", s)
}
