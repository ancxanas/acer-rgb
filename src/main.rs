use serde::Deserialize;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

// ---- JSON types ----

#[derive(Debug, Clone, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColorPoint {
    pub color: Color,
    pub transition: Option<String>,
    pub transition_time: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ColorProfile {
    Single(Color),
    Multiple(Vec<ColorPoint>),
    None,
}

#[derive(Debug, Deserialize)]
pub struct ProfileSelector {
    pub leds: Vec<LedProfile>,
}

#[derive(Debug, Deserialize)]
pub struct LedProfile {
    pub device_name: String,
    pub function: String,
    pub profile: String,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub color: Color,
    pub duration_ms: u64,
}

// ---- parsing ----

fn parse_keyboard_json(json: &str) -> Result<ColorProfile, String> {
    serde_json::from_str(json).map_err(|e| format!("json parse: {}", e))
}

fn parse_profile_selector(json: &str) -> Result<ProfileSelector, String> {
    serde_json::from_str(json).map_err(|e| format!("selector parse: {}", e))
}

// ---- animation ----

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    let v = a as f64 + (b as f64 - a as f64) * t;
    v.clamp(0.0, 255.0).round() as u8
}

fn build_frames(profile: &ColorProfile) -> Vec<Frame> {
    match profile {
        ColorProfile::None => vec![Frame {
            color: Color { r: 0, g: 0, b: 0 },
            duration_ms: 1000,
        }],
        ColorProfile::Single(c) => vec![Frame {
            color: Color {
                r: c.r,
                g: c.g,
                b: c.b,
            },
            duration_ms: 1000,
        }],
        ColorProfile::Multiple(points) => {
            let mut frames = Vec::new();
            let step_ms = 80u64;
            for i in 0..points.len() {
                let from = &points[i];
                let to = &points[(i + 1) % points.len()];
                let is_linear = from.transition.as_deref() != Some("None");
                let t_ms = from.transition_time.unwrap_or(1000).max(1);
                if is_linear {
                    let steps = (t_ms / step_ms).max(1);
                    for s in 0..=steps {
                        let t = s as f64 / steps as f64;
                        frames.push(Frame {
                            color: Color {
                                r: lerp(from.color.r, to.color.r, t),
                                g: lerp(from.color.g, to.color.g, t),
                                b: lerp(from.color.b, to.color.b, t),
                            },
                            duration_ms: step_ms,
                        });
                    }
                } else {
                    frames.push(Frame {
                        color: Color {
                            r: to.color.r,
                            g: to.color.g,
                            b: to.color.b,
                        },
                        duration_ms: t_ms,
                    });
                }
            }
            frames
        }
    }
}

// ---- daemon runtime ----

const ACTIVE_PROFILE_PATH: &str = "/etc/tailord/active_profile.json";
const KEYBOARD_DIR: &str = "/etc/tailord/keyboard";
const LED_PATH: &str = "/sys/class/leds/rgb:kbd_backlight/multi_intensity";
const CMD_PATH: &str = "/run/kbd-rgbd/cmd";

fn load_frames(selector_path: &Path) -> Result<Vec<Frame>, String> {
    let sel_text = fs::read_to_string(selector_path)
        .map_err(|e| format!("read '{}': {}", selector_path.display(), e))?;
    let selector = parse_profile_selector(&sel_text)?;
    let profile_name = selector
        .leds
        .first()
        .ok_or("no LED entries in profile")?
        .profile
        .as_str();
    let kb_path = Path::new(KEYBOARD_DIR).join(format!("{}.json", profile_name));
    let kb_text =
        fs::read_to_string(&kb_path).map_err(|e| format!("read '{}': {}", kb_path.display(), e))?;
    let color_profile = parse_keyboard_json(&kb_text)?;
    let frames = build_frames(&color_profile);
    if frames.is_empty() {
        return Err("no frames generated".into());
    }
    Ok(frames)
}

fn resolve_active_path() -> Option<String> {
    let link = fs::read_link(ACTIVE_PROFILE_PATH).ok()?;
    let path = link.to_string_lossy().to_string();
    if path.starts_with('/') {
        Some(path)
    } else {
        let dir = Path::new(ACTIVE_PROFILE_PATH).parent().unwrap();
        Some(dir.join(path).to_string_lossy().to_string())
    }
}

fn write_sysfs(color: &Color) -> io::Result<()> {
    let mut f = fs::OpenOptions::new().write(true).open(LED_PATH)?;
    write!(f, "{} {} {}\n", color.r, color.g, color.b)
}

fn read_command() -> Option<String> {
    let mut f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(CMD_PATH)
        .ok()?;
    let mut content = String::new();
    f.read_to_string(&mut content).ok()?;
    let content = content.trim().to_string();
    if content.is_empty() {
        return None;
    }
    let _ = f.set_len(0);
    Some(content)
}

fn handle_profile_cmd(name: &str, frames: &mut Vec<Frame>, frame_idx: &mut usize) {
    let profile_path = format!("/etc/tailord/profiles/{}.json", name);
    match load_frames(Path::new(&profile_path)) {
        Ok(f) => {
            // Only update symlink after confirming the profile loads
            let tmp = "/etc/tailord/active_profile.json.tmp";
            let active = Path::new(ACTIVE_PROFILE_PATH);
            let _ = fs::remove_file(tmp);
            if std::os::unix::fs::symlink(&profile_path, tmp).is_ok() {
                let _ = fs::rename(tmp, active);
            }
            eprintln!("switched to '{}' ({} frames)", name, f.len());
            *frames = f;
            *frame_idx = 0;
        }
        Err(e) => eprintln!("switch to '{}': {}", name, e),
    }
}

