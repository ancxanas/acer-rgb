use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
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

fn main() {
    println!("kbd-rgbd starting (pid {})", std::process::id());
    loop {
        sleep(Duration::from_secs(60));
    }
}
