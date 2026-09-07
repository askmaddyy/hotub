use iocraft::prelude::*;
use std::time::Duration;

// ---------- tiny deterministic noise ----------

fn hash01(x: i32, y: i32, t: i32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add((t as u32).wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

// ---------- fire ----------

const FIRE_CHARS: &[char] = &[' ', '.', ':', ';', '!', '*', '+', '#', '%', '@', '#'];

fn fire_color(h: f32) -> Color {
    // black -> maroon -> red -> orange -> yellow -> white
    let (r, g, b) = if h < 0.2 {
        let k = h / 0.2;
        (lerp(10.0, 180.0, k), lerp(0.0, 20.0, k), lerp(0.0, 0.0, k))
    } else if h < 0.45 {
        let k = (h - 0.2) / 0.25;
        (lerp(180.0, 255.0, k), lerp(20.0, 60.0, k), 0.0)
    } else if h < 0.7 {
        let k = (h - 0.45) / 0.25;
        (255.0, lerp(60.0, 150.0, k), lerp(0.0, 0.0, k))
    } else if h < 0.88 {
        let k = (h - 0.7) / 0.18;
        (255.0, lerp(150.0, 225.0, k), lerp(0.0, 80.0, k))
    } else {
        let k = (h - 0.88) / 0.12;
        (255.0, lerp(225.0, 255.0, k), lerp(80.0, 230.0, k))
    };
    Color::Rgb {
        r: r as u8,
        g: g as u8,
        b: b as u8,
    }
}

fn lerp(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
}

fn fire_heat(x: i32, y: i32, w: i32, h: i32, tick: i32) -> f32 {
    if w <= 0 || h <= 0 {
        return 0.0;
    }
    let xf = x as f32;
    let yf = y as f32;
    let wf = w as f32;
    let hf = h as f32;
    // depth: 1 at bottom, 0 at top
    let d = 1.0 - (yf / hf);
    // center boost so flames lick higher in the middle
    let cx = ((xf - wf / 2.0) / (wf / 2.0).max(1.0)).abs();
    let shape = (1.0 - cx * cx * 0.55).max(0.25);
    // flickering fuel at the bottom
    let fuel = 0.72
        + 0.28 * (xf * 0.32 + tick as f32 * 0.22).sin()
        + 0.12 * (xf * 0.11 - tick as f32 * 0.11).sin();
    // rising turbulence
    let sway = (yf * 0.28 - tick as f32 * 0.24 + (xf * 0.22 + tick as f32 * 0.06).sin() * 2.2).sin();
    let grain = hash01(x, y / 2, tick / 2) - 0.5;
    let mut heat = (d.powf(1.35) * fuel + sway * 0.10 * d + grain * 0.22 * d) * shape;
    // embers / sparks shooting up
    let spark = hash01(x * 3 + 17, 0, tick / 2);
    if spark > 0.93 {
        let rise = ((tick * 2 + (spark * 100.0) as i32) % (h * 2)).max(0);
        let sy = h - 1 - rise % h.max(1);
        if (y - sy).abs() <= 1 && (hash01(x, tick, 7) > 0.4) {
            heat += 0.55;
        }
    }
    clamp01(heat * 1.25)
}

fn fire_row_contents(y: i32, w: i32, h: i32, tick: i32) -> Vec<MixedTextContent> {
    let mut out: Vec<MixedTextContent> = Vec::new();
    let mut buf = String::new();
    let mut cur: Option<Color> = None;
    let flush = |buf: &mut String, cur: &mut Option<Color>, out: &mut Vec<MixedTextContent>| {
        if !buf.is_empty() {
            let c = cur.unwrap_or(Color::Reset);
            out.push(MixedTextContent::new(std::mem::take(buf)).color(c));
        }
    };
    for x in 0..w {
        let heat = fire_heat(x, y, w, h, tick);
        let idx = (heat * (FIRE_CHARS.len() - 1) as f32).round() as usize;
        let ch = FIRE_CHARS[idx.min(FIRE_CHARS.len() - 1)];
        let col = if heat < 0.03 {
            Color::Reset
        } else {
            fire_color(heat)
        };
        if cur != Some(col) {
            flush(&mut buf, &mut cur, &mut out);
            cur = Some(col);
        }
        buf.push(ch);
    }
    flush(&mut buf, &mut cur, &mut out);
    if out.is_empty() {
        out.push(MixedTextContent::new(" ".repeat(w.max(0) as usize)));
    }
    out
}

// ---------- water ----------

fn water_color(depth: f32, shimmer: f32) -> Color {
    // shallow cyan -> blue -> deep navy, sparkled by shimmer
    let sparkle = (shimmer * 40.0) as i16;
    let (r, g, b) = if depth < 0.25 {
        (64 + sparkle, 224, 255)
    } else if depth < 0.55 {
        (20, 130 + sparkle / 2, 255)
    } else if depth < 0.8 {
        (10, 70 + sparkle / 3, 190)
    } else {
        (8, 30 + sparkle / 4, 120)
    };
    Color::Rgb {
        r: r.clamp(0, 255) as u8,
        g: g.clamp(0, 255) as u8,
        b: b.clamp(0, 255) as u8,
    }
}

fn surface_y(x: i32, w: i32, h: i32, t: f32) -> i32 {
    let base = h as f32 * 0.32;
    let s = (x as f32 * 0.24 + t * 0.11).sin() * 1.3
        + (x as f32 * 0.11 - t * 0.065).sin() * 1.9
        + ((x as f32 + t * 1.4) * 0.045).sin() * 1.1
        + (w as f32 * 0.01).sin();
    (base + s).round() as i32
}

fn water_row_contents(y: i32, w: i32, h: i32, tick: i32, speed: f32) -> Vec<MixedTextContent> {
    let t = tick as f32 * speed;
    let mut out: Vec<MixedTextContent> = Vec::new();
    let mut buf = String::new();
    let mut cur: Option<Color> = None;
    let push_char = |ch: char, col: Color, buf: &mut String, cur: &mut Option<Color>, out: &mut Vec<MixedTextContent>| {
        if *cur != Some(col) {
            if !buf.is_empty() {
                let c = cur.unwrap_or(Color::Reset);
                out.push(MixedTextContent::new(std::mem::take(buf)).color(c));
            }
            *cur = Some(col);
        }
        buf.push(ch);
    };
    // fish!
    let fish_x = ((t * 1.6) as i32 % (w + 20)) - 10;
    let fish_y = (h as f32 * 0.62 + (t * 0.09).sin() * 2.0).round() as i32;
    let fish_dir_right = (tick / 200) % 2 == 0;
    for x in 0..w {
        let sy = surface_y(x, w, h, t).clamp(0, h - 1);
        if y < sy {
            // air: occasional spray drops near the crest
            let near = sy - y;
            if near <= 2 && hash01(x * 2, y * 3, (tick / 3) as i32) > 0.86 {
                let c = if near == 1 {
                    Color::Rgb { r: 180, g: 240, b: 255 }
                } else {
                    Color::Rgb { r: 110, g: 190, b: 240 }
                };
                let ch = if hash01(x, y, tick) > 0.5 { '\'' } else { '.' };
                push_char(ch, c, &mut buf, &mut cur, &mut out);
            } else {
                push_char(' ', Color::Reset, &mut buf, &mut cur, &mut out);
            }
        } else if y == sy {
            // foam crest
            let foam = hash01(x, (tick / 4) as i32, 99);
            let ch = if foam > 0.8 {
                '*'
            } else if foam > 0.55 {
                '='
            } else if foam > 0.3 {
                '~'
            } else {
                '-'
            };
            push_char(
                ch,
                Color::Rgb { r: 235, g: 250, b: 255 },
                &mut buf,
                &mut cur,
                &mut out,
            );
        } else {
            // underwater
            if x >= fish_x - 2 && x <= fish_x + 2 && y == fish_y {
                let offset = x - fish_x;
                let (ch, col) = match (offset, fish_dir_right) {
                    (-2, true) => ('<', Color::Yellow),
                    (-1, true) => ('>', Color::Yellow),
                    (0, true) => ('<', Color::Yellow),
                    (1, true) => ('>', Color::Yellow),
                    (2, true) => ('<', Color::Yellow),
                    (-2, false) => ('>', Color::Yellow),
                    (-1, false) => ('<', Color::Yellow),
                    (0, false) => ('>', Color::Yellow),
                    (1, false) => ('<', Color::Yellow),
                    _ => ('>', Color::Yellow),
                };
                let _ = col;
                push_char(ch, Color::Yellow, &mut buf, &mut cur, &mut out);
                continue;
            }
            let depth = ((y - sy) as f32 / (h - sy).max(1) as f32).clamp(0.0, 1.0);
            // bubbles rising
            if hash01(x * 5, y * 2, (tick / 5) as i32) > 0.986 {
                push_char(
                    'o',
                    Color::Rgb { r: 200, g: 240, b: 255 },
                    &mut buf,
                    &mut cur,
                    &mut out,
                );
                continue;
            }
            let shimmer = (x as f32 * 0.55 + t * 0.35).sin() * (y as f32 * 0.42 - t * 0.22).sin();
            let col = water_color(depth, shimmer);
            let ch = if depth < 0.18 {
                if shimmer > 0.5 {
                    '='
                } else {
                    '~'
                }
            } else if shimmer > 0.72 {
                '.'
            } else if shimmer > 0.35 {
                '~'
            } else if shimmer > -0.2 {
                '-'
            } else if depth > 0.7 {
                ' '
            } else {
                '~'
            };
            // deep water: use background-ish dim char
            if depth > 0.7 && ch == ' ' {
                push_char(' ', Color::Reset, &mut buf, &mut cur, &mut out);
            } else {
                push_char(ch, col, &mut buf, &mut cur, &mut out);
            }
        }
    }
    if !buf.is_empty() {
        let c = cur.unwrap_or(Color::Reset);
        out.push(MixedTextContent::new(std::mem::take(&mut buf)).color(c));
    }
    if out.is_empty() {
        out.push(MixedTextContent::new(" ".repeat(w.max(0) as usize)));
    }
    out
}

// ---------- props ----------

#[derive(Default, Props)]
struct FireProps {
    tick: i32,
    width: usize,
    height: usize,
}

#[component]
fn FireView(props: &FireProps) -> impl Into<AnyElement<'static>> {
    let w = props.width.max(8) as i32;
    let h = props.height.max(4) as i32;
    let tick = props.tick;
    // log pile at the bottom
    let log_y1 = h - 2;
    let log_y2 = h - 1;
    element! {
        View(flex_direction: FlexDirection::Column) {
            #( (0..h).map(|y| {
                if y == log_y1 || y == log_y2 {
                    let logs = if y == log_y1 {
                        let pad = (w / 2 - 9).max(0) as usize;
                        format!("{}__||___||__", " ".repeat(pad))
                    } else {
                        let pad = (w / 2 - 11).max(0) as usize;
                        format!("{}|..||||..|||.|", " ".repeat(pad))
                    };
                    element! {
                        MixedText(contents: vec![
                            MixedTextContent::new(logs).color(Color::DarkYellow),
                        ])
                    }.into_any()
                } else {
                    element! {
                        MixedText(contents: fire_row_contents(y, w, h, tick))
                    }.into_any()
                }
            }))
        }
    }
}

