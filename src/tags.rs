// SPDX-License-Identifier: GPL-3.0-only
use std::fs;

pub const EXTS: &[&str] = &[
    ".mp3", ".flac", ".wav", ".ogg", ".opus", ".aiff", ".aif", ".au",
];

pub fn is_audio_name(n: &str) -> bool {
    let l = n.to_lowercase();
    EXTS.iter().any(|e| l.ends_with(e))
}

pub fn strip_ext(n: &str) -> String {
    let l = n.to_lowercase();
    for e in EXTS {
        if l.ends_with(e) {
            return n[..n.len() - e.len()].to_string();
        }
    }
    n.to_string()
}

#[derive(Clone, Default)]
pub struct TrackMeta {
    pub artist: String,
    pub album: String,
    pub title: String,
    pub duration: f64,
    pub bitrate: u32,
}

fn latin(b: &[u8]) -> String {
    let s: String = b.iter().map(|&x| x as char).collect();
    s.trim().trim_matches('\0').to_string()
}

fn utf16(b: &[u8], be: bool) -> String {
    let mut u = Vec::new();
    let mut i = 0;
    let mut order = be;
    if b.len() >= 2 {
        if b[0] == 0xFF && b[1] == 0xFE {
            order = false;
            i = 2;
        } else if b[0] == 0xFE && b[1] == 0xFF {
            order = true;
            i = 2;
        }
    }
    while i + 1 < b.len() {
        let v = if order {
            u16::from_be_bytes([b[i], b[i + 1]])
        } else {
            u16::from_le_bytes([b[i], b[i + 1]])
        };
        if v == 0 {
            break;
        }
        u.push(v);
        i += 2;
    }
    char::decode_utf16(u)
        .filter_map(|r| r.ok())
        .collect::<String>()
        .trim()
        .to_string()
}

fn text_frame(enc: u8, b: &[u8]) -> String {
    match enc {
        0x01 => utf16(b, false),
        0x02 => utf16(b, true),
        0x03 => String::from_utf8_lossy(b)
            .trim()
            .trim_matches('\0')
            .to_string(),
        _ => latin(b),
    }
}

fn ss28(b: &[u8]) -> u32 {
    ((b[0] as u32 & 0x7f) << 21)
        | ((b[1] as u32 & 0x7f) << 14)
        | ((b[2] as u32 & 0x7f) << 7)
        | (b[3] as u32 & 0x7f)
}

// (artist, album, title, lyrics, cover_bytes+formato)
struct Id3Out {
    artist: String,
    album: String,
    title: String,
    lyrics: String,
    cover: Option<(Vec<u8>, String)>,
}

fn id3(path: &str) -> Id3Out {
    let mut out = Id3Out {
        artist: String::new(),
        album: String::new(),
        title: String::new(),
        lyrics: String::new(),
        cover: None,
    };
    let d = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return out,
    };
    if d.len() < 10 || &d[0..3] != b"ID3" {
        return out;
    }
    let ver = d[3];
    let unsync = d[5] & 0x80 != 0;
    let tlen = ss28(&d[6..10]).min(32 * 1024 * 1024) as usize;
    let mut tag = d[10..d.len().min(10 + tlen)].to_vec();
    if unsync {
        let mut u = Vec::with_capacity(tag.len());
        let mut i = 0;
        while i < tag.len() {
            u.push(tag[i]);
            if tag[i] == 0xFF && i + 1 < tag.len() && tag[i + 1] == 0x00 {
                i += 1;
            }
            i += 1;
        }
        tag = u;
    }
    let v22 = ver == 2;
    let mut p = 0;
    while p + if v22 { 6 } else { 10 } <= tag.len() {
        if tag[p] == 0 {
            break;
        }
        let (fid, fsz, hlen) = if v22 {
            let id = String::from_utf8_lossy(&tag[p..p + 3]).into_owned();
            let sz = ((tag[p + 3] as u32) << 16) | ((tag[p + 4] as u32) << 8) | tag[p + 5] as u32;
            (id, sz as usize, 6)
        } else {
            let id = String::from_utf8_lossy(&tag[p..p + 4]).into_owned();
            let sz = if ver >= 4 {
                ss28(&tag[p + 4..p + 8])
            } else {
                u32::from_be_bytes([tag[p + 4], tag[p + 5], tag[p + 6], tag[p + 7]])
            };
            (id, sz as usize, 10)
        };
        p += hlen;
        if p + fsz > tag.len() {
            break;
        }
        let b = &tag[p..p + fsz];
        if b.len() > 1 {
            match fid.as_str() {
                "TPE1" | "TP1" => out.artist = text_frame(b[0], &b[1..]),
                "TALB" | "TAL" => out.album = text_frame(b[0], &b[1..]),
                "TIT2" | "TT2" => out.title = text_frame(b[0], &b[1..]),
                "USLT" | "ULT" if out.lyrics.is_empty() => out.lyrics = uslt(b),
                "APIC" | "PIC" if out.cover.is_none() => out.cover = apic(b, v22),
                _ => {}
            }
        }
        p += fsz;
    }
    out
}

