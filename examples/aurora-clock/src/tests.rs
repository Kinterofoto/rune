use super::*;
use rune::{Env, Runtime};

fn runtime(h: i64, m: i64, sec: i64) -> Runtime<State> {
    let mut s = init();
    fix(&mut s, h, m, sec, false);
    let mut rt = Runtime::new(s, view, SIZE, Env { system_dark: true, scale: 2.0 });
    rt.frame(0.0);
    rt
}

#[test]
fn the_fixed_scene_reads_saturday_the_seventh() {
    let rt = runtime(9, 41, 27);
    let t = rt.state.local();
    assert_eq!((t.year, t.month, t.day, t.hour, t.minute, t.second), (2026, 11, 7, 9, 41, 27));
    assert_eq!(DAYS[weekday(&t)], "Saturday");
}

#[test]
fn a_tap_on_the_card_switches_to_twelve_hour_time() {
    let mut rt = runtime(19, 58, 45);
    assert!(rt.find("ampm").is_none());
    let (x, y) = rt.find("card").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..30 { rt.frame(1.0 / 60.0); }
    assert!(rt.state.h12);
    assert!(rt.find("ampm").is_some());
}

#[test]
fn the_second_bar_grows_on_a_spring() {
    let mut rt = runtime(9, 41, 10);
    for _ in 0..90 { rt.frame(1.0 / 60.0); }
    let w = |rt: &Runtime<State>| rt.presented("bar-fill").unwrap().0[2];
    let before = w(&rt);
    rt.update(|s, _| s.epoch_ms += 20_000);
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    let mid = w(&rt);
    for _ in 0..90 { rt.frame(1.0 / 60.0); }
    let after = w(&rt);
    assert!(before < mid && mid < after, "{before} → {mid} → {after}");
    assert!((after - BAR * 31.0 / 60.0).abs() < 0.5);
}