#[derive(Default, Props)]
struct WaterProps {
    tick: i32,
    speed: f32,
    width: usize,
    height: usize,
}

#[component]
fn WaterView(props: &WaterProps) -> impl Into<AnyElement<'static>> {
    let w = props.width.max(8) as i32;
    let h = props.height.max(4) as i32;
    let tick = props.tick;
    let speed = props.speed;
    element! {
        View(flex_direction: FlexDirection::Column) {
            #( (0..h).map(|y| element! {
                MixedText(contents: water_row_contents(y, w, h, tick, speed))
            }.into_any()))
        }
    }
}

// ---------- app ----------

#[component]
fn App(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let (tw, th) = hooks.use_terminal_size();
    let mut system = hooks.use_context_mut::<SystemContext>();
    let mut tick = hooks.use_state(|| 0i32);
    let mut mode = hooks.use_state(|| 0u8); // 0 fire, 1 water, 2 split
    let mut paused = hooks.use_state(|| false);
    let mut speed = hooks.use_state(|| 1.0f32);
    let mut should_exit = hooks.use_state(|| false);

    hooks.use_future(async move {
        loop {
            smol::Timer::after(Duration::from_millis(50)).await;
            if !paused.get() {
                tick += 1;
            }
        }
    });

    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Char('q') | KeyCode::Esc => should_exit.set(true),
                KeyCode::Char('1') => mode.set(0),
                KeyCode::Char('2') => mode.set(1),
                KeyCode::Char('3') => mode.set(2),
                KeyCode::Char(' ') => paused.set(!paused.get()),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    speed.set((speed.get() + 0.25).min(4.0))
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    speed.set((speed.get() - 0.25).max(0.25))
                }
                _ => {}
            }
        }
        _ => {}
    });

    if should_exit.get() {
        system.exit();
    }

    let tw = tw as usize;
    let th = th as usize;
    let body_h = th.saturating_sub(6).max(6);
    let mode_v = mode.get();
    let tick_v = tick.get();
    let speed_v = speed.get();
    let paused_v = paused.get();

    let (title, title_color) = match mode_v {
        0 => (
            format!("EMBERFLOW - FIRE {}  tick {}", if paused_v { "[paused]" } else { "" }, tick_v),
            Color::Rgb { r: 255, g: 140, b: 0 },
        ),
        1 => (
            format!("EMBERFLOW - WATER {}  tick {}", if paused_v { "[paused]" } else { "" }, tick_v),
            Color::Rgb { r: 64, g: 200, b: 255 },
        ),
        _ => (
            format!("EMBERFLOW - FIRE + WATER {}  tick {}", if paused_v { "[paused]" } else { "" }, tick_v),
            Color::Magenta,
        ),
    };

    let doctest_hint = "1 fire  2 water  3 split  space pause  +/- speed  q quit";

    element! {
        View(
            width: 100pct,
            height: 100pct,
            flex_direction: FlexDirection::Column,
            background_color: Color::Black,
        ) {
            View(
                height: 3,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_style: BorderStyle::Single,
                border_edges: Edges::Bottom,
                border_color: Color::DarkGrey,
            ) {
                Text(content: title, color: title_color, weight: Weight::Bold)
            }
            View(flex_grow: 1.0f32, flex_direction: FlexDirection::Row) {
                #(match mode_v {
                    0 => vec![element! {
                        View(flex_grow: 1.0f32, padding_left: 1, padding_right: 1) {
                            FireView(tick: tick_v, width: tw.saturating_sub(4), height: body_h)
                        }
                    }.into_any()],
                    1 => vec![element! {
                        View(flex_grow: 1.0f32, padding_left: 1, padding_right: 1) {
                            WaterView(tick: tick_v, speed: speed_v, width: tw.saturating_sub(4), height: body_h)
                        }
                    }.into_any()],
                    _ => {
                        let half = tw.saturating_sub(7) / 2;
                        vec![
                            element! {
                                View(width: half as i32, flex_shrink: 0.0f32) {
                                    FireView(tick: tick_v, width: half, height: body_h)
                                }
                            }.into_any(),
                            element! {
                                View(width: 3, flex_shrink: 0.0f32, align_items: AlignItems::Center) {
                                    Text(content: "|", color: Color::DarkGrey)
                                    Text(content: "|", color: Color::DarkGrey)
                                }
                            }.into_any(),
                            element! {
                                View(flex_grow: 1.0f32) {
                                    WaterView(tick: tick_v, speed: speed_v, width: half, height: body_h)
                                }
                            }.into_any(),
                        ]
                    }
                })
            }
            View(
                height: 3,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                border_style: BorderStyle::Single,
                border_edges: Edges::Top,
                border_color: Color::DarkGrey,
            ) {
                Text(content: doctest_hint, color: Color::Grey)
                Text(content: format!("speed {:.2}x{}", speed_v, if paused_v { "  PAUSED - hit space" } else { "" }), color: Color::DarkGrey)
            }
        }
    }
}

fn main() {
    // `cargo run -- smoke` prints one static frame of each (for CI / screenshots).
    // default: fullscreen animated. Switch live with 1/2/3.
    let arg = std::env::args().nth(1).unwrap_or_default();
    if arg == "smoke" || arg == "once" {
        element! {
            View(flex_direction: FlexDirection::Column) {
                Text(content: "--- FIRE (tick 40) ---", color: Color::Yellow, weight: Weight::Bold)
                FireView(tick: 40i32, width: 60usize, height: 18usize)
                Text(content: "--- WATER (tick 40) ---", color: Color::Cyan, weight: Weight::Bold)
                WaterView(tick: 40i32, speed: 1.0f32, width: 60usize, height: 14usize)
            }
        }
        .print();
        return;
    }
    smol::block_on(element!(App).fullscreen()).unwrap();
}
