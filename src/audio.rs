// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    lyrics::{self, Lyrics},
    tags,
};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

fn probe(path: &str) -> (f64, u32) {
    // ponytail: un solo open; el tamaño sale del fd (fstat), no de un segundo stat al path.
    let Ok(f) = File::open(path) else {
        return (0.0, 0);
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let dur = Decoder::new(BufReader::new(f))
        .ok()
        .and_then(|d| d.total_duration())
        .map(|x| x.as_secs_f64())
        .unwrap_or(0.0);
    let br = if dur > 0.0 {
        len as f64 * 8.0 / dur / 1000.0 + 0.5
    } else {
        0.0
    } as u32;
    (dur, br)
}

// ponytail: RSS ~18MB base + ~12MB con audio (rodio/cpal/ALSA). Upgrade: decoder PCM-only sin rodio.
pub struct Player {
    _stream: OutputStream,
    sink: Sink,
    pub songs: Vec<String>,
    pub dir: String,
    pub playing_now: String,
    pub row: usize,
    pub volume: f32,
    pub paused: bool,
    pub shuffle: bool,
    pub loop_on: bool,
    pub lyrics: Lyrics,
    dur: f64,
    soff: f64,
    wall: Instant,
    rng: u64,
    metas: Arc<Mutex<Vec<tags::TrackMeta>>>,
    ready: Arc<AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Player {
    pub fn new() -> Self {
        let (stream, h) = OutputStream::try_default().unwrap_or_else(|e| {
            eprintln!("rmustp: cannot open audio device ({e})");
            std::process::exit(1);
        });
        let sink = Sink::try_new(&h).unwrap_or_else(|e| {
            eprintln!("rmustp: cannot create sink ({e})");
            std::process::exit(1);
        });
        sink.set_volume(0.5);
        Player {
            _stream: stream,
            sink,
            songs: Vec::new(),
            dir: String::new(),
            playing_now: String::new(),
            row: 0,
            volume: 0.5,
            paused: false,
            shuffle: false,
            loop_on: false,
            lyrics: Lyrics::default(),
            dur: 0.0,
            soff: 0.0,
            wall: Instant::now(),
            rng: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1),
            metas: Arc::new(Mutex::new(Vec::new())),
            ready: Arc::new(AtomicUsize::new(0)),
            handle: None,
        }
    }

    // ponytail: LCG, no el crate `rand`; calidad sobra para shuffle.
    fn rnd(&mut self, n: usize) -> usize {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.rng >> 33) as usize) % n.max(1)
    }

    pub fn song_path(&self) -> String {
        if self.playing_now.is_empty() {
            String::new()
        } else {
            format!("{}/{}", self.dir, self.playing_now)
        }
    }

    pub fn meta(&self, i: usize) -> tags::TrackMeta {
        if i < self.ready.load(Ordering::Acquire) {
            if let Ok(v) = self.metas.lock() {
                if i < v.len() {
                    return v[i].clone();
                }
            }
        }
        tags::TrackMeta::default()
    }

    pub fn meta_done(&self) -> bool {
        self.ready.load(Ordering::Acquire) >= self.songs.len()
    }

    pub fn elapsed(&self) -> f64 {
        if self.playing_now.is_empty() {
            0.0
        } else if self.paused {
            self.soff
        } else {
            self.soff + self.wall.elapsed().as_secs_f64()
        }
    }

    pub fn duration(&self) -> f64 {
        self.dur
    }

    pub fn is_ended(&self) -> bool {
        !self.playing_now.is_empty() && self.sink.empty()
    }

    fn start_file(&mut self, path: &str) -> bool {
        let Ok(f) = File::open(path) else {
            return false;
        };
        let Ok(dec) = Decoder::new(BufReader::new(f)) else {
            return false;
        };
        self.dur = dec.total_duration().map(|d| d.as_secs_f64()).unwrap_or(0.0);
        self.sink.stop();
        self.sink.append(dec);
        self.sink.set_volume(self.volume);
        self.sink.play();
        self.soff = 0.0;
        self.wall = Instant::now();
        self.lyrics = lyrics::resolve(path);
        true
    }

    pub fn play_current(&mut self) -> bool {
        if self.songs.is_empty() {
            return false;
        }
        self.row = self.row.min(self.songs.len() - 1);
        self.playing_now = self.songs[self.row].clone();
        self.paused = false;
        let p = self.song_path();
        if !self.start_file(&p) {
            eprintln!("rmustp: cannot decode {}", self.playing_now);
            return false;
        }
        true
    }

    pub fn toggle_pause(&mut self) {
        if self.playing_now.is_empty() {
            return;
        }
        if self.paused {
            self.wall = Instant::now();
            self.paused = false;
            self.sink.play();
        } else {
            self.soff = self.elapsed();
            self.paused = true;
            self.sink.pause();
        }
    }

    pub fn seek(&mut self, delta: f64) {
        if self.playing_now.is_empty() {
            return;
        }
        let tgt = (self.elapsed() + delta).clamp(
            0.0,
            if self.dur > 0.0 {
                (self.dur - 0.5).max(0.0)
            } else {
                0.0
            },
        );
        // ponytail: try_seek donde el formato lo permite; si falla, seguimos donde estábamos.
        let _ = self.sink.try_seek(Duration::from_secs_f64(tgt));
        self.soff = tgt;
        self.wall = Instant::now();
    }

    pub fn change_vol(&mut self, d: f32) {
        self.volume = (self.volume + d).clamp(0.0, 1.0);
        self.sink.set_volume(self.volume);
    }

    pub fn next_song(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        if self.loop_on {
            if self.play_current() {
                return;
            }
        } else {
            // ponytail: un intento por canción como máximo; si todo falla, stop (no spin).
            for _ in 0..self.songs.len() {
                self.row = if self.shuffle {
                    self.rnd(self.songs.len())
                } else {
                    (self.row + 1) % self.songs.len()
                };
                if self.play_current() {
                    return;
                }
            }
        }
        self.stop_all();
    }

    pub fn prev_song(&mut self) {
        if self.songs.is_empty() {
            return;
        }
        if self.loop_on {
            self.play_current();
            return;
        }
        self.row = if self.shuffle {
            self.rnd(self.songs.len())
        } else {
            (self.row + self.songs.len() - 1) % self.songs.len()
        };
        self.play_current();
    }

    pub fn stop_all(&mut self) {
        self.sink.stop();
        self.playing_now.clear();
        self.paused = false;
        self.lyrics = Lyrics::default();
    }

    pub fn load_dir(&mut self, path: &str) {
        self.dir = path.to_string();
        self.songs.clear();
        self.row = 0;
        if let Ok(rd) = std::fs::read_dir(path) {
            for e in rd.flatten() {
                let nm = e.file_name().to_string_lossy().into_owned();
                if nm.starts_with('.') {
                    continue;
                }
                if e.file_type().map(|t| t.is_file()).unwrap_or(false) && tags::is_audio_name(&nm) {
                    self.songs.push(nm);
                }
            }
        }
        self.songs.sort_by_key(|s| s.to_lowercase());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        if let Ok(mut v) = self.metas.lock() {
            *v = vec![tags::TrackMeta::default(); self.songs.len()];
        }
        self.ready.store(0, Ordering::Release);
        if !self.songs.is_empty() {
            let songs = self.songs.clone();
            let dir = self.dir.clone();
            let metas = Arc::clone(&self.metas);
            let ready = Arc::clone(&self.ready);
            self.handle = Some(std::thread::spawn(move || {
                for (i, s) in songs.iter().enumerate() {
                    let fp = format!("{dir}/{s}");
                    let mut m = tags::read_tags(&fp).unwrap_or_default();
                    let (d, b) = probe(&fp);
                    m.duration = d;
                    m.bitrate = b;
                    if let Ok(mut v) = metas.lock() {
                        if i < v.len() {
                            v[i] = m;
                        }
                    }
                    ready.store(i + 1, Ordering::Release);
                }
            }));
        }
    }
}