fn uslt(b: &[u8]) -> String {
    if b.len() < 5 {
        return String::new();
    }
    let enc = b[0];
    let mut i = 4; // salta idioma (3)
    if enc == 1 || enc == 2 {
        while i + 1 < b.len() {
            if b[i] == 0 && b[i + 1] == 0 {
                i += 2;
                break;
            }
            i += 2;
        }
        utf16(&b[i.min(b.len())..], enc == 2)
    } else {
        while i < b.len() && b[i] != 0 {
            i += 1;
        }
        i = (i + 1).min(b.len());
        String::from_utf8_lossy(&b[i..])
            .trim()
            .trim_matches('\0')
            .to_string()
    }
}

fn sniff(b: &[u8]) -> &str {
    if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF {
        "jpeg"
    } else if b.len() >= 4 && b[0..4] == [0x89, b'P', b'N', b'G'] {
        "png"
    } else {
        ""
    }
}

fn apic(b: &[u8], v22: bool) -> Option<(Vec<u8>, String)> {
    if b.len() < 5 {
        return None;
    }
    let enc = b[0];
    let mut p: usize;
    let mime: String;
    if v22 {
        if b.len() < 4 {
            return None;
        }
        mime = String::from_utf8_lossy(&b[1..4]).to_lowercase();
        p = 4;
    } else {
        let s = 1;
        p = s;
        while p < b.len() && b[p] != 0 {
            p += 1;
        }
        mime = String::from_utf8_lossy(&b[s..p]).to_lowercase();
        p += 1;
    }
    p += 1; // tipo de imagen
    if enc == 1 || enc == 2 {
        while p + 1 < b.len() {
            if b[p] == 0 && b[p + 1] == 0 {
                p += 2;
                break;
            }
            p += 2;
        }
    } else {
        while p < b.len() && b[p] != 0 {
            p += 1;
        }
        p += 1;
    }
    if p >= b.len() {
        return None;
    }
    let data = &b[p.min(b.len())..];
    let m = if mime.contains("png") {
        "png"
    } else if mime.contains("jpg") || mime.contains("jpeg") {
        "jpeg"
    } else {
        sniff(data)
    };
    if data.len() > 4 && !m.is_empty() {
        Some((data.to_vec(), m.to_string()))
    } else {
        None
    }
}

fn kv(m: &mut TrackMeta, lyr: &mut String, s: &str) {
    let Some(eq) = s.find('=') else { return };
    let (k, v) = (&s[..eq], s[eq + 1..].trim().to_string());
    if v.is_empty() {
        return;
    }
    if k.eq_ignore_ascii_case("ARTIST") {
        m.artist = v;
    } else if k.eq_ignore_ascii_case("ALBUM") {
        m.album = v;
    } else if k.eq_ignore_ascii_case("TITLE") {
        m.title = v;
    } else if k.eq_ignore_ascii_case("LYRICS") && lyr.is_empty() {
        *lyr = v;
    }
}

fn vorbis(
    d: &[u8],
    off: usize,
    m: &mut TrackMeta,
    lyr: &mut String,
    pic: &mut Option<(Vec<u8>, String)>,
) {
    let le = |o: usize| -> u32 {
        if o + 4 > d.len() {
            0
        } else {
            u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
        }
    };
    if off + 4 > d.len() {
        return;
    }
    let mut o = off + 4 + le(off) as usize;
    if o + 4 > d.len() {
        return;
    }
    let n = le(o) as usize;
    o += 4;
    for _ in 0..n {
        if o + 4 > d.len() {
            return;
        }
        let l = le(o) as usize;
        o += 4;
        if o + l > d.len() {
            return;
        }
        let s = String::from_utf8_lossy(&d[o..o + l]).into_owned();
        if let Some(rest) = s.strip_prefix("METADATA_BLOCK_PICTURE=") {
            if pic.is_none() {
                if let Some(p) = picture(&b64(rest)) {
                    *pic = Some(p);
                }
            }
        } else {
            kv(m, lyr, &s);
        }
        o += l;
    }
}

