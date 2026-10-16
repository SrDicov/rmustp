use std::io::Write;
use std::os::unix::io::RawFd;
use std::sync::Mutex;

static SAVED: Mutex<Option<libc::termios>> = Mutex::new(None);

extern "C" fn on_term(_sig: i32) {
    // ponytail: restore best-effort + _exit; tcsetattr en handler no es 100% async-signal-safe.
    if let Ok(g) = SAVED.lock() {
        if let Some(t) = g.as_ref() {
            // SAFETY: tcsetattr con el termios guardado; unico writer en el hilo main.
            unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, t) };
        }
    }
    // SAFETY: _exit nunca retorna; restauracion de termios ya realizada.
    unsafe { libc::_exit(143) };
}

pub fn install_signals() {
    // SAFETY: sigaction con handler sin datos; restaura el termios guardado.
    unsafe {
        libc::signal(libc::SIGTERM, on_term as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_term as *const () as libc::sighandler_t);
    }
    // ponytail: el hook corre también con panic="abort" (el abort salta Drops, no el hook);
    // restaura el terminal donde el Drop ya no llega. Encadena el hook previo (mensaje).
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Ok(g) = SAVED.lock() {
            if let Some(t) = g.as_ref() {
                // SAFETY: contexto de pánico normal (no señal); tcsetattr best-effort.
                unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, t) };
            }
        }
        prev(info);
    }));
}

#[derive(Clone, Copy)]
pub struct TSz {
    pub cols: usize,
    pub rows: usize,
}

pub fn term_size() -> TSz {
    // SAFETY: ioctl con TIOCGWINSZ sobre un struct local; solo lectura.
    unsafe {
        let mut w: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut w) == 0
            && w.ws_col > 10
            && w.ws_row > 5
        {
            return TSz {
                cols: w.ws_col as usize,
                rows: w.ws_row as usize,
            };
        }
    }
    TSz { cols: 80, rows: 24 }
}

pub struct Raw {
    fd: RawFd,
    saved: libc::termios,
}

impl Raw {
    pub fn new() -> Self {
        // SAFETY: tcgetattr guarda el estado; cfmakeraw solo toca la copia local.
        unsafe {
            let fd = libc::STDIN_FILENO;
            let mut saved: libc::termios = std::mem::zeroed();
            libc::tcgetattr(fd, &mut saved);
            let mut r = saved;
            libc::cfmakeraw(&mut r);
            libc::tcsetattr(fd, libc::TCSANOW, &r);
            if let Ok(mut g) = SAVED.lock() {
                *g = Some(saved);
            }
            Raw { fd, saved }
        }
    }

    pub fn read_key(&self, timeout: f64) -> String {
        if Self::wait(self.fd, timeout) <= 0 {
            return String::new();
        }
        let mut b = [0u8; 1];
        // SAFETY: read a un buffer local de 1 byte; n<=0 = nada.
        let n = unsafe { libc::read(self.fd, b.as_mut_ptr() as *mut _, 1) };
        if n != 1 {
            return String::new();
        }
        if b[0] != 0x1b {
            return String::from_utf8_lossy(&b).into_owned();
        }
        let mut seq = vec![0x1bu8];
        for _ in 0..6 {
            if Self::wait(self.fd, 0.05) <= 0 {
                break;
            }
            // SAFETY: igual que arriba.
            let n = unsafe { libc::read(self.fd, b.as_mut_ptr() as *mut _, 1) };
            if n != 1 {
                break;
            }
            seq.push(b[0]);
            if b[0].is_ascii_alphabetic() || b[0] == b'~' {
                break;
            }
        }
        String::from_utf8_lossy(&seq).into_owned()
    }

    fn wait(fd: RawFd, timeout: f64) -> i32 {
        // SAFETY: select sobre un único fd con structs locales.
        unsafe {
            let mut fds: libc::fd_set = std::mem::zeroed();
            libc::FD_ZERO(&mut fds);
            libc::FD_SET(fd, &mut fds);
            if timeout < 0.0 {
                return libc::select(
                    fd + 1,
                    &mut fds,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            let mut tv = libc::timeval {
                tv_sec: timeout as i64,
                tv_usec: ((timeout - timeout.floor()) * 1e6) as i64,
            };
            libc::select(
                fd + 1,
                &mut fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            )
        }
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        // SAFETY: restaura el termios guardado al entrar.
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.saved);
        }
    }
}

pub fn emit(s: &str) {
    // ponytail: único escritor stdout (write_all+flush, ignora errores); el CLI lo reusa:
    // cero println! en todo el árbol con una sola primitiva.
    let mut o = std::io::stdout();
    let _ = o.write_all(s.as_bytes());
    let _ = o.flush();
}

// ponytail: sin unicode-width a propósito; CJK de ancho doble desalinea columnas.
// Upgrade: unicode-width (+1 dep, ~+50KB). ASCII/latín exactos sin ella.
pub fn dw(s: &str) -> usize {
    s.chars().count()
}

pub fn trunc(s: &str, lim: usize) -> String {
    if lim == 0 {
        return String::new();
    }
    if dw(s) < lim {
        return s.to_string();
    }
    let mut o = String::new();
    for (w, c) in s.chars().enumerate() {
        if w + 1 >= lim {
            o.push('…');
            break;
        }
        o.push(c);
    }
    o
}

pub fn pad_r(s: &str, w: usize) -> String {
    let d = dw(s);
    if d >= w {
        trunc(s, w)
    } else {
        format!("{s}{}", " ".repeat(w - d))
    }
}

pub fn center(s: &str, w: usize) -> String {
    let d = dw(s);
    if d >= w {
        return trunc(s, w);
    }
    let lp = (w - d) / 2;
    format!("{}{s}{}", " ".repeat(lp), " ".repeat(w - d - lp))
}

pub fn fmt_t(s: f64) -> String {
    let v = s.max(0.0) as u64;
    format!("{}:{:02}", v / 60, v % 60)
}
