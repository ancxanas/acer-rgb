use crate::types::{Color, ColorProfile, Frame, Transition};

pub fn lerp(a: u8, b: u8, t: f64) -> u8 {
    let v = a as f64 + (b as f64 - a as f64) * t;
    v.clamp(0.0, 255.0).round() as u8
}

pub fn build_frames(profile: &ColorProfile) -> Vec<Frame> {
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
                let is_linear = !matches!(from.transition, Some(Transition::None));
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
}
