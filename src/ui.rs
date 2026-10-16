// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    audio::Player,
    cover, tags,
    term::{self, TSz},
    theme::ThemeMgr,
};

const RST: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const HIDE: &str = "\x1b[?25l";
const SHOW: &str = "\x1b[?25h";

fn header(o: &mut String, cols: usize, mid: &str, t: &ThemeMgr) {
    let c = &t.c;
    let tw = 2 + c.name.len();
    o.push_str(&format!(
        "{}{BOLD} {}♪  RMUSTP{}{}{} {}{} {RST}\n",
        c.bg_hdr,
        c.grn,
        c.w1,
        c.w2,
        term::center(mid, cols.saturating_sub(10 + tw)),
        c.w3,
        c.name
    ));
}

fn status(o: &mut String, cols: usize, p: &Player, t: &ThemeMgr, extra: &str) {
    let c = &t.c;
    o.push_str(&c.bg_stat);
    if p.paused {
        o.push(' ');
        o.push_str(&c.amb);
        o.push('⏸');
    } else if p.playing_now.is_empty() {
        o.push(' ');
        o.push_str(&c.w3);
        o.push('■');
    } else {
        o.push(' ');
        o.push_str(&c.grn);
        o.push('▶');
    }
    o.push_str(&c.w2);
    o.push_str(" ▐ ");
    let vn = (p.volume * 8.0).round().clamp(0.0, 8.0) as usize;
    o.push_str(&c.grn);
    o.push_str(&"█".repeat(vn));
    o.push_str(&c.w3);
    o.push_str(&"░".repeat(8 - vn));
    o.push_str(&c.w2);
    let nums = format!("{}/{}", p.row + 1, p.songs.len());
    o.push_str("  ");
    o.push_str(&nums);
    o.push(' ');
    o.push_str(if p.shuffle { &c.grn } else { &c.w3 });
    o.push_str(" ⇄ ");
    o.push_str(if p.shuffle { "SHUF" } else { "shuf" });
    o.push_str(&c.w3);
    o.push_str("  ");
    o.push_str(if p.loop_on { &c.grn } else { &c.w3 });
    o.push_str(" ↺ ");
    o.push_str(if p.loop_on { "LOOP" } else { "loop" });
    o.push_str(&c.w2);
    // ponytail: fijo = 32 + dígitos; el resto se recorta al ancho (la línea nunca hace wrap).
    let fixed = 32 + nums.len();
    let budget = cols.saturating_sub(fixed);
    let ex = term::trunc(extra, budget);
    o.push_str(&" ".repeat(budget.saturating_sub(term::dw(&ex))));
    o.push_str(&c.w3);
    o.push_str(&ex);
    o.push_str(RST);
}
pub struct BD {
    pub dirs: Vec<String>,
    pub songs: Vec<String>,
}

pub fn scan(path: &str) -> BD {
    let mut d = BD {
        dirs: Vec::new(),
        songs: Vec::new(),
    };
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            let nm = e.file_name().to_string_lossy().into_owned();
            if nm.starts_with('.') {
                continue;
            }
            let ft = e.file_type().ok();
            if ft.is_some_and(|t| t.is_dir()) {
                d.dirs.push(nm);
            } else if ft.is_some_and(|t| t.is_file()) && tags::is_audio_name(&nm) {
                d.songs.push(nm);
            }
        }
    }
    d.dirs.sort_by_key(|s| s.to_lowercase());
    d.songs.sort_by_key(|s| s.to_lowercase());
    d
}

