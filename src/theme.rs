// SPDX-License-Identifier: GPL-3.0-only
use std::fs;

pub struct Def {
    pub name: &'static str,
    pub fg: [[u8; 3]; 4],
    pub acc: [u8; 3],
    pub warn: [u8; 3],
    pub bghdr: [[u8; 3]; 2],
    pub bgsel: [[u8; 3]; 2],
    pub bgplay: [[u8; 3]; 2],
    pub bgstat: [[u8; 3]; 2],
}

// ponytail: 9 fijos en código; XML se eliminó. Añadir temas = una línea aquí.
pub const THEMES: &[Def] = &[
    Def {
        name: "default",
        fg: [
            [230, 230, 220],
            [180, 178, 168],
            [130, 128, 118],
            [85, 83, 75],
        ],
        acc: [140, 200, 140],
        warn: [210, 170, 70],
        bghdr: [[22, 22, 18], [190, 185, 175]],
        bgsel: [[50, 50, 44], [255, 255, 245]],
        bgplay: [[20, 42, 20], [140, 200, 140]],
        bgstat: [[14, 14, 12], [140, 135, 125]],
    },
    Def {
        name: "catppuccin",
        fg: [
            [205, 214, 244],
            [166, 173, 200],
            [108, 112, 134],
            [88, 91, 112],
        ],
        acc: [166, 227, 161],
        warn: [249, 226, 175],
        bghdr: [[30, 30, 46], [180, 190, 254]],
        bgsel: [[49, 50, 68], [205, 214, 244]],
        bgplay: [[30, 45, 30], [166, 227, 161]],
        bgstat: [[17, 17, 27], [108, 112, 134]],
    },
    Def {
        name: "dracula",
        fg: [
            [248, 248, 242],
            [189, 147, 249],
            [98, 114, 164],
            [68, 71, 90],
        ],
        acc: [80, 250, 123],
        warn: [241, 250, 140],
        bghdr: [[40, 42, 54], [189, 147, 249]],
        bgsel: [[68, 71, 90], [248, 248, 242]],
        bgplay: [[30, 50, 35], [80, 250, 123]],
        bgstat: [[21, 22, 30], [98, 114, 164]],
    },
    Def {
        name: "nord",
        fg: [
            [236, 239, 244],
            [216, 222, 233],
            [136, 192, 208],
            [76, 86, 106],
        ],
        acc: [163, 190, 140],
        warn: [235, 203, 139],
        bghdr: [[46, 52, 64], [136, 192, 208]],
        bgsel: [[67, 76, 94], [236, 239, 244]],
        bgplay: [[35, 50, 40], [163, 190, 140]],
        bgstat: [[36, 41, 51], [76, 86, 106]],
    },
    Def {
        name: "gruvbox",
        fg: [
            [235, 219, 178],
            [214, 196, 155],
            [168, 153, 132],
            [102, 92, 84],
        ],
        acc: [184, 187, 38],
        warn: [215, 153, 33],
        bghdr: [[40, 40, 40], [168, 153, 132]],
        bgsel: [[60, 56, 54], [235, 219, 178]],
        bgplay: [[30, 40, 20], [184, 187, 38]],
        bgstat: [[29, 32, 33], [102, 92, 84]],
    },
    Def {
        name: "rosepine",
        fg: [
            [224, 222, 244],
            [196, 167, 231],
            [144, 122, 169],
            [78, 70, 100],
        ],
        acc: [235, 188, 186],
        warn: [246, 193, 119],
        bghdr: [[25, 23, 36], [144, 122, 169]],
        bgsel: [[42, 39, 63], [224, 222, 244]],
        bgplay: [[38, 25, 40], [235, 188, 186]],
        bgstat: [[18, 16, 26], [78, 70, 100]],
    },
    Def {
        name: "tokyonight",
        fg: [
            [192, 202, 245],
            [169, 177, 214],
            [86, 95, 137],
            [65, 72, 104],
        ],
        acc: [115, 218, 202],
        warn: [224, 175, 104],
        bghdr: [[26, 27, 38], [86, 95, 137]],
        bgsel: [[40, 42, 60], [192, 202, 245]],
        bgplay: [[20, 40, 38], [115, 218, 202]],
        bgstat: [[16, 17, 26], [65, 72, 104]],
    },
    Def {
        name: "everforest",
        fg: [
            [211, 198, 170],
            [167, 192, 128],
            [131, 165, 152],
            [92, 110, 95],
        ],
        acc: [167, 192, 128],
        warn: [219, 188, 127],
        bghdr: [[37, 42, 37], [131, 165, 152]],
        bgsel: [[57, 64, 55], [211, 198, 170]],
        bgplay: [[30, 46, 30], [167, 192, 128]],
        bgstat: [[29, 34, 29], [92, 110, 95]],
    },
    Def {
        name: "cream",
        fg: [
            [60, 50, 40],
            [100, 88, 75],
            [150, 135, 120],
            [185, 172, 158],
        ],
        acc: [80, 140, 80],
        warn: [190, 130, 30],
        bghdr: [[240, 232, 220], [100, 88, 75]],
        bgsel: [[210, 198, 182], [40, 35, 28]],
        bgplay: [[195, 225, 195], [50, 110, 50]],
        bgstat: [[228, 218, 206], [150, 135, 120]],
    },
];

fn fg(c: [u8; 3]) -> String {
    format!("\x1b[38;2;{};{};{}m", c[0], c[1], c[2])
}

fn bg(c: [u8; 3]) -> String {
    format!("\x1b[48;2;{};{};{}m", c[0], c[1], c[2])
}

#[derive(Clone, Default)]
pub struct Colors {
    pub name: String,
    pub w1: String,
    pub w2: String,
    pub w3: String,
    pub grn: String,
    pub amb: String,
    pub bg_hdr: String,
    pub bg_sel: String,
    pub bg_play: String,
    pub bg_stat: String,
}

impl Colors {
    fn of(d: &Def) -> Self {
        Colors {
            name: d.name.to_string(),
            w1: fg(d.fg[1]),
            w2: fg(d.fg[2]),
            w3: fg(d.fg[3]),
            grn: fg(d.acc),
            amb: fg(d.warn),
            bg_hdr: format!("{}{}", bg(d.bghdr[0]), fg(d.bghdr[1])),
            bg_sel: format!("{}{}", bg(d.bgsel[0]), fg(d.bgsel[1])),
            bg_play: format!("{}{}", bg(d.bgplay[0]), fg(d.bgplay[1])),
            bg_stat: format!("{}{}", bg(d.bgstat[0]), fg(d.bgstat[1])),
        }
    }
}

pub struct ThemeMgr {
    pub idx: usize,
    pub c: Colors,
}

impl ThemeMgr {
    pub fn load() -> Self {
        let mut m = ThemeMgr {
            idx: 0,
            c: Colors::of(&THEMES[0]),
        };
        if let Ok(s) = fs::read_to_string(".last_theme") {
            if let Ok(i) = s.trim().parse::<usize>() {
                m.apply(i);
                return m;
            }
        }
        m
    }

    pub fn apply(&mut self, i: usize) {
        self.idx = i % THEMES.len();
        self.c = Colors::of(&THEMES[self.idx]);
        // ponytail: .last_theme en CWD, no en $XDG_STATE_HOME. Upgrade: XDG.
        let _ = fs::write(".last_theme", format!("{}\n", self.idx));
    }
}
