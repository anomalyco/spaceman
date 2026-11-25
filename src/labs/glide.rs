//! The Scrolling Strip's glide tuning in `[labs]`: how long a glide feels (or that there is
//! none), how springy it is, and how columns meet an edge shared with another monitor. Each key
//! is left out of `config.toml` at its default, and the worker reloads them before its next
//! command. Settings offers the glide as one pick, [`GlideMotion`].

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::motion::SpringParams;

/// The tuning's keys in `[labs]`.
pub const DURATION_KEY: &str = "scrolling_strip_duration_ms";
pub const DAMPING_KEY: &str = "scrolling_strip_damping";
pub const SHARED_EDGE_KEY: &str = "scrolling_strip_shared_edge";

/// `scrolling_strip_duration_ms`: how long a glide feels, in milliseconds. `0` is no glide at
/// all: columns snap to where they rest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlideDuration(u16);

impl GlideDuration {
    /// No glide: the strip snaps.
    pub const OFF: Self = Self(0);
    pub const MIN: u16 = 80;
    pub const MAX: u16 = 900;

    /// `ms` when it is 0 (no glide) or in range.
    pub fn new(ms: u16) -> Result<Self, String> {
        if ms == 0 || (Self::MIN..=Self::MAX).contains(&ms) {
            Ok(Self(ms))
        } else {
            Err(Self::expected(ms))
        }
    }

    fn expected(got: impl std::fmt::Display) -> String {
        format!(
            "expected 0 (no glide) or {}..={} milliseconds, got {got}",
            Self::MIN,
            Self::MAX
        )
    }

    #[must_use]
    pub const fn ms(self) -> u16 {
        self.0
    }

    /// Whether the strip snaps instead of gliding.
    #[must_use]
    pub const fn is_off(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for GlideDuration {
    fn default() -> Self {
        Self(SpringParams::DEFAULT_DURATION_MS)
    }
}

impl Serialize for GlideDuration {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for GlideDuration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ms = i64::deserialize(deserializer)?;
        u16::try_from(ms)
            .map_err(|_| Self::expected(ms))
            .and_then(Self::new)
            .map_err(serde::de::Error::custom)
    }
}

/// `scrolling_strip_damping`: the spring's damping ratio, kept to hundredths. 1.0 arrives
/// without overshoot; lower bounces a little, higher eases in more softly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlideDamping(u16);

impl GlideDamping {
    pub const MIN: f64 = 0.6;
    pub const MAX: f64 = 1.3;

    /// `ratio`, rounded to hundredths, when it is in range.
    pub fn new(ratio: f64) -> Result<Self, String> {
        let hundredths = (ratio * 100.0).round();
        if ratio.is_finite() && (Self::MIN * 100.0..=Self::MAX * 100.0).contains(&hundredths) {
            Ok(Self(hundredths as u16))
        } else {
            Err(format!(
                "expected a damping ratio from {} to {}, got {ratio}",
                Self::MIN,
                Self::MAX
            ))
        }
    }

    #[must_use]
    pub fn ratio(self) -> f64 {
        f64::from(self.0) / 100.0
    }

    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for GlideDamping {
    fn default() -> Self {
        Self(100)
    }
}

impl Serialize for GlideDamping {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.ratio())
    }
}

impl<'de> Deserialize<'de> for GlideDamping {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(f64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Settings' one pick for the glide. Each is a duration at the default damping, so the file
/// holds a single key, or none for [`GlideMotion::Fast`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlideMotion {
    /// No glide: columns snap to where they rest.
    Off,
    /// The default glide.
    Fast,
    /// A longer, gentler glide.
    Smooth,
}

impl GlideMotion {
    pub const ALL: [Self; 3] = [Self::Off, Self::Fast, Self::Smooth];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Fast => "Fast",
            Self::Smooth => "Smooth",
        }
    }

    #[must_use]
    pub const fn duration(self) -> GlideDuration {
        GlideDuration(match self {
            Self::Off => 0,
            Self::Fast => SpringParams::DEFAULT_DURATION_MS,
            Self::Smooth => 260,
        })
    }

    /// The pick these values are, or `None` for a glide tuned by hand in the file. Without a
    /// glide the damping has nothing to shape, so any damping is [`GlideMotion::Off`].
    #[must_use]
    pub fn of(duration: GlideDuration, damping: GlideDamping) -> Option<Self> {
        Self::ALL.into_iter().find(|motion| {
            motion.duration() == duration && (duration.is_off() || damping.is_default())
        })
    }
}