fn meta_row(p: &Player, i: usize, avail: usize) -> String {
    let m = p.meta(i);
    let title = if m.title.is_empty() {
        tags::strip_ext(&p.songs[i])
    } else {
        m.title.clone()
    };
    let dur = if m.duration > 0.0 {
        term::fmt_t(m.duration)
    } else {
        String::new()
    };
    let br = if m.bitrate > 0 {
        m.bitrate.to_string()
    } else {
        String::new()
    };
    let (mut aw, mut lw, mut bw, mut dw_) = (0usize, 0usize, 0usize, 6usize);
    if avail >= 34 {
        aw = ((avail as f64 * 0.22) as usize).clamp(8, 16);
    }
    if avail >= 44 {
        lw = ((avail as f64 * 0.22) as usize).clamp(8, 16);
    }
    if avail >= 58 {
        bw = 5;
    }
    for _ in 0..3 {
        if avail.saturating_sub(dw_ + bw + aw + lw) >= 10 {
            break;
        }
        if lw > 0 {
            lw = 0;
        } else if aw > 0 {
            aw = 0;
        } else if bw > 0 {
            bw = 0;
        } else {
            dw_ = 0;
        }
    }
    let tw = avail.saturating_sub(
        dw_ + bw + if aw > 0 { aw + 1 } else { 0 } + if lw > 0 { lw + 1 } else { 0 },
    );
    let g = "\x1b[38;2;130;128;118m";
    let mut o = String::with_capacity(avail * 2);
    if aw > 0 {
        o.push_str(g);
        o.push_str(&term::pad_r(&m.artist, aw));
        o.push(' ');
    }
    if lw > 0 {
        o.push_str(g);
        o.push_str(&term::pad_r(&m.album, lw));
        o.push(' ');
    }
    o.push_str(&term::pad_r(&title, tw));
    if dw_ > 0 {
        o.push_str(g);
        o.push_str(&term::pad_r(&dur, dw_));
    }
    if bw > 0 {
        o.push_str(g);
        o.push_str(&term::pad_r(&br, bw));
    }
    o
}

fn dir_name(d: &str) -> String {
    std::path::Path::new(d)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| d.to_string())
}

pub fn draw_player(p: &Player, t: &ThemeMgr, cc: &mut cover::Cache) {
    let TSz { cols, rows } = term::term_size();
    let content = rows.max(4) - 4;
    let show = cols >= 50;
    let (mut cw, mut ch) = (0, 0);
    if show {
        ch = content;
        cw = (ch * 2).min(cols.saturating_sub(29));
        if cw < 10 {
            cw = 0;
            ch = 0;
        } else {
            ch = cw / 2;
        }
    }
    let pw = cols - if cw > 0 { cw + 1 } else { 0 };
    let empty: Vec<String> = Vec::new();
    let lines: &[String] = if cw > 0 {
        cc.lines_for(&p.song_path(), cw, ch)
    } else {
        &empty
    };
    let total = p.songs.len();
    let mut start = p.row.saturating_sub(content / 2);
    if start + content > total {
        start = total.saturating_sub(content);
    }
    let c = &t.c;
    let mut o = String::with_capacity(cols * rows * 8);
    o.push_str("\x1b[?2026h\x1b[H\x1b[2J");
    o.push_str(HIDE);
    header(&mut o, cols, &dir_name(&p.dir), t);
    for r in 0..content {
        if cw > 0 {
            o.push_str(lines.get(r).map(|s| s.as_str()).unwrap_or(""));
            o.push_str(&c.w3);
            o.push('│');
            o.push_str(RST);
        }
        let i = start + r;
        if i < total {
            let s = &p.songs[i];
            let cur = i == p.row;
            let play = *s == p.playing_now;
            let disp = meta_row(p, i, pw.saturating_sub(5));
            if cur && play {
                o.push_str(&format!("{}{BOLD}  ▶  {disp}", c.bg_play));
            } else if cur {
                o.push_str(&format!("{}{BOLD}  ›  {disp}", c.bg_sel));
            } else if play {
                o.push_str(&format!("{}  ♪  {disp}", c.grn));
            } else {
                o.push_str(&format!("{}     {disp}", c.w3));
            }
        } else {
            o.push_str(RST);
            o.push_str(&" ".repeat(pw));
        }
        o.push_str(RST);
        o.push('\n');
    }
    let el = p.elapsed();
    let dur = p.duration();
    let now = if p.playing_now.is_empty() {
        "Stopped".to_string()
    } else {
        tags::strip_ext(&p.playing_now)
    };
    let e = term::fmt_t(el);
    let time_vis = if dur > 0.0 {
        e.len() + 3 + term::fmt_t(dur).len()
    } else {
        e.len()
    };
    let nw = cols.saturating_sub(time_vis + 2);
    let shown = term::trunc(&now, nw);
    o.push_str(&c.w1);
    o.push_str(BOLD);
    o.push_str(&shown);
    o.push_str(RST);
    o.push_str(&" ".repeat(nw.saturating_sub(term::dw(&shown))));
    o.push_str(&c.w2);
    o.push_str(&e);
    if dur > 0.0 {
        o.push_str(&c.w3);
        o.push_str(" / ");
        o.push_str(&c.w2);
        o.push_str(&term::fmt_t(dur));
    }
    o.push_str(RST);
    o.push('\n');
    if dur > 0.0 {
        let f = (el / dur * cols as f64) as usize;
        o.push_str(&c.grn);
        o.push_str(&"━".repeat(f.min(cols)));
        o.push_str(&c.w3);
        o.push_str(&"─".repeat(cols - f.min(cols)));
    } else {
        let sp = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
        o.push_str(&c.amb);
        o.push_str(sp[(el * 6.0) as usize % 8]);
        o.push_str(RST);
        o.push(' ');
        o.push_str(&c.w3);
        o.push_str(&"─".repeat(cols.saturating_sub(2)));
    }
    o.push_str(RST);
    o.push('\n');
    status(
        &mut o,
        cols,
        p,
        t,
        " · ↑/↓ nav · Enter play · Space pause · ←/→ seek · / search · l lyrics · o browse · a about · t theme · q quit",
    );
    o.push_str("\x1b[?2026l");
    term::emit(&o);
}

