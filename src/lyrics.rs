use crate::tags;

pub struct Line {
    pub t: f64,
    pub text: String,
}

pub struct Lyrics {
    pub lines: Vec<Line>,
    pub synced: bool,
}

impl Default for Lyrics {
    fn default() -> Self {
        Lyrics {
            lines: Vec::new(),
            synced: true,
        }
    }
}

fn is_time(tag: &str) -> bool {
    let Some(c) = tag.find(':') else { return false };
    if c == 0 || c + 1 >= tag.len() {
        return false;
    }
    tag.bytes()
        .all(|x| x.is_ascii_digit() || x == b':' || x == b'.' || x == b',')
}

pub fn parse_lrc(s: &str) -> Lyrics {
    // ponytail: el BOM de editores Windows invalidaba la primera línea; se pela aquí.
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut out = Lyrics::default();
    let mut off = 0.0;
    for raw in s.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let mut i = 0;
        let mut ts = Vec::new();
        while line[i..].starts_with('[') {
            let Some(c) = line[i..].find(']') else { break };
            let tag = &line[i + 1..i + c];
            if let Some(v) = tag.strip_prefix("offset:") {
                off = v.parse().unwrap_or(0.0);
            } else if is_time(tag) {
                let col = tag.find(':').unwrap();
                let t = tag[..col].parse::<f64>().unwrap_or(0.0) * 60.0
                    + tag[col + 1..]
                        .replace(',', ".")
                        .parse::<f64>()
                        .unwrap_or(0.0);
                ts.push(t);
            }
            i += c + 1;
        }
        if ts.is_empty() {
            continue;
        }
        let text = line[i..].trim().to_string();
        if text.is_empty() {
            continue;
        }
        for t in ts {
            out.lines.push(Line {
                t: t + off / 1000.0,
                text: text.clone(),
            });
        }
    }
    out.lines
        .sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    out
}

pub fn resolve(audio_path: &str) -> Lyrics {
    if let Some(d) = audio_path.rfind('.') {
        let lrc = format!("{}.lrc", &audio_path[..d]);
        if let Ok(s) = std::fs::read_to_string(&lrc) {
            let l = parse_lrc(&s);
            if !l.lines.is_empty() {
                return l;
            }
        }
    }
    let emb = tags::lyrics_text(audio_path);
    if emb.is_empty() {
        return Lyrics::default();
    }
    let mut out = Lyrics {
        lines: Vec::new(),
        synced: false,
    };
    for raw in emb.lines() {
        out.lines.push(Line {
            t: 0.0,
            text: raw.strip_suffix('\r').unwrap_or(raw).to_string(),
        });
    }
    while out.lines.last().is_some_and(|l| l.text.is_empty()) {
        out.lines.pop();
    }
    if out.lines.first().is_some_and(|l| l.text.is_empty()) {
        out.lines.remove(0);
    }
    out
}

pub fn active(l: &Lyrics, t: f64) -> i64 {
    l.lines.partition_point(|x| x.t <= t) as i64 - 1
}
