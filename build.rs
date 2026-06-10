use image::{Rgba, RgbaImage};
use std::env;
use std::path::PathBuf;

const ICON_SIZE: u32 = 256;
// App accent green
const BG: [u8; 4] = [45, 180, 140, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let img = generate_icon();

    let out_dir: PathBuf = env::var_os("OUT_DIR").expect("OUT_DIR not set").into();

    let png_path = out_dir.join("icon.png");
    img.save(&png_path).expect("save icon.png");

    let ico_path = out_dir.join("icon.ico");
    write_ico(&img, &ico_path);

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().expect("ico path utf8"));
        if let Err(e) = res.compile() {
            println!("cargo:warning=icon embed skipped: {}", e);
        }
    }
}

fn generate_icon() -> RgbaImage {
    let mut img = RgbaImage::new(ICON_SIZE, ICON_SIZE);
    let bg = Rgba(BG);
    let white = Rgba(WHITE);

    // Rounded square background
    let corner_r = 36i32;
    let n = ICON_SIZE as i32;
    for y in 0..n {
        for x in 0..n {
            if in_rounded_rect(x, y, n, n, corner_r) {
                img.put_pixel(x as u32, y as u32, bg);
            }
        }
    }

    // Three horizontal bars of decreasing width to suggest lines of text.
    // (start_x, top_y, width, height)
    let bars: &[(i32, i32, i32, i32)] = &[
        (60, 84, 136, 22),
        (60, 122, 104, 22),
        (60, 160, 72, 22),
    ];
    let bar_r = 8i32;
    for &(sx, sy, w, h) in bars {
        for y in 0..h {
            for x in 0..w {
                if in_rounded_rect(x, y, w, h, bar_r) {
                    let px = (sx + x) as u32;
                    let py = (sy + y) as u32;
                    if px < ICON_SIZE && py < ICON_SIZE {
                        img.put_pixel(px, py, white);
                    }
                }
            }
        }
    }

    img
}

/// True if (x, y) lies inside a rounded rectangle of size w*h with corner radius r,
/// anchored at (0, 0).
fn in_rounded_rect(x: i32, y: i32, w: i32, h: i32, r: i32) -> bool {
    if x < 0 || y < 0 || x >= w || y >= h {
        return false;
    }
    // Pick corner circle center if we are inside one of the four corner squares.
    let cx = if x < r {
        r
    } else if x >= w - r {
        w - 1 - r
    } else {
        x // in the straight zone, distance check passes automatically
    };
    let cy = if y < r {
        r
    } else if y >= h - r {
        h - 1 - r
    } else {
        y
    };
    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
}

fn write_ico(src: &RgbaImage, path: &std::path::Path) {
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in &[16u32, 24, 32, 48, 64, 128, 256] {
        let resized = image::imageops::resize(src, size, size, image::imageops::FilterType::Lanczos3);
        let raw = resized.into_raw();
        let icon_image = ico::IconImage::from_rgba_data(size, size, raw);
        let entry = ico::IconDirEntry::encode(&icon_image).expect("encode ico entry");
        dir.add_entry(entry);
    }
    let file = std::fs::File::create(path).expect("create icon.ico");
    dir.write(file).expect("write icon.ico");
}
