//! A desk clock: a frosted glass card over a slowly turning aurora. The hours and minutes roll
//! like an odometer when they change, the seconds roll beside them and a bar fills across the
//! minute on a spring. Two layers of soft colour blobs turn in opposite directions behind the
//! card, and the card blurs them with `El::backdrop_blur`, so the glass looks the same on every
//! platform. Click the card (or press Space) to switch between 24- and 12-hour time.
//!
//! ```sh
//! cargo run -p aurora-clock --release
//! cargo run -p aurora-clock -- --snap aurora.png --scene evening   # headless, at a fixed time
//! ```
//!
//! `poll` asks for a wake at the next whole second and returns true only when the second
//! changed; the aurora turns on looping rotations, which need no state.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rune::time::LocalTime;
use rune::{AppSpec, Color, Cx, El, Key, KeyEvent, LoopSpec, Spring, TextStyle, Tx, col, el, oklch, row, text};
use rune_kit::{Kit, RollingText};

const SIZE: (f32, f32) = (720.0, 480.0);
const BAR: f32 = 300.0;
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

struct State {
    epoch_ms: i64,
    /// Seconds east of UTC for a fixed clock (scenes); None reads the OS time zone.
    offset: Option<i32>,
    /// A scene holds its time still.
    frozen: bool,
    h12: bool,
}

/// The wall clock, which is what a clock shows; scenes and tests set `epoch_ms` instead.
#[allow(clippy::disallowed_methods)]
fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn init() -> State {
    State { epoch_ms: now_ms(), offset: None, frozen: false, h12: false }
}

impl State {
    fn local(&self) -> LocalTime {
        match self.offset {
            Some(offset) => LocalTime::at_offset(self.epoch_ms, offset),
            None => LocalTime::at(self.epoch_ms),
        }
    }
}

/// 0 is Sunday, by Sakamoto's method for the Gregorian calendar.
fn weekday(t: &LocalTime) -> usize {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if t.month < 3 { t.year - 1 } else { t.year };
    ((y + y / 4 - y / 100 + y / 400 + T[t.month as usize - 1] + t.day as i32).rem_euclid(7)) as usize
}

fn greeting(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=21 => "Good evening",
        _ => "Good night",
    }
}

/// A soft round glow: a radial gradient from `color` to clear.
fn blob(x: f32, y: f32, d: f32, color: Color) -> El {
    el().absolute(x, y).size(d, d).radial_gradient([0.5, 0.5], d * 0.5, [(0.0, color.alpha(0.85)), (0.55, color.alpha(0.35)), (1.0, color.alpha(0.0))])
}

/// A square layer centred on the window that turns once every `period` seconds, carrying blobs.
fn aurora_layer(key: &str, period: f64, reverse: bool, blobs: Vec<El>) -> El {
    let d = 1200.0;
    let layer = el().key(key.to_owned()).absolute((SIZE.0 - d) * 0.5, (SIZE.1 - d) * 0.5).size(d, d).children(blobs);
    // A turn the other way is the same rotation seen in a mirror.
    let layer = if reverse { layer.rotate(std::f32::consts::PI) } else { layer };
    layer.looping(LoopSpec::rotation(period))
}

fn aurora() -> [El; 2] {
    let violet = oklch(0.62, 0.22, 300.0);
    let teal = oklch(0.78, 0.15, 180.0);
    let pink = oklch(0.68, 0.22, 350.0);
    let blue = oklch(0.6, 0.2, 255.0);
    let green = oklch(0.82, 0.18, 145.0);
    let amber = oklch(0.8, 0.15, 70.0);
    [
        aurora_layer("aurora-a", 48.0, false, vec![
            blob(250.0, 300.0, 520.0, violet),
            blob(560.0, 520.0, 460.0, teal),
            blob(380.0, 640.0, 420.0, pink),
        ]),
        aurora_layer("aurora-b", 71.0, true, vec![
            blob(620.0, 300.0, 440.0, blue),
            blob(300.0, 520.0, 380.0, green),
            blob(500.0, 700.0, 360.0, amber),
        ]),
    ]
}