/// The spring a glide rides, or `None` when there is no glide.
#[must_use]
pub fn spring(duration: GlideDuration, damping: GlideDamping) -> Option<SpringParams> {
    (!duration.is_off()).then(|| SpringParams::for_duration(duration.ms(), damping.ratio()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labs::LabsConfig;

    #[test]
    fn the_defaults_are_the_default_spring() {
        let labs = LabsConfig::default();
        let spring = labs.strip_spring().expect("a glide by default");
        assert!((spring.stiffness - SpringParams::default().stiffness).abs() < 1e-9);
        assert_eq!(spring.damping_ratio, 1.0);
        assert_eq!(spring.mass, 1.0);
    }

    #[test]
    fn a_shorter_glide_is_a_proportionally_faster_spring() {
        let half = SpringParams::for_duration(90, 1.0);
        let default = SpringParams::for_duration(180, 1.0);
        let double = SpringParams::for_duration(360, 1.0);
        assert!((half.angular_frequency() / default.angular_frequency() - 2.0).abs() < 1e-9);
        assert!((default.angular_frequency() / double.angular_frequency() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn tuning_round_trips_and_stays_out_of_the_file_at_its_defaults() {
        let source = concat!(
            "scrolling_strip = true\n",
            "scrolling_strip_duration_ms = 380\n",
            "scrolling_strip_damping = 0.78\n",
            "scrolling_strip_shared_edge = \"slide\"\n",
        );
        let labs: LabsConfig = toml::from_str(source).unwrap();
        assert_eq!(labs.scrolling_strip_duration_ms.ms(), 380);
        assert_eq!(labs.scrolling_strip_damping.ratio(), 0.78);
        assert_eq!(
            labs.scrolling_strip_shared_edge,
            crate::motion::SharedEdge::Slide
        );
        assert_eq!(toml::to_string(&labs).unwrap(), source);
        let whole: LabsConfig = toml::from_str("scrolling_strip_damping = 1").unwrap();
        assert!(whole.is_default());
        assert_eq!(toml::to_string(&LabsConfig::default()).unwrap(), "");
    }

    #[test]
    fn a_zero_duration_is_no_glide_and_has_no_spring() {
        let off: LabsConfig = toml::from_str("scrolling_strip_duration_ms = 0").unwrap();
        assert!(off.scrolling_strip_duration_ms.is_off());
        assert_eq!(off.strip_spring(), None);
        assert_eq!(
            toml::to_string(&off).unwrap(),
            "scrolling_strip_duration_ms = 0\n"
        );
        assert!(!GlideDuration::default().is_off());
        assert_eq!(GlideDuration::OFF, GlideDuration::new(0).unwrap());
    }

    #[test]
    fn motion_names_its_three_picks_and_leaves_a_hand_tuned_glide_unnamed() {
        let of = |source: &str| {
            let labs: LabsConfig = toml::from_str(source).unwrap();
            GlideMotion::of(
                labs.scrolling_strip_duration_ms,
                labs.scrolling_strip_damping,
            )
        };
        assert_eq!(of(""), Some(GlideMotion::Fast));
        assert_eq!(
            of("scrolling_strip_duration_ms = 180"),
            Some(GlideMotion::Fast)
        );
        assert_eq!(
            of("scrolling_strip_duration_ms = 260"),
            Some(GlideMotion::Smooth)
        );
        assert_eq!(
            of("scrolling_strip_duration_ms = 0"),
            Some(GlideMotion::Off)
        );
        assert_eq!(
            of("scrolling_strip_duration_ms = 0\nscrolling_strip_damping = 0.78"),
            Some(GlideMotion::Off),
            "damping shapes nothing without a glide"
        );
        assert_eq!(of("scrolling_strip_duration_ms = 400"), None);
        assert_eq!(of("scrolling_strip_damping = 0.78"), None);
        for motion in GlideMotion::ALL {
            assert_eq!(
                GlideMotion::of(motion.duration(), GlideDamping::default()),
                Some(motion)
            );
            assert!(GlideDuration::new(motion.duration().ms()).is_ok());
        }
        assert_eq!(GlideMotion::Fast.duration(), GlideDuration::default());
    }

    #[test]
    fn tuning_out_of_range_is_refused() {
        for bad in [
            "scrolling_strip_duration_ms = 1",
            "scrolling_strip_duration_ms = 79",
            "scrolling_strip_duration_ms = 901",
            "scrolling_strip_duration_ms = -5",
            "scrolling_strip_duration_ms = 70000",
            "scrolling_strip_damping = 0.5",
            "scrolling_strip_damping = 1.4",
            "scrolling_strip_damping = \"soft\"",
            "scrolling_strip_shared_edge = \"curtain\"",
        ] {
            assert!(toml::from_str::<LabsConfig>(bad).is_err(), "{bad}");
        }
        assert!(GlideDuration::new(80).is_ok() && GlideDuration::new(900).is_ok());
        assert!(GlideDamping::new(0.6).is_ok() && GlideDamping::new(1.3).is_ok());
    }
}
