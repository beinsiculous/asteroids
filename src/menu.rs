//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`.

use engine_core::prelude::*;
use crate::types::*;

/// Title-screen menu entries. Both halves (input hit-testing here, drawing
/// in `drawing.rs`) derive rows from `TITLE_ITEMS`, so keyboard, mouse, and
/// the drawn panel can never disagree about what row N is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleItem {
    OnePlayer,
    TwoPlayerCoop,
    Achievements,
    Exit,
}

/// The rows the title screen offers. On the web the achievements page lives
/// on the site (it reads the shared localStorage save), so the in-game
/// Achievements entry is native-only.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::OnePlayer,
    TitleItem::TwoPlayerCoop,
    TitleItem::Achievements,
    TitleItem::Exit,
];
#[cfg(target_arch = "wasm32")]
pub(crate) const TITLE_ITEMS: &[TitleItem] =
    &[TitleItem::OnePlayer, TitleItem::TwoPlayerCoop, TitleItem::Exit];

/// Row index of `item` on this build's title menu (0 if absent — only
/// reachable for rows this build offers, so the fallback is theoretical).
pub(crate) fn title_index(item: TitleItem) -> u8 {
    TITLE_ITEMS.iter().position(|i| *i == item).unwrap_or(0) as u8
}

pub(crate) fn title_label(item: TitleItem) -> &'static str {
    match item {
        TitleItem::OnePlayer => "1 Player",
        TitleItem::TwoPlayerCoop => "2 Player Co-op",
        TitleItem::Achievements => "Achievements",
        TitleItem::Exit => "Exit",
    }
}

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`drawing.rs`) — the geometry must match or clicks land beside
/// the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 380.0, TITLE_ITEMS.len())
}
pub(crate) fn mode_select_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 400.0, ChaosMode::ALL.len())
}
pub(crate) fn achievements_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, window_size.x - 120.0, 15)
}

/// One-line description of what each chaos mode means in this game.
pub(crate) fn mode_hint(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "The classic belt. Rocks split, you drift.",
        ChaosMode::Insane => "Rocks fly 60% faster and mercy is brief.",
        ChaosMode::Ridiculous => "Big rocks shatter into three - and bring a friend.",
        ChaosMode::Insiculous => "Triple splits, extra rocks, everything faster.",
    }
}

impl AsteroidsGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = title_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        // An out-of-range stored selection (e.g. the shorter wasm menu)
        // clamps instead of panicking at the dispatch index below.
        let selection = selection.min(TITLE_ITEMS.len() as u8 - 1);
        let mut selection = input.navigate(selection, TITLE_ITEMS.len() as u8);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match TITLE_ITEMS[selection as usize] {
                TitleItem::OnePlayer => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                TitleItem::TwoPlayerCoop => {
                    self.mode = GameMode::TwoPlayerCoop;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                TitleItem::Achievements => self.state = GameState::Achievements,
                TitleItem::Exit => ctx.exit_requested = true,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // The page is one big non-selectable list: any click on it dismisses,
        // same as confirm/back.
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen {
                selection: title_index(TitleItem::Achievements),
            };
        }
    }

    pub(crate) fn update_mode_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = mode_select_panel("", ctx.window_size).mouse_select(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, count);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::ModeSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
            self.chaos_mode = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx);
        }
    }
}
