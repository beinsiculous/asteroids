//! UI text: menus, HUD, and the game-over overlay. World rendering (ship,
//! rocks) is wireframe line art — see `wireframe.rs`.

use engine_core::prelude::*;
use crate::achievements::DISPLAY_SECTIONS;
use crate::menu::mode_hint;
use crate::types::*;

impl AsteroidsGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&ChaosTheme::for_mode(self.chaos_mode))
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::ModeSelect { selection } => self.draw_mode_select(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("INSICULOUS ASTEROIDS", ctx.window_size / 2.0, 380.0, 4);
        let mut y = panel.begin(ctx.ui, &style);
        let items = ["1 Player", "2 Player Co-op", "Achievements", "Exit"];
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, "Navigate to move, confirm to select", &style);

        // Control scheme, printed beneath the window.
        let rect = panel.panel_rect();
        let cx = ctx.window_size.x / 2.0;
        ctx.ui.label_centered("P1 WASD + SPACE   -   P2 Arrows + ENTER", Vec2::new(cx, rect.y + rect.height + 24.0));
        ctx.ui.label_centered("A/D rotate - W thrust - fire to shoot", Vec2::new(cx, rect.y + rect.height + 48.0));
    }

    fn draw_mode_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("SELECT CHAOS MODE", ctx.window_size / 2.0, 400.0, ChaosMode::ALL.len());
        let mut y = panel.begin(ctx.ui, &style);
        for (i, &mode) in ChaosMode::ALL.iter().enumerate() {
            // Each entry glows in its chaos mode's banner color.
            let c = ChaosTheme::for_mode(mode).banner_color;
            y = panel.item_colored(ctx.ui, y, mode.label(), c, i as u8 == selection, &style);
        }
        panel.hint(
            ctx.ui,
            mode_hint(ChaosMode::ALL[selection as usize % ChaosMode::ALL.len()]),
            &style,
        );
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        // Tall window; the section list draws left-aligned inside it.
        let panel = MenuPanel::new("ACHIEVEMENTS", ctx.window_size / 2.0, ctx.window_size.x - 120.0, 15);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} unlocked"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section, ids) in DISPLAY_SECTIONS {
            ctx.ui.label_styled(section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                // Registry always has entries for these ids (registered in init).
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                ctx.ui.label_styled(
                    &format!("{marker} {}", ach.name),
                    Vec2::new(left + 8.0, y),
                    name_color,
                    14.0,
                );
                ctx.ui.label_styled(&ach.description, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, "ESC or SPACE to go back", &style);
    }

    fn draw_gameplay(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        self.draw_hud(ctx, cx);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(banner) = theme.banner_text {
            let color = Color::new(
                theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w,
            );
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 24.0), color, 16.0);
        }

        if self.state == GameState::Playing {
            ctx.ui.label("ESC to pause", Vec2::new(40.0, ctx.window_size.y - 24.0));
        }

        if self.state == GameState::GameOver {
            let style = self.menu_style();
            let panel = MenuPanel::new("OUT OF SHIPS", Vec2::new(cx, cy), 400.0, 2);
            let mut y = panel.begin(ctx.ui, &style);
            y = panel.line(ctx.ui, y, &self.final_score_line(), &style);
            panel.line(ctx.ui, y, "SPACE / ENTER to play again", &style);
            panel.hint(ctx.ui, "ESC for title screen", &style);
        }

        if self.pause.is_active() {
            self.pause.draw(ctx.ui, ctx.window_size, &self.menu_style());
        }
    }

    /// Wave is always centered. Single player shows one score/ships pair;
    /// co-op puts player 1 on the left and player 2 on the right.
    fn draw_hud(&self, ctx: &mut GameContext, cx: f32) {
        ctx.ui.label_centered(&format!("WAVE {}", self.wave), Vec2::new(cx, 16.0));
        match self.mode {
            GameMode::SinglePlayer => {
                let (score, lives) = self.ship_hud(0);
                ctx.ui.label(&format!("SCORE {score}"), Vec2::new(40.0, 16.0));
                ctx.ui.label(&format!("SHIPS {lives}"), Vec2::new(ctx.window_size.x - 120.0, 16.0));
            }
            GameMode::TwoPlayerCoop => {
                let (s1, l1) = self.ship_hud(0);
                let (s2, l2) = self.ship_hud(1);
                ctx.ui.label(&format!("P1 {s1}  SHIPS {l1}"), Vec2::new(40.0, 16.0));
                ctx.ui.label(
                    &format!("P2 {s2}  SHIPS {l2}"),
                    Vec2::new(ctx.window_size.x - 220.0, 16.0),
                );
            }
        }
    }

    /// A ship's `(score, lives)` for the HUD, or zeros if that slot is empty.
    fn ship_hud(&self, index: usize) -> (u32, u32) {
        self.ships.get(index).map(|s| (s.score, s.lives)).unwrap_or((0, 0))
    }

    /// Game-over score summary: the lone score in solo, both players in co-op.
    fn final_score_line(&self) -> String {
        match self.mode {
            GameMode::SinglePlayer => {
                let (score, _) = self.ship_hud(0);
                format!("Final score: {}  -  wave {}", score, self.wave)
            }
            GameMode::TwoPlayerCoop => {
                let (s1, _) = self.ship_hud(0);
                let (s2, _) = self.ship_hud(1);
                format!("P1 {s1}   P2 {s2}   -   wave {}", self.wave)
            }
        }
    }
}
