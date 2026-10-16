# rmustp

Reproductor de música para terminal, en Rust. Solo POSIX (Linux/macOS).
Reescritura de `cmuspp`: un binario, 4 dependencias, cero código de audio propio.

## Compilar / ejecutar

```bash
cargo build --release
./target/release/rmustp
```

En Linux hacen falta cabeceras ALSA en build (`alsa-lib-devel` o similar;
en runtime las usa rodio/cpal por dentro). Sin `themes/` al lado:
los 9 temas van compilados. Sin red, sin config.

## Comportamiento

- `↑/↓` navegar, `Enter` reproducir/entrar, `o` explorador, `q` salir.
- `Space` pausa, `←/→` ∓5s, `n`/`p` siguiente/anterior, `s` shuffle, `r` repeat.
- `/` buscar (ASCII), `l` letras (`.lrc` al lado, si no tags incrustados),
  `t` tema (9 built-in), `a` info, `-h`/`-v` usage/version.
- Portadas: incrustadas (mp3/flac/ogg) + `cover.jpg/png` etc. Solo ANSI
  half-block. Sin protocolo Kitty a propósito.
- Tags: escáner propio mínimo ID3v2/FLAC/Vorbis. Lo que rodio no decodifica
  se salta con una línea en stderr.
- Escribe `.last_theme` en el directorio de trabajo (heredado de cmuspp).
- Windows: ni compila (`compile_error!`). Sin TTY (stdin/stdout): exit 2 limpio.

## Dependencias (las 4, una línea cada una)

- `rodio` — suena y decodifica (reemplaza libsndfile + ALSA/CoreAudio a mano).
- `jpeg-decoder` — portadas jpg sin libjpeg (solo RGB/gris).
- `png` — portadas png sin libpng (solo decode RGB/RGBA/gris-8).
- `libc` — raw/ioctl/select POSIX sin crossterm. Todo el `unsafe` vive
  en `src/term.rs`, con `// SAFETY:` por bloque.
- Transitivas (45, sin features de más): decode vía `symphonia` (mp3),
  `claxon` (flac), `lewton` (vorbis); audio vía `cpal` + `alsa-sys`;
  resto utilidades (`bitflags`, `log`, etc.). `rodio` las trae con
  `default-features=false` + solo nuestros 4 codecs.

## Verificación y diseño

- Por qué rodio y no libsndfile vía FFI: rodio ya hace decode + resample interno
  al device; reintroducir FFI sería el mismo trabajo en otro lenguaje.
- Verificación: `cargo build --release && cargo clippy -- -D warnings && cargo fmt --check`;
  smoke manual del binario. No hay tests (decisión del proyecto).
- Requisitos: ~18 MB RAM típico (~30 MB con `LD_PRELOAD` de asignador
  alternativo como mimalloc; `// ponytail: arenas rodio/cpal/ALSA. Upgrade:
  decoder PCM-only propio sin rodio`). Objetivo: <25 MB en típico.
- Limitaciones conocidas: CJK de ancho doble desalinea columnas
  (sin `unicode-width` a propósito, ver `src/term.rs`).
