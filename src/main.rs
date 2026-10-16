mod audio;
mod cover;
mod lyrics;
mod tags;
mod term;
mod theme;
mod ui;

#[cfg(windows)]
compile_error!("rmustp is POSIX-only (Linux/macOS); Windows is not supported");

use std::io::IsTerminal;
use std::time::{Duration, Instant};
use term::Raw;
use theme::ThemeMgr;

fn main() {
    let a = std::env::args().nth(1).unwrap_or_default();
    if a == "-h" || a == "--help" {
        term::emit("rmustp 1.0.0 — POSIX terminal music player\nuso: rmustp\n");
        std::process::exit(0);
    }
    if a == "-v" || a == "--version" {
        term::emit(&format!("rmustp {}\n", env!("CARGO_PKG_VERSION")));
        std::process::exit(0);
    }
    // ponytail: sin TTY no hay TUI; salir limpio en vez de spin en select/read.
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("rmustp: stdin/stdout must be a TTY");
        std::process::exit(2);
    }
    let mut themes = ThemeMgr::load();
    let raw = Raw::new();
    term::install_signals();
    term::emit("\x1b[?1049h\x1b[H\x1b[2J");

    let mut sel = ui::browse(&raw, &themes);
    if sel.is_empty() {
        term::emit("\x1b[?1049l\x1b[?25h");
        return;
    }
    let mut p = audio::Player::new();
    p.load_dir(&sel);
    let mut cc = cover::Cache::new();
    let mut scroll_song = String::new();

    let mut lyrics_mode = false;
    let mut lyr_scroll = 0usize;
    // Firma con args (no captura nada mutable) para poder mutar lyr_scroll en el loop.
    let draw =
        |p: &audio::Player, cc: &mut cover::Cache, lm: bool, ls: usize, themes: &ThemeMgr| {
            if lm {
                ui::draw_lyrics(p, themes, ls);
            } else {
                ui::draw_player(p, themes, cc);
            }
        };
    draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);

    let mut searching = false;
    let mut query = String::new();
    let mut matches: Vec<usize> = Vec::new();
    let mut mrow = 0usize;
    let rebuild = |p: &audio::Player, query: &str, matches: &mut Vec<usize>, mrow: &mut usize| {
        matches.clear();
        let q = query.to_lowercase();
        for (i, s) in p.songs.iter().enumerate() {
            if s.to_lowercase().contains(&q) {
                matches.push(i);
            }
        }
        if matches.is_empty() {
            *mrow = 0;
        } else if *mrow >= matches.len() {
            *mrow = matches.len() - 1;
        }
    };

    let mut last_draw = Instant::now();
    let mut last_sz = term::term_size();
    const DRAW: Duration = Duration::from_millis(125);

    loop {
        if p.playing_now != scroll_song {
            scroll_song = p.playing_now.clone();
            lyr_scroll = 0;
        }
        if p.songs.is_empty() {
            term::emit("\x1b[H\x1b[2J\x1b[?25h");
            sel = ui::browse(&raw, &themes);
            if !sel.is_empty() {
                p.load_dir(&sel);
                draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);
                last_draw = Instant::now();
            } else {
                break;
            }
            continue;
        }
        if !p.playing_now.is_empty() && !p.paused && p.is_ended() {
            p.next_song();
            draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);
            last_draw = Instant::now();
            continue;
        }
        if searching {
            let k = raw.read_key(-1.0);
            let mut leave = false;
            if k == "\u{1b}" || k == "\u{3}" || k == "\u{4}" {
                leave = true;
            } else if k == "\u{7f}" {
                if !query.is_empty() {
                    query.pop();
                    rebuild(&p, &query, &mut matches, &mut mrow);
                }
            } else if k == "\u{1b}[A" || k == "k" {
                if !matches.is_empty() {
                    mrow = (mrow + matches.len() - 1) % matches.len();
                }
            } else if k == "\u{1b}[B" || k == "j" {
                if !matches.is_empty() {
                    mrow = (mrow + 1) % matches.len();
                }
            } else if k == "\r" {
                if let Some(&i) = matches.get(mrow) {
                    p.row = i;
                    p.play_current();
                }
                leave = true;
            // ponytail: búsqueda ASCII imprimible; no UTF-8 multibyte.
            } else if k.len() == 1 && matches!(k.as_bytes()[0], 0x20..=0x7e) {
                query += &k;
                rebuild(&p, &query, &mut matches, &mut mrow);
            }
            if leave {
                searching = false;
                draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);
            } else {
                ui::draw_search(&p, &themes, &query, &matches, mrow);
            }
            last_draw = Instant::now();
            continue;
        }

        let key = raw.read_key(0.12);
        let sz = term::term_size();
        let resized = sz.cols != last_sz.cols || sz.rows != last_sz.rows;
        if resized {
            last_sz = sz;
        }
        if key.is_empty() {
            // Las tres ramas hacían lo mismo: dibujar si toca. Una sola condición.
            let due = last_draw.elapsed() >= DRAW;
            let meta_due = !p.meta_done() && last_draw.elapsed() >= Duration::from_millis(250);
            if resized || (!p.playing_now.is_empty() && !p.paused && due) || meta_due {
                draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);
                last_draw = Instant::now();
            }
            continue;
        }

        let mut redraw = true;
        let n = p.songs.len();
        if key == "\u{1b}[A" || key == "k" {
            if lyrics_mode {
                if !p.lyrics.synced {
                    lyr_scroll = lyr_scroll.saturating_sub(1);
                }
            } else {
                p.row = (p.row + n - 1) % n;
            }
        } else if key == "\u{1b}[B" || key == "j" {
            if lyrics_mode {
                if !p.lyrics.synced {
                    lyr_scroll += 1;
                }
            } else {
                p.row = (p.row + 1) % n;
            }
        } else if key == "\r" {
            p.play_current();
        } else if key == "n" || key == "N" {
            p.next_song();
        } else if key == "p" || key == "P" {
            p.prev_song();
        } else if key == " " {
            p.toggle_pause();
        } else if key == "\u{1b}[D" || key == "h" {
            p.seek(-5.0);
        } else if key == "\u{1b}[C" {
            p.seek(5.0);
        } else if key == "+" || key == "=" {
            p.change_vol(0.05);
        } else if key == "-" || key == "_" {
            p.change_vol(-0.05);
        } else if key == "s" || key == "S" {
            p.shuffle = !p.shuffle;
        } else if key == "r" || key == "R" {
            p.loop_on = !p.loop_on;
        } else if key == "l" || key == "L" {
            lyrics_mode = !lyrics_mode;
            lyr_scroll = 0;
        } else if key == "t" || key == "T" {
            themes.apply(themes.idx + 1);
        } else if key == "o" || key == "O" {
            term::emit("\x1b[H\x1b[2J\x1b[?25h");
            let s2 = ui::browse(&raw, &themes);
            if !s2.is_empty() {
                p.load_dir(&s2);
            }
        } else if key == "a" || key == "A" {
            ui::draw_about(&themes);
            raw.read_key(-1.0);
        } else if key == "/" {
            searching = true;
            query.clear();
            mrow = p.row;
            rebuild(&p, &query, &mut matches, &mut mrow);
            ui::draw_search(&p, &themes, &query, &matches, mrow);
            last_draw = Instant::now();
            redraw = false;
        } else if key == "q" || key == "Q" {
            break;
        } else {
            redraw = resized;
        }

        if redraw {
            draw(&p, &mut cc, lyrics_mode, lyr_scroll, &themes);
            last_draw = Instant::now();
        }
    }

    p.stop_all();
    term::emit("\x1b[?1049l\x1b[?25h");
}