pub fn draw_lyrics(p: &Player, t: &ThemeMgr, scroll: usize) {
    let TSz { cols, rows } = term::term_size();
    let l = &p.lyrics;
    let cur = crate::lyrics::active(l, p.elapsed());
    let total = l.lines.len();
    let mut o = String::with_capacity(cols * rows * 4);
    o.push_str("\x1b[?2026h\x1b[H\x1b[2J");
    o.push_str(HIDE);
    header(&mut o, cols, "LYRICS", t);
    let title = if p.playing_now.is_empty() {
        "Stopped".to_string()
    } else {
        tags::strip_ext(&p.playing_now)
    };
    o.push_str(&t.c.w2);
    o.push_str(&term::center(
        &term::trunc(&title, cols.saturating_sub(4)),
        cols,
    ));
    o.push_str(RST);
    o.push('\n');
    let area = (rows as i64 - 4).max(1) as usize;
    let mut start = 0;
    if total > 0 {
        start = if l.synced {
            (cur - area as i64 / 2).clamp(0, (total as i64 - area as i64).max(0)) as usize
        } else {
            scroll.min(total.saturating_sub(area))
        };
    }
    for y in 0..area {
        let idx = start + y;
        if total == 0 && y == area / 2 {
            o.push_str(&t.c.w3);
            o.push_str(&term::center("no lyrics (.lrc or tags)", cols));
        } else if idx < total {
            let txt = &l.lines[idx].text;
            if l.synced && idx as i64 == cur {
                o.push_str(&format!(
                    "{}{BOLD} ▶ {}{RST}\n",
                    t.c.bg_play,
                    term::pad_r(txt, cols.saturating_sub(3))
                ));
            } else if l.synced && (idx as i64) < cur {
                o.push_str(&format!(
                    "{}   {}{RST}\n",
                    t.c.w3,
                    term::pad_r(txt, cols.saturating_sub(3))
                ));
            } else {
                o.push_str(&format!(
                    "{}   {}{RST}\n",
                    t.c.w2,
                    term::pad_r(txt, cols.saturating_sub(3))
                ));
            }
            continue;
        } else {
            o.push_str(&" ".repeat(cols));
        }
        o.push_str(RST);
        o.push('\n');
    }
    status(
        &mut o,
        cols,
        p,
        t,
        " · l lista · Space pause · ←/→ seek · n next · q quit",
    );
    o.push_str("\x1b[?2026l");
    term::emit(&o);
}