fn picture(b: &[u8]) -> Option<(Vec<u8>, String)> {
    if b.len() < 32 {
        return None;
    }
    let be = |o: usize| u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let ml = be(4) as usize;
    if 8 + ml + 4 > b.len() {
        return None;
    }
    let mime = String::from_utf8_lossy(&b[8..8 + ml]).to_lowercase();
    let dl = be(8 + ml) as usize;
    let o = 8 + ml + 4 + dl + 16 + 4;
    if o > b.len() {
        return None;
    }
    let dl2 = be(o - 4) as usize;
    if o + dl2 > b.len() {
        return None;
    }
    let data = &b[o..o + dl2];
    let m = if mime.contains("png") {
        "png"
    } else if mime.contains("jpeg") || mime.contains("jpg") {
        "jpeg"
    } else {
        sniff(data)
    };
    if !m.is_empty() {
        Some((data.to_vec(), m.to_string()))
    } else {
        None
    }
}

fn b64(s: &str) -> Vec<u8> {
    // ponytail: decodificador mínimo (alfabeto estándar).
    fn v(c: u8) -> i8 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i8,
            b'a'..=b'z' => (c - b'a' + 26) as i8,
            b'0'..=b'9' => (c - b'0' + 52) as i8,
            b'+' => 62,
            b'/' => 63,
            _ => -1,
        }
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut n = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let x = v(c);
        if x < 0 {
            continue;
        }
        acc = (acc << 6) | x as u32;
        n += 1;
        if n == 4 {
            out.extend_from_slice(&acc.to_be_bytes()[1..]);
            acc = 0;
            n = 0;
        }
    }
    if n == 3 {
        out.extend_from_slice(&acc.to_be_bytes()[1..3]);
    } else if n == 2 {
        out.push((acc >> 4) as u8);
    }
    out
}

fn flac(path: &str, m: &mut TrackMeta, lyr: &mut String, pic: &mut Option<(Vec<u8>, String)>) {
    let d = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return,
    };
    if d.len() < 4 || &d[0..4] != b"fLaC" {
        return;
    }
    let mut o = 4;
    for _ in 0..64 {
        if o + 4 > d.len() {
            return;
        }
        let last = d[o] & 0x80 != 0;
        let typ = d[o] & 0x7F;
        let sz = ((d[o + 1] as usize) << 16) | ((d[o + 2] as usize) << 8) | d[o + 3] as usize;
        o += 4;
        if o + sz > d.len() {
            return;
        }
        if typ == 4 {
            vorbis(&d, o, m, lyr, pic);
        } else if typ == 6 && pic.is_none() {
            if let Some(p) = picture(&d[o..o + sz]) {
                *pic = Some(p);
            }
        }
        o += sz;
        if last {
            return;
        }
    }
}

fn ogg(path: &str, m: &mut TrackMeta, lyr: &mut String, pic: &mut Option<(Vec<u8>, String)>) {
    let d = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let n = d.len().min(65536);
    let mut k = 0;
    while k + 8 < n {
        if &d[k..k + 7] == b"\x03vorbis" {
            vorbis(&d, k + 7, m, lyr, pic);
            return;
        }
        if &d[k..k + 8] == b"OpusTags" {
            vorbis(&d, k + 8, m, lyr, pic);
            return;
        }
        k += 1;
    }
}

pub fn read_tags(path: &str) -> Result<TrackMeta, String> {
    let mut m = TrackMeta::default();
    let mut lyr = String::new();
    let mut pic: Option<(Vec<u8>, String)> = None;
    let l = path.to_lowercase();
    if l.ends_with(".mp3") {
        let o = id3(path);
        m.artist = o.artist;
        m.album = o.album;
        m.title = o.title;
        lyr = o.lyrics;
        pic = o.cover;
    } else if l.ends_with(".flac") {
        flac(path, &mut m, &mut lyr, &mut pic);
    } else if l.ends_with(".ogg") || l.ends_with(".opus") {
        ogg(path, &mut m, &mut lyr, &mut pic);
    }
    // ponytail: letras/portada se releen bajo demanda (play/cover);
    // aquí solo importan artista/álbum/título para la lista.
    let _ = (lyr, pic);
    Ok(m)
}

pub fn lyrics_text(path: &str) -> String {
    let l = path.to_lowercase();
    if l.ends_with(".mp3") {
        id3(path).lyrics
    } else {
        let mut m = TrackMeta::default();
        let mut y = String::new();
        let mut p = None;
        if l.ends_with(".flac") {
            flac(path, &mut m, &mut y, &mut p);
        } else if l.ends_with(".ogg") || l.ends_with(".opus") {
            ogg(path, &mut m, &mut y, &mut p);
        }
        y
    }
}

pub fn cover_bytes(path: &str) -> Option<(Vec<u8>, String)> {
    let l = path.to_lowercase();
    if l.ends_with(".mp3") {
        id3(path).cover
    } else {
        let mut m = TrackMeta::default();
        let mut y = String::new();
        let mut p = None;
        if l.ends_with(".flac") {
            flac(path, &mut m, &mut y, &mut p);
        } else if l.ends_with(".ogg") || l.ends_with(".opus") {
            ogg(path, &mut m, &mut y, &mut p);
        }
        p
    }
}
