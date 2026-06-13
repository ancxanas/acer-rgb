pub mod animation;
pub mod error;
pub mod runtime;
pub mod types;

pub use animation::{build_frames, lerp};
pub use error::{KbdError, Result};
pub use runtime::run;
pub use types::{
    parse_keyboard_json, parse_profile_selector, Color, ColorPoint, ColorProfile, Frame,
    LedProfile, ProfileSelector, Transition,
};