pub fn draw_search(p: &Player, t: &ThemeMgr, query: &str, matches: &[usize], mrow: usize) {
    let TSz { cols, rows } = term::term_size();
    let c = &t.c;
    let max_vis = rows.max(3) - 3;
    let total = matches.len();
    let mut start = 0;
    let mut end = 0;
    if total > 0 {
        start = mrow.saturating_sub(max_vis / 2);
        end = (start + max_vis).min(total);
        if end == total {
            start = total.saturating_sub(max_vis);
        }
    }
    let mut o = String::with_capacity(cols * rows * 8);
    o.push_str("\x1b[?2026h\x1b[H\x1b[2J");
    o.push_str(HIDE);
    header(&mut o, cols, "SEARCH", t);
    let input = format!(" 🔍  {}", term::trunc(query, cols.saturating_sub(8).max(1)));
    o.push_str(&c.bg_stat);
    o.push_str(&c.w3);
    o.push_str(&input);
    o.push_str(&c.grn);
    o.push_str(BOLD);
    o.push('▎');
    o.push_str(&c.w2);
    // ponytail: +2 cubre el emoji de 2 celdas; si es de 1, la línea queda 1 corta (nunca wrap).
    o.push_str(&" ".repeat(cols.saturating_sub(term::dw(&input) + 2)));
    o.push_str(RST);
    o.push('\n');
    for r in 0..max_vis {
        let i = start + r;
        if i < end {
            let s = &p.songs[matches[i]];
            let cur = i == mrow;
            let play = *s == p.playing_now;
            if cur && play {
                o.push_str(&c.bg_play);
                o.push_str(BOLD);
                o.push_str("  ▶  ");
            } else if cur {
                o.push_str(&c.bg_sel);
                o.push_str(BOLD);
                o.push_str("  ›  ");
            } else if play {
                o.push_str(&c.grn);
                o.push_str("  ♪  ");
            } else {
                o.push_str(&c.w3);
                o.push_str("     ");
            }
            o.push_str(&term::pad_r(&tags::strip_ext(s), cols.saturating_sub(5)));
        } else {
            o.push_str(RST);
            o.push_str(&" ".repeat(cols));
        }
        o.push_str(RST);
        o.push('\n');
    }
    o.push_str(&c.bg_stat);
    if total == 0 {
        o.push_str(&c.amb);
        o.push_str("  no matches");
        o.push_str(&" ".repeat(cols.saturating_sub(12)));
    } else {
        let left = format!("  {}/{}", mrow + 1, total);
        let right = " ↑/↓ select · Enter play · Esc cancel";
        o.push_str(&c.w2);
        o.push_str(&left);
        // ponytail: hints recortadas al ancho como en status() (nunca wrap).
        let budget = cols.saturating_sub(left.len());
        let rshow = term::trunc(right, budget);
        o.push_str(&" ".repeat(budget.saturating_sub(term::dw(&rshow))));
        o.push_str(&c.w3);
        o.push_str(&rshow);
    }
    o.push_str(RST);
    o.push_str("\x1b[?2026l");
    term::emit(&o);
}

