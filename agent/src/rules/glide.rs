//! The Scrolling Strip's glide tuning in `[labs]`, edited in place like the rest of the file.
//! A key at its default leaves the file; one the file holds out of range reads as the default.

use spaceman::labs::{
    DAMPING_KEY, DURATION_KEY, GlideDamping, GlideDuration, GlideMotion, SHARED_EDGE_KEY,
    SharedEdge,
};
use toml_edit::{Item, value};

use super::Doc;

impl Doc {
    fn lab_item(&self, key: &str) -> Option<&Item> {
        self.doc
            .get("labs")
            .and_then(Item::as_table_like)
            .and_then(|table| table.get(key))
    }

    pub fn strip_duration(&self) -> GlideDuration {
        self.lab_item(DURATION_KEY)
            .and_then(Item::as_integer)
            .and_then(|ms| u16::try_from(ms).ok())
            .and_then(|ms| GlideDuration::new(ms).ok())
            .unwrap_or_default()
    }

    /// Settings' pick for the glide, or `None` when the file tunes it by hand.
    pub fn strip_motion(&self) -> Option<GlideMotion> {
        GlideMotion::of(self.strip_duration(), self.strip_damping())
    }

    /// Writes `motion` as the file's one glide key (none for the default), dropping a
    /// hand-tuned damping so the pick is exactly what runs.
    pub fn set_strip_motion(&mut self, motion: GlideMotion) {
        let duration = motion.duration();
        let item = (!duration.is_default()).then(|| value(i64::from(duration.ms())));
        self.set_key("labs", DURATION_KEY, item);
        self.set_key("labs", DAMPING_KEY, None);
    }

    pub fn strip_damping(&self) -> GlideDamping {
        self.lab_item(DAMPING_KEY)
            .and_then(|item| {
                item.as_float()
                    .or_else(|| item.as_integer().map(|whole| whole as f64))
            })
            .and_then(|ratio| GlideDamping::new(ratio).ok())
            .unwrap_or_default()
    }

    pub fn strip_shared_edge(&self) -> SharedEdge {
        self.lab_item(SHARED_EDGE_KEY)
            .and_then(Item::as_str)
            .and_then(SharedEdge::parse)
            .unwrap_or_default()
    }

    pub fn set_strip_shared_edge(&mut self, edge: SharedEdge) {
        let item = (!edge.is_default()).then(|| value(edge.as_str()));
        self.set_key("labs", SHARED_EDGE_KEY, item);
    }
}

#[cfg(test)]
mod tests {
    use spaceman::{
        config::Config,
        labs::{GlideMotion, LabFeature, SharedEdge},
    };

    use super::Doc;

    #[test]
    fn motion_writes_one_key_and_leaves_the_default_out() {
        let source = "version = 1\n\n[labs]\nscrolling_strip = true\n";
        let mut doc = Doc::parse(source);
        assert_eq!(doc.strip_motion(), Some(GlideMotion::Fast));
        assert_eq!(doc.strip_shared_edge(), SharedEdge::Tuck);

        doc.set_strip_motion(GlideMotion::Smooth);
        doc.set_strip_shared_edge(SharedEdge::Slide);
        assert_eq!(doc.strip_motion(), Some(GlideMotion::Smooth));
        assert_eq!(doc.strip_shared_edge(), SharedEdge::Slide);
        let tuned = doc.source();
        assert!(
            tuned.contains("scrolling_strip_duration_ms = 260"),
            "{tuned}"
        );
        assert!(!tuned.contains("scrolling_strip_damping"), "{tuned}");
        assert!(
            tuned.contains("scrolling_strip_shared_edge = \"slide\""),
            "{tuned}"
        );
        let labs = Config::parse(&tuned).unwrap().labs();
        assert_eq!(labs.scrolling_strip_duration_ms.ms(), 260);
        assert_eq!(labs.scrolling_strip_shared_edge, SharedEdge::Slide);
        assert_eq!(labs, doc.labs());

        doc.set_strip_motion(GlideMotion::Off);
        assert!(
            doc.source().contains("scrolling_strip_duration_ms = 0"),
            "{}",
            doc.source()
        );
        assert_eq!(doc.strip_motion(), Some(GlideMotion::Off));
        assert_eq!(doc.labs().strip_spring(), None);
        assert_eq!(Config::parse(&doc.source()).unwrap().labs(), doc.labs());

        doc.set_strip_motion(GlideMotion::Fast);
        doc.set_strip_shared_edge(SharedEdge::default());
        assert_eq!(doc.source(), source);
        // Switching the Lab off leaves no empty table behind.
        doc.set_lab(LabFeature::ScrollingStrip, false);
        assert!(!doc.source().contains("[labs]"));
    }

    #[test]
    fn a_hand_tuned_glide_is_no_pick_until_one_is_made() {
        let mut doc = Doc::parse(
            "version = 1\n[labs]\nscrolling_strip_duration_ms = 400\n\
             scrolling_strip_damping = 0.78\n",
        );
        assert_eq!(doc.strip_motion(), None);
        assert_eq!(doc.strip_duration().ms(), 400);
        assert_eq!(doc.strip_damping().ratio(), 0.78);
        // A pick replaces both keys.
        doc.set_strip_motion(GlideMotion::Fast);
        assert_eq!(doc.strip_motion(), Some(GlideMotion::Fast));
        assert!(
            !doc.source().contains("scrolling_strip_d"),
            "{}",
            doc.source()
        );
    }

    #[test]
    fn out_of_range_tuning_reads_as_the_default() {
        let doc = Doc::parse(
            "version = 1\n[labs]\nscrolling_strip_duration_ms = 5\n\
             scrolling_strip_damping = 1\nscrolling_strip_shared_edge = \"sideways\"\n",
        );
        assert!(doc.strip_duration().is_default());
        assert!(doc.strip_damping().is_default());
        assert_eq!(doc.strip_motion(), Some(GlideMotion::Fast));
        assert_eq!(doc.strip_shared_edge(), SharedEdge::Tuck);
    }
}
