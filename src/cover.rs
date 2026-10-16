use crate::tags;
use std::fs;

struct Px {
    r: u8,
    g: u8,
    b: u8,
}

struct Img {
    px: Vec<Px>,
    w: usize,
    h: usize,
}

fn decode_jpeg(b: &[u8]) -> Option<Img> {
    let mut d = jpeg_decoder::Decoder::new(b);
    let px = d.decode().ok()?;
    let i = d.info()?;
    let (w, h) = (i.width as usize, i.height as usize);
    let out: Vec<Px> = match i.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| Px {
                r: c[0],
                g: c[1],
                b: c[2],
            })
            .collect(),
        jpeg_decoder::PixelFormat::L8 => px.iter().map(|&g| Px { r: g, g, b: g }).collect(),
        _ => return None,
    };
    // ponytail: techo 16Mpx; portadas reales son KB. Más = fichero hostil, se ignora.
    if out.len() != w * h || w * h > 16 * 1024 * 1024 {
        return None;
    }
    Some(Img { px: out, w, h })
}

fn decode_png(b: &[u8]) -> Option<Img> {
    let d = png::Decoder::new(b);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0u8; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    // ponytail: techo 16Mpx, ver jpeg.
    if w * h > 16 * 1024 * 1024 {
        return None;
    }
    let px: Vec<Px> = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgb, png::BitDepth::Eight) => buf[..w * h * 3]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|c| Px {
                r: c[0],
                g: c[1],
                b: c[2],
            })
            .collect(),
        (png::ColorType::Rgba, png::BitDepth::Eight) => buf[..w * h * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| Px {
                r: c[0],
                g: c[1],
                b: c[2],
            })
            .collect(),
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            buf[..w * h].iter().map(|&g| Px { r: g, g, b: g }).collect()
        }
        _ => return None,
    };
    Some(Img { px, w, h })
}

// ponytail: promedio por área, sin filtro; suficiente para miniaturas de terminal.
fn resize(s: &Img, tw: usize, th: usize) -> Option<Img> {
    if tw == 0 || th == 0 || s.w == 0 || s.h == 0 {
        return None;
    }
    let mut px = Vec::with_capacity(tw * th);
    for y in 0..th {
        let y1 = y * s.h / th;
        let y2 = ((y + 1) * s.h / th).max(y1 + 1).min(s.h);
        for x in 0..tw {
            let x1 = x * s.w / tw;
            let x2 = ((x + 1) * s.w / tw).max(x1 + 1).min(s.w);
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for cy in y1..y2 {
                for cx in x1..x2 {
                    let p = &s.px[cy * s.w + cx];
                    r += p.r as u32;
                    g += p.g as u32;
                    b += p.b as u32;
                    n += 1;
                }
            }
            px.push(Px {
                r: (r / n) as u8,
                g: (g / n) as u8,
                b: (b / n) as u8,
            });
        }
    }
    Some(Img { px, w: tw, h: th })
}

fn render(img: &Img, cols: usize, rows: usize) -> Vec<String> {
    let s = match resize(img, cols, rows * 2) {
        Some(s) if !s.px.is_empty() => s,
        _ => return blank(cols, rows),
    };
    (0..rows)
        .map(|ty| {
            let mut l = String::with_capacity(cols * 28);
            for tx in 0..cols {
                let up = &s.px[(ty * 2) * cols + tx];
                let dn = &s.px[((ty * 2 + 1).min(rows * 2 - 1)) * cols + tx];
                l.push_str(&format!(
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                    up.r, up.g, up.b, dn.r, dn.g, dn.b
                ));
            }
            l.push_str("\x1b[0m");
            l
        })
        .collect()
}

fn blank(cols: usize, rows: usize) -> Vec<String> {
    (0..rows)
        .map(|y| {
            if y == rows / 2 {
                let t = "(no cover)";
                let p = cols.saturating_sub(t.len()) / 2;
                format!("{}{t}{}", " ".repeat(p), " ".repeat(cols - p - t.len()))
            } else {
                " ".repeat(cols)
            }
        })
        .collect()
}

const FALLBACK: &[&str] = &[
    "cover.jpg",
    "cover.png",
    "folder.jpg",
    "folder.png",
    "album.jpg",
    "album.png",
    "front.jpg",
    "front.png",
];

fn load(audio_path: &str) -> Option<Img> {
    if let Some((b, m)) = tags::cover_bytes(audio_path) {
        let img = if m == "png" {
            decode_png(&b)
        } else {
            decode_jpeg(&b)
        };
        if img.is_some() {
            return img;
        }
    }
    let dir = std::path::Path::new(audio_path).parent()?;
    for n in FALLBACK {
        let p = dir.join(n);
        if let Ok(b) = fs::read(&p) {
            if b.len() > 8 * 1024 * 1024 {
                continue;
            }
            let s = p.to_string_lossy().to_lowercase();
            let img = if s.ends_with(".png") {
                decode_png(&b)
            } else {
                decode_jpeg(&b)
            };
            if img.is_some() {
                return img;
            }
        }
    }
    None
}

pub struct Cache {
    path: String,
    cols: usize,
    rows: usize,
    lines: Vec<String>,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            path: String::new(),
            cols: 0,
            rows: 0,
            lines: Vec::new(),
        }
    }

    pub fn lines_for(&mut self, audio_path: &str, cols: usize, rows: usize) -> &[String] {
        if audio_path != self.path || cols != self.cols || rows != self.rows {
            self.path = audio_path.to_string();
            self.cols = cols;
            self.rows = rows;
            self.lines = match (audio_path.is_empty(), load(audio_path)) {
                (true, _) => blank(cols, rows),
                (_, Some(img)) => render(&img, cols, rows),
                (_, None) => blank(cols, rows),
            };
        }
        &self.lines
    }
}