pub fn draw_browser(path: &str, d: &BD, t: &ThemeMgr, cur: usize, scr: usize) {
    let TSz { cols, rows } = term::term_size();
    let c = &t.c;
    let max_vis = rows.max(9) - 9;
    let total = d.dirs.len() + d.songs.len();
    let mut o = String::with_capacity(cols * rows * 6);
    o.push_str("\x1b[?2026h\x1b[H\x1b[2J");
    o.push_str(HIDE);
    o.push_str(&c.bg_hdr);
    o.push_str(BOLD);
    o.push_str(&format!(
        "  {}♪   RMUSTP{}  📁  {}File Browser",
        c.grn, c.w1, c.w3
    ));
    o.push_str(&c.w2);
    o.push_str(&" ".repeat(cols.saturating_sub(30)));
    o.push_str(RST);
    o.push('\n');
    o.push_str(&format!(
        "{} {}{RST}\n",
        c.w3,
        term::trunc(path, cols.saturating_sub(2))
    ));
    if d.songs.is_empty() {
        o.push_str(&format!(" {}no audio files{RST}\n", c.w3));
    } else {
        o.push_str(&format!(
            " {}{}  {} track{}{RST}\n",
            c.grn,
            "♪",
            d.songs.len(),
            if d.songs.len() > 1 { "s" } else { "" }
        ));
    }
    o.push_str(&format!("{}{}{RST}\n", c.w3, "─".repeat(cols)));
    let end = (scr + max_vis).min(total);
    for i in scr..end {
        let sel = i == cur;
        if i < d.dirs.len() {
            let nm = &d.dirs[i];
            let mut n = 0;
            if let Ok(rd) = std::fs::read_dir(format!("{path}/{nm}")) {
                for e in rd.flatten() {
                    let s = e.file_name().to_string_lossy().into_owned();
                    if e.file_type().map(|x| x.is_file()).unwrap_or(false)
                        && tags::is_audio_name(&s)
                    {
                        n += 1;
                    }
                }
            }
            let badge = if n > 0 {
                format!(" {}{n}{RST}", c.grn)
            } else {
                String::new()
            };
            let bw = if n > 0 { 2 + n.to_string().len() } else { 0 };
            if sel {
                o.push_str(&format!(
                    "{}{BOLD}  ›  📂 {}{badge}",
                    c.bg_sel,
                    term::pad_r(nm, cols.saturating_sub(8 + bw))
                ));
            } else {
                o.push_str(&format!(
                    "{}     📁 {}{badge}",
                    c.w3,
                    term::pad_r(nm, cols.saturating_sub(8 + bw))
                ));
            }
        } else {
            let disp = tags::strip_ext(&d.songs[i - d.dirs.len()]);
            if sel {
                o.push_str(&format!(
                    "{}{}{BOLD}  ›  ≈ {}",
                    c.bg_sel,
                    c.grn,
                    term::pad_r(&disp, cols.saturating_sub(9))
                ));
            } else {
                o.push_str(&format!(
                    "{}     ≈ {}",
                    c.w2,
                    term::pad_r(&disp, cols.saturating_sub(9))
                ));
            }
        }
        o.push_str(RST);
        o.push('\n');
    }
    for _ in (end - scr)..max_vis {
        o.push_str("\x1b[K\n");
    }
    if total == 0 {
        o.push_str(&format!("{}  (empty){RST}\n", c.w3));
    }
    if total > max_vis {
        o.push_str(&format!(
            "{}  {}-{} of {}{RST}\n",
            c.w3,
            scr + 1,
            end,
            total
        ));
    }
    o.push_str(&format!("{}{}{RST}\n", c.w3, "─".repeat(cols)));
    o.push_str(&c.bg_stat);
    // ponytail: en terminales estrechas sobran pares (nunca wrap); el resto se rellena.
    let mut used = 0;
    for (k, v) in [
        ("↑↓", "nav"),
        ("→/Enter", "open"),
        ("←/Bksp", "up"),
        ("o", "home"),
        ("q", "cancel"),
    ] {
        let w = 2 + term::dw(k) + term::dw(v);
        if used + w > cols {
            break;
        }
        o.push_str(&format!(" {}{} {}{}", c.w1, k, c.w3, v));
        used += w;
    }
    o.push_str(&" ".repeat(cols.saturating_sub(used)));
    o.push_str(RST);
    o.push_str("\x1b[?2026l");
    term::emit(&o);
}

pub fn draw_about(t: &ThemeMgr) {
    let TSz { cols, rows } = term::term_size();
    let c = &t.c;
    let body: Vec<String> = vec![
        "Rust terminal music player (POSIX)".to_string(),
        String::new(),
        "decode  rodio (mp3 flac wav ogg ...)".to_string(),
        "cover   embedded + cover.jpg/png (ANSI)".to_string(),
        "lyrics  .lrc synced + embedded tags".to_string(),
        "themes  9 built-in".to_string(),
        "audio   ALSA (Linux) / CoreAudio (macOS)".to_string(),
        String::new(),
        format!("v{} · MIT", env!("CARGO_PKG_VERSION")),
    ];
    let mut iw = 0;
    for l in &body {
        iw = iw.max(term::dw(l));
    }
    let bw = (iw.max(8) + 4).min(cols.saturating_sub(4).max(8));
    let bh = body.len() + 6;
    let top = rows.saturating_sub(bh) / 2;
    let left = cols.saturating_sub(bw) / 2;
    let mut o = String::with_capacity(cols * bh * 4);
    o.push_str("\x1b[H\x1b[2J");
    o.push_str(HIDE);
    o.push_str(&"\n".repeat(top));
    let edge = |o: &mut String, l: &str, r: &str, mid: &str| {
        o.push_str(&" ".repeat(left));
        o.push_str(&c.grn);
        o.push_str(BOLD);
        o.push_str(l);
        o.push_str(mid);
        o.push_str(r);
        o.push_str(RST);
        o.push('\n');
    };
    edge(&mut o, "╭", "╮", &"─".repeat(bw - 2));
    o.push_str(&format!(
        "{}{}{BOLD}│{}{}{}│{RST}\n",
        " ".repeat(left),
        c.grn,
        c.w1,
        term::center("RMUSTP", bw - 2),
        c.grn
    ));
    edge(&mut o, "├", "┤", &"─".repeat(bw - 2));
    for l in &body {
        // ponytail: center ya recorta si sobra (trunc previo redundante).
        let mid = if l.is_empty() {
            " ".repeat(bw - 2)
        } else {
            term::center(l, bw - 2)
        };
        o.push_str(&format!(
            "{}{}{BOLD}│{}{}{}{}│{RST}\n",
            " ".repeat(left),
            c.grn,
            c.w2,
            mid,
            c.grn,
            ""
        ));
    }
    edge(&mut o, "├", "┤", &"─".repeat(bw - 2));
    o.push_str(&format!(
        "{}{}{BOLD}│{}{}{}│{RST}\n",
        " ".repeat(left),
        c.grn,
        c.amb,
        term::center("Press any key to close", bw - 2),
        c.grn
    ));
    edge(&mut o, "╰", "╯", &"─".repeat(bw - 2));
    if top + bh < rows {
        o.push_str(&"\n".repeat(rows - top - bh));
    }
    term::emit(&o);
}

