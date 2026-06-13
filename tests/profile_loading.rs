use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir() -> PathBuf {
    let base = std::env::temp_dir().join("kbd_rgbd_test");
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = base.join(format!("{}", id));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_json(dir: &Path, name: &str, json: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, json).unwrap();
    path
}

/// End-to-end test: write profile+keyboard JSONs to temp dirs,
/// parse via public API, and verify frame generation.
#[test]
fn test_end_to_end_profile_loading() {
    let root = temp_dir();
    let profiles = root.join("profiles");
    let keyboard = root.join("keyboard");
    fs::create_dir_all(&profiles).unwrap();
    fs::create_dir_all(&keyboard).unwrap();

    // Write a profile selector
    write_json(
        &profiles,
        "cycle.json",
        r#"{"leds":[{"device_name":"platform:tuxedo_keyboard","function":"kbd_backlight","profile":"rainbow","mode":"Rgb"}]}"#,
    );

    // Write a keyboard animation with 2 color points
    write_json(
        &keyboard,
        "rainbow.json",
        r#"{"Multiple":[
            {"color":{"r":255,"g":0,"b":0},"transition":"Linear","transition_time":4000},
            {"color":{"r":0,"g":255,"b":0},"transition":"Linear","transition_time":4000}
        ]}"#,
    );

    // Read and parse profile selector
    let sel_text = fs::read_to_string(profiles.join("cycle.json")).unwrap();
    let selector = kbd_rgbd::parse_profile_selector(&sel_text).unwrap();
    let profile_name = &selector.leds.first().unwrap().profile;

    // Read and parse keyboard animation
    let kb_path = keyboard.join(format!("{}.json", profile_name));
    let kb_text = fs::read_to_string(&kb_path).unwrap();
    let color_profile = kbd_rgbd::parse_keyboard_json(&kb_text).unwrap();

    // Build frames
    let frames = kbd_rgbd::build_frames(&color_profile);
    assert!(!frames.is_empty(), "should generate frames");
    assert!(frames.len() >= 100, "rainbow should produce many frames");

    // Verify each frame has a valid duration
    for frame in &frames {
        assert!(frame.duration_ms > 0);
    }

    // First frame should be red (255,0,0)
    assert_eq!(frames[0].color.r, 255);
    assert_eq!(frames[0].color.g, 0);
    // Somewhere later we should reach green (0,255,0)
    let has_green = frames.iter().any(|f| f.color.r == 0 && f.color.g == 255);
    assert!(has_green, "should reach green at some point");

    // Test Single profile
    write_json(
        &keyboard,
        "static-red.json",
        r#"{"Single":{"r":255,"g":0,"b":0}}"#,
    );
    let single_text = fs::read_to_string(keyboard.join("static-red.json")).unwrap();
    let single_profile = kbd_rgbd::parse_keyboard_json(&single_text).unwrap();
    let single_frames = kbd_rgbd::build_frames(&single_profile);
    assert_eq!(single_frames.len(), 1);
    assert_eq!(single_frames[0].color.r, 255);

    // Test None profile
    write_json(&keyboard, "off.json", r#""None""#);
    let none_text = fs::read_to_string(keyboard.join("off.json")).unwrap();
    let none_profile = kbd_rgbd::parse_keyboard_json(&none_text).unwrap();
    let none_frames = kbd_rgbd::build_frames(&none_profile);
    assert_eq!(none_frames.len(), 1);
    assert_eq!(none_frames[0].color.r, 0);
}

/// Test that short transitions (≤80ms) still produce interpolation
#[test]
fn test_short_transition_steps() {
    // transition_time=80ms, step_ms=80ms → steps=1
    // With 0..=steps, this must produce 2 frames (start + end) per segment
    let profile = kbd_rgbd::parse_keyboard_json(
        r#"{"Multiple":[
            {"color":{"r":255,"g":0,"b":0},"transition":"Linear","transition_time":80},
            {"color":{"r":0,"g":255,"b":0},"transition":"Linear","transition_time":80}
        ]}"#,
    )
    .unwrap();
    let frames = kbd_rgbd::build_frames(&profile);
    // 2 segments × (steps+1 = 2) = 4 frames
    assert_eq!(frames.len(), 4);
    // Frame 0: red (t=0.0)
    assert_eq!(frames[0].color.r, 255);
    assert_eq!(frames[0].color.g, 0);
    // Frame 1: green (t=1.0) — this is the frame that would be
    // missing with 0..steps, proving the bug is fixed
    assert_eq!(frames[1].color.r, 0);
    assert_eq!(frames[1].color.g, 255);
}
