//! The Scrolling Strip's glide tuning in `[labs]`: how long a glide feels, how springy it is,
//! and how columns meet an edge shared with another monitor. Each key is left out of
//! `config.toml` at its default, and the worker reloads them before its next command.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::motion::SpringParams;

/// The tuning's keys in `[labs]`.
pub const DURATION_KEY: &str = "scrolling_strip_duration_ms";
pub const DAMPING_KEY: &str = "scrolling_strip_damping";
pub const SHARED_EDGE_KEY: &str = "scrolling_strip_shared_edge";

/// `scrolling_strip_duration_ms`: how long a glide feels, in milliseconds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlideDuration(u16);

impl GlideDuration {
    pub const MIN: u16 = 80;
    pub const MAX: u16 = 900;
    /// The Settings presets, fastest first.
    pub const PRESETS: [u16; 4] = [120, 180, 280, 400];

    /// `ms` when it is in range.
    pub fn new(ms: u16) -> Result<Self, String> {
        if (Self::MIN..=Self::MAX).contains(&ms) {
            Ok(Self(ms))
        } else {
            Err(format!(
                "expected {}..={} milliseconds, got {ms}",
                Self::MIN,
                Self::MAX
            ))
        }
    }

    #[must_use]
    pub const fn ms(self) -> u16 {
        self.0
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
            .map_err(|_| {
                format!(
                    "expected {}..={} milliseconds, got {ms}",
                    Self::MIN,
                    Self::MAX
                )
            })
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
    /// The Settings presets: Bouncy, Snappy, Soft.
    pub const PRESETS: [(&str, f64); 3] = [("Bouncy", 0.78), ("Snappy", 1.0), ("Soft", 1.18)];

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

#[must_use]
pub fn spring(duration: GlideDuration, damping: GlideDamping) -> SpringParams {
    SpringParams::for_duration(duration.ms(), damping.ratio())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::labs::LabsConfig;

    #[test]
    fn the_defaults_are_the_default_spring() {
        let labs = LabsConfig::default();
        let spring = labs.strip_spring();
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
    fn tuning_out_of_range_is_refused() {
        for bad in [
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
