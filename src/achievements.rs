//! Asteroids achievement definitions.
//!
//! Registered once in `init()`. Wave/untouchable achievements unlock from
//! `check_wave_clear`, score tiers from `award_points`, and the skill set
//! (sharpshooter, close call, double tap) from `shatter_asteroid`.

use engine_core::prelude::*;

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const WAVE5_NORMAL: &str = "wave5_normal";
pub(crate) const WAVE5_INSANE: &str = "wave5_insane";
pub(crate) const WAVE5_RIDICULOUS: &str = "wave5_ridiculous";
pub(crate) const WAVE5_INSICULOUS: &str = "wave5_insiculous";

pub(crate) const SCORE_10K: &str = "score_10k";
pub(crate) const SCORE_30K: &str = "score_30k";

pub(crate) const SHARPSHOOTER: &str = "sharpshooter";
pub(crate) const CLOSE_CALL: &str = "close_call";
pub(crate) const UNTOUCHABLE: &str = "untouchable";
pub(crate) const DOUBLE_TAP: &str = "double_tap";
pub(crate) const LIVING_ON_EDGE: &str = "living_on_edge";

/// Grouped display order for the achievements page. First tuple element is
/// the section header, second is the list of ids to render under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("Wave Milestones (wave 5 per mode)",
        &[WAVE5_NORMAL, WAVE5_INSANE, WAVE5_RIDICULOUS, WAVE5_INSICULOUS]),
    ("Score",
        &[SCORE_10K, SCORE_30K]),
    ("Skill",
        &[SHARPSHOOTER, CLOSE_CALL, UNTOUCHABLE, DOUBLE_TAP, LIVING_ON_EDGE]),
];

/// Register every Asteroids achievement. Call once from `Game::init`.
pub(crate) fn register_all(mgr: &mut AchievementManager) {
    mgr.register(Achievement::new(WAVE5_NORMAL,
        "Rock Steady",
        "Reach wave 5 in Normal mode."));
    mgr.register(Achievement::new(WAVE5_INSANE,
        "Belt at Full Speed",
        "Reach wave 5 in Insane mode."));
    mgr.register(Achievement::new(WAVE5_RIDICULOUS,
        "Triple Trouble",
        "Reach wave 5 in Ridiculous mode."));
    mgr.register(Achievement::new(WAVE5_INSICULOUS,
        "Insiculous Belt Runner",
        "Reach wave 5 in Insiculous mode."));

    mgr.register(Achievement::new(SCORE_10K,
        "Ten Grand",
        "Score 10,000 points."));
    mgr.register(Achievement::new(SCORE_30K,
        "Asteroid Legend",
        "Score 30,000 points."));

    mgr.register(Achievement::new(SHARPSHOOTER,
        "Sharpshooter",
        "Land 10 hits in a row without a shot expiring."));
    mgr.register(Achievement::new(CLOSE_CALL,
        "Close Call",
        "Shatter a rock within 60 pixels of your ship."));
    mgr.register(Achievement::new(UNTOUCHABLE,
        "Untouchable",
        "Clear 3 waves in a row without dying."));
    mgr.register(Achievement::new(DOUBLE_TAP,
        "Double Tap",
        "Shatter two rocks within half a second."));
    mgr.register(Achievement::new(LIVING_ON_EDGE,
        "Living on the Edge",
        "Reach wave 5 on your last ship."));
}

pub(crate) fn wave5_id(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => WAVE5_NORMAL,
        ChaosMode::Insane => WAVE5_INSANE,
        ChaosMode::Ridiculous => WAVE5_RIDICULOUS,
        ChaosMode::Insiculous => WAVE5_INSICULOUS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_all_adds_eleven() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        assert_eq!(mgr.total(), 11);
    }

    #[test]
    fn wave5_id_maps_each_mode() {
        assert_eq!(wave5_id(ChaosMode::Normal), WAVE5_NORMAL);
        assert_eq!(wave5_id(ChaosMode::Insane), WAVE5_INSANE);
        assert_eq!(wave5_id(ChaosMode::Ridiculous), WAVE5_RIDICULOUS);
        assert_eq!(wave5_id(ChaosMode::Insiculous), WAVE5_INSICULOUS);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        for id in [
            WAVE5_NORMAL, WAVE5_INSANE, WAVE5_RIDICULOUS, WAVE5_INSICULOUS,
            SCORE_10K, SCORE_30K,
            SHARPSHOOTER, CLOSE_CALL, UNTOUCHABLE, DOUBLE_TAP, LIVING_ON_EDGE,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }
}