pub fn browse(raw: &term::Raw, t: &ThemeMgr) -> String {
    let mut cur = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    cur = std::fs::canonicalize(&cur)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or(cur);
    let (mut cursor, mut scroll) = (0, 0);
    let mut d = scan(&cur);
    draw_browser(&cur, &d, t, cursor, scroll);
    let mut last = term::term_size();
    loop {
        let key = raw.read_key(-1.0);
        let sz = term::term_size();
        let resized = sz.cols != last.cols || sz.rows != last.rows;
        last = sz;
        if key.is_empty() && !resized {
            continue;
        }
        let max_vis = (sz.rows.max(9) - 9).max(1);
        let total = d.dirs.len() + d.songs.len();
        let mut redraw = true;
        if key == "\u{1b}[A" || key == "k" {
            if total > 0 {
                cursor = (cursor + total - 1) % total;
            }
        } else if key == "\u{1b}[B" || key == "j" {
            if total > 0 {
                cursor = (cursor + 1) % total;
            }
        } else if key == "\u{1b}[C" || key == "\r" || key == "l" {
            if total == 0 {
                redraw = resized;
            } else if cursor < d.dirs.len() {
                let nxt = format!("{cur}/{}", d.dirs[cursor]);
                cur = std::fs::canonicalize(&nxt)
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or(nxt);
                d = scan(&cur);
                cursor = 0;
                scroll = 0;
                if d.dirs.is_empty() && !d.songs.is_empty() {
                    term::emit(&format!("\x1b[H\x1b[2J{SHOW}"));
                    return cur;
                }
            } else {
                term::emit(&format!("\x1b[H\x1b[2J{SHOW}"));
                return cur;
            }
        } else if key == "\u{1b}[D" || key == "\u{7f}" || key == "h" {
            if let Some(par) = std::path::Path::new(&cur)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
            {
                if !par.is_empty() && par != cur {
                    cur = par;
                    d = scan(&cur);
                    cursor = 0;
                    scroll = 0;
                } else {
                    redraw = resized;
                }
            }
        } else if key == "o" || key == "O" {
            cur = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
            cur = std::fs::canonicalize(&cur)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or(cur);
            d = scan(&cur);
            cursor = 0;
            scroll = 0;
        } else if key == "q" || key == "Q" {
            term::emit(&format!("\x1b[H\x1b[2J{SHOW}"));
            return String::new();
        } else {
            redraw = resized;
        }
        if total > 0 {
            cursor = cursor.min(total - 1);
        } else {
            cursor = 0;
        }
        if cursor < scroll {
            scroll = cursor;
        }
        if cursor >= scroll + max_vis {
            scroll = cursor - max_vis + 1;
        }
        if redraw || resized {
            draw_browser(&cur, &d, t, cursor, scroll);
        }
    }
}