fn retry_sleep(secs: u64, running: &mut bool) {
    for _ in 0..secs {
        if let Some(cmd) = read_command() {
            if cmd == "stop" {
                *running = false;
                return;
            }
        }
        sleep(Duration::from_secs(1));
    }
}

fn main() {
    eprintln!("kbd-rgbd starting (pid {})", std::process::id());

    let _ = fs::create_dir_all("/run/kbd-rgbd");

    // Ensure cmd file exists with world-writable permissions for unprivileged scripts
    if let Ok(f) = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(CMD_PATH)
    {
        drop(f);
        let _ = fs::set_permissions(CMD_PATH, fs::Permissions::from_mode(0o666));
    }

    let (mut frames, mut frame_idx) = loop {
        match resolve_active_path().and_then(|p| match load_frames(Path::new(&p)) {
            Ok(f) => {
                eprintln!("loaded {} frames", f.len());
                Some(f)
            }
            Err(e) => {
                eprintln!("profile load: {}", e);
                None
            }
        }) {
            Some(f) => break (f, 0usize),
            None => {
                eprintln!("waiting for active profile...");
                let mut running = true;
                retry_sleep(5, &mut running);
                if !running {
                    return;
                }
            }
        }
    };

    loop {
        if let Some(cmd) = read_command() {
            match cmd.as_str() {
                "stop" => {
                    let _ = write_sysfs(&Color { r: 0, g: 0, b: 0 });
                    break;
                }
                "reload" => {
                    if let Some(p) = resolve_active_path() {
                        if let Ok(f) = load_frames(Path::new(&p)) {
                            eprintln!("reloaded {} frames", f.len());
                            frames = f;
                            frame_idx = 0;
                        }
                    }
                }
                _ if cmd.starts_with("profile ") => {
                    let name = cmd.trim_start_matches("profile ").trim().to_string();
                    handle_profile_cmd(&name, &mut frames, &mut frame_idx);
                }
                _ => {}
            }
        }

        let frame = &frames[frame_idx];
        if let Err(e) = write_sysfs(&frame.color) {
            eprintln!("write error: {}", e);
            let mut recovered = false;
            for _ in 0..5 {
                if let Some(cmd) = read_command() {
                    if cmd == "stop" {
                        return;
                    }
                    if cmd.starts_with("profile ") {
                        let name = cmd.trim_start_matches("profile ").trim().to_string();
                        handle_profile_cmd(&name, &mut frames, &mut frame_idx);
                        recovered = true;
                        break;
                    }
                }
                if write_sysfs(&frame.color).is_ok() {
                    recovered = true;
                    break;
                }
                sleep(Duration::from_secs(1));
            }
            if !recovered {
                eprintln!("retry failed, reloading profile");
                if let Some(p) = resolve_active_path() {
                    if let Ok(f) = load_frames(Path::new(&p)) {
                        frames = f;
                        frame_idx = 0;
                    }
                }
            }
            continue;
        }

        frame_idx = (frame_idx + 1) % frames.len();
        sleep(Duration::from_millis(frame.duration_ms));
    }

    eprintln!("kbd-rgbd stopped");
}

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp_bounds() {
        assert_eq!(lerp(0, 255, 0.0), 0);
        assert_eq!(lerp(0, 255, 1.0), 255);
    }

    #[test]
    fn test_lerp_mid() {
        assert_eq!(lerp(0, 255, 0.5), 128);
        assert_eq!(lerp(100, 200, 0.5), 150);
    }

    #[test]
    fn test_single_profile() {
        let json = r#"{"Single":{"r":255,"g":100,"b":0}}"#;
        let profile = parse_keyboard_json(json).unwrap();
        match &profile {
            ColorProfile::Single(c) => {
                assert_eq!(c.r, 255);
                assert_eq!(c.g, 100);
                assert_eq!(c.b, 0);
            }
            _ => panic!("expected Single"),
        }
        let frames = build_frames(&profile);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].color.r, 255);
        assert_eq!(frames[0].duration_ms, 1000);
    }

    #[test]
    fn test_none_profile() {
        let json = r#""None""#;
        let profile = parse_keyboard_json(json).unwrap();
        let frames = build_frames(&profile);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].color.r, 0);
        assert_eq!(frames[0].color.g, 0);
        assert_eq!(frames[0].color.b, 0);
    }

    #[test]
    fn test_multiple_profile_has_frames() {
        let json = r#"{"Multiple":[
            {"color":{"r":255,"g":0,"b":0},"transition":"Linear","transition_time":4000},
            {"color":{"r":0,"g":255,"b":0},"transition":"Linear","transition_time":4000}
        ]}"#;
        let profile = parse_keyboard_json(json).unwrap();
        let frames = build_frames(&profile);
        assert!(frames.len() >= 100);
    }

    #[test]
    fn test_selector_parse() {
        let json = r#"{"leds":[{"device_name":"platform:tuxedo_keyboard","function":"kbd_backlight","profile":"rainbow","mode":"Rgb"}]}"#;
        let sel = parse_profile_selector(json).unwrap();
        assert_eq!(sel.leds.len(), 1);
        assert_eq!(sel.leds[0].profile, "rainbow");
    }

    #[test]
    fn test_multiple_none_transition() {
        let json = r#"{"Multiple":[
            {"color":{"r":255,"g":0,"b":0},"transition":"None","transition_time":500},
            {"color":{"r":0,"g":255,"b":0},"transition":"None","transition_time":500}
        ]}"#;
        let profile = parse_keyboard_json(json).unwrap();
        let frames = build_frames(&profile);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].duration_ms, 500);
        assert_eq!(frames[1].duration_ms, 500);
    }
}