fn card(s: &State, k: &Kit) -> El {
    let t = s.local();
    let white = Color::WHITE;
    let hour = if s.h12 { (t.hour + 11) % 12 + 1 } else { t.hour };
    let sod = f64::from(t.hour * 3600 + t.minute * 60 + t.second);
    let digits = TextStyle::new(104.0, 280.0).lh(112.0).tracking(-3.0).features("'tnum' 1");
    let secs = TextStyle::new(34.0, 380.0).lh(40.0).features("'tnum' 1");
    let caps = TextStyle::new(12.0, 600.0).tracking(1.6);

    let date = format!("{} · {} {}", DAYS[weekday(&t)], MONTHS[t.month as usize - 1], t.day).to_uppercase();
    let hm = RollingText::new("hm", if s.h12 { format!("{hour}:{:02}", t.minute) } else { format!("{hour:02}:{:02}", t.minute) }).value(sod).font(digits).color(white).build(k);
    let ss = RollingText::new("ss", format!("{:02}", t.second)).value(sod).font(secs).color(white.alpha(0.7)).build(k);
    let suffix = s.h12.then(|| text(if t.hour < 12 { "AM" } else { "PM" }).key("ampm").font(caps).color(white.alpha(0.7)));
    let side = col().key("side").gap(4.0).pb(18.0).children(suffix).child(ss);

    let fill = BAR * (t.second + 1) as f32 / 60.0;
    let bar = el().key("bar").w(BAR).h(4.0).radius(2.0).fill(white.alpha(0.16))
        .child(el().key("bar-fill").w(fill).h(4.0).radius(2.0).linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, white.alpha(0.55)), (1.0, white)])
            .motion(Spring::new(0.5, 0.1)));

    col()
        .key("card")
        .role("button")
        .label(if s.h12 { "Clock, 12-hour" } else { "Clock, 24-hour" })
        .hit()
        .on_press(|s: &mut State| s.h12 = !s.h12)
        .w(460.0)
        .padding(36.0)
        .gap(14.0)
        .items_center()
        .radius(36.0)
        .backdrop_blur(42.0)
        .fill(Color::WHITE.alpha(0.08))
        .border(1.0, Color::WHITE.alpha(0.22))
        .shadow(24.0, 60.0, Color { r: 0.0, g: 0.0, b: 0.08, a: 0.45 })
        .motion(Spring::new(0.5, 0.15))
        .child(text(date).key("date").font(caps).color(white.alpha(0.75)))
        .child(row().key("time").gap(10.0).items_end().child(hm).child(side))
        .child(bar)
        .child(text(greeting(t.hour)).key("greeting").font(TextStyle::new(15.0, 450.0)).color(white.alpha(0.8)))
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let k = Kit::neutral(true);
    col()
        .key("root")
        .w_full()
        .h_full()
        .center()
        .fill(oklch(0.16, 0.04, 270.0))
        .children(aurora())
        .child(card(s, &k))
}

fn poll(s: &mut State, _: f64, tx: &mut Tx) -> bool {
    if s.frozen {
        return false;
    }
    let ms = now_ms();
    tx.wake_after(Duration::from_millis((1000 - ms.rem_euclid(1000)) as u64));
    if ms.div_euclid(1000) == s.epoch_ms.div_euclid(1000) {
        return false;
    }
    s.epoch_ms = ms;
    true
}

fn on_key(s: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
    match &event.key {
        Key::Text(t) if t == " " => s.h12 = !s.h12,
        _ => return false,
    }
    true
}

/// 2026-11-07 at `h:m:s` UTC, held still.
fn fix(s: &mut State, h: i64, m: i64, sec: i64, h12: bool) {
    let midnight = 1_794_009_600_000; // 2026-11-07T00:00:00Z
    s.epoch_ms = midnight + ((h * 60 + m) * 60 + sec) * 1000;
    s.offset = Some(0);
    s.frozen = true;
    s.h12 = h12;
}

fn main() {
    rune::run(AppSpec::<State> {
        min_size: Some(SIZE),
        poll: Some(poll),
        on_key: Some(on_key),
        scenes: &[
            ("morning", |s| fix(s, 9, 41, 27, false)),
            ("evening", |s| fix(s, 19, 58, 45, true)),
        ],
        ..AppSpec::new("Aurora Clock", SIZE, init, view)
    });
}

#[cfg(test)]
mod tests;
