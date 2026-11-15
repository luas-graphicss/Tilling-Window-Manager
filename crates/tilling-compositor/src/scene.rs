use tilling_core::geometry::LogicalRect;
use tilling_core::layout::{Gaps, Placement};
use tilling_core::{DesktopState, OutputId, WindowId, WindowMode};

pub const BORDER_WIDTH: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SceneLayer {
    Tiled,
    Floating,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneWindow {
    pub id: WindowId,
    pub output: OutputId,
    pub frame: LogicalRect,
    pub content: LogicalRect,
    pub layer: SceneLayer,
    pub focused: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scene {
    back_to_front: Vec<SceneWindow>,
}

impl SceneLayer {
    const fn of(mode: WindowMode) -> Self {
        match mode {
            WindowMode::Tiled => Self::Tiled,
            WindowMode::Floating(_) => Self::Floating,
            WindowMode::Fullscreen(_) => Self::Fullscreen,
        }
    }

    const fn border(self) -> u32 {
        match self {
            Self::Tiled | Self::Floating => BORDER_WIDTH,
            Self::Fullscreen => 0,
        }
    }
}

impl Scene {
    pub(crate) fn build(desktop: &DesktopState, gaps: Gaps) -> Self {
        let focused = desktop.focused_window();
        let mut back_to_front = Vec::new();
        for (output_id, output) in desktop.outputs() {
            let Some(placements) = desktop.placements(output.workspace(), gaps) else {
                continue;
            };
            let mut placed: Vec<SceneWindow> = placements
                .into_iter()
                .filter_map(|placement| {
                    let mode = desktop.window(placement.window)?.mode();
                    Some(place(placement, output_id, SceneLayer::of(mode), focused))
                })
                .collect();
            placed.sort_by_key(|window| (window.layer, raised(window)));
            back_to_front.extend(placed);
        }
        Self { back_to_front }
    }

    #[must_use]
    pub fn windows(&self) -> &[SceneWindow] {
        &self.back_to_front
    }

    #[must_use]
    pub fn window(&self, id: WindowId) -> Option<&SceneWindow> {
        self.back_to_front.iter().find(|window| window.id == id)
    }

    pub(crate) fn remove(&mut self, id: WindowId) -> bool {
        let before = self.back_to_front.len();
        self.back_to_front.retain(|window| window.id != id);
        self.back_to_front.len() != before
    }
}

fn place(
    placement: Placement,
    output: OutputId,
    layer: SceneLayer,
    focused: Option<WindowId>,
) -> SceneWindow {
    SceneWindow {
        id: placement.window,
        output,
        frame: placement.area,
        content: placement.area.shrink(layer.border()),
        layer,
        focused: focused == Some(placement.window),
    }
}

fn raised(window: &SceneWindow) -> bool {
    window.layer == SceneLayer::Floating && window.focused
}

#[cfg(test)]
mod tests {
    use tilling_core::geometry::Scale;
    use tilling_core::{Command, Effect, ModeChange, WindowMetadata};

    use super::*;

    const GAPS: Gaps = Gaps { inner: 8, outer: 8 };

    fn logical(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    fn desktop() -> DesktopState {
        let mut desktop = DesktopState::new("1");
        desktop
            .apply(Command::AddOutput {
                name: "TEST-1".into(),
                area: logical(0, 0, 1280, 720),
                scale: Scale::ONE,
            })
            .unwrap();
        desktop
    }

    fn open(desktop: &mut DesktopState, mode: WindowMode) -> WindowId {
        let transition = desktop
            .apply(Command::OpenWindow {
                metadata: WindowMetadata::default(),
                workspace: None,
                transient_for: None,
                mode,
            })
            .unwrap();
        transition
            .effects
            .iter()
            .find_map(|effect| match effect {
                Effect::WindowOpened(window) => Some(*window),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn tiles_keep_a_border_inside_their_frame() {
        let mut desktop = desktop();
        let window = open(&mut desktop, WindowMode::Tiled);

        let scene = Scene::build(&desktop, GAPS);
        let placed = scene.window(window).unwrap();

        assert_eq!(placed.frame, logical(8, 8, 1264, 704));
        assert_eq!(placed.content, logical(10, 10, 1260, 700));
        assert!(placed.focused);
    }

    #[test]
    fn fullscreen_covers_the_output_without_a_border() {
        let mut desktop = desktop();
        let window = open(&mut desktop, WindowMode::Tiled);
        desktop
            .apply(Command::ChangeWindowMode {
                window,
                change: ModeChange::EnterFullscreen,
            })
            .unwrap();

        let scene = Scene::build(&desktop, GAPS);
        let placed = scene.window(window).unwrap();

        assert_eq!(placed.layer, SceneLayer::Fullscreen);
        assert_eq!(placed.content, logical(0, 0, 1280, 720));
    }

    #[test]
    fn focused_floating_window_is_drawn_above_other_floating_windows() {
        let mut desktop = desktop();
        let tile = open(&mut desktop, WindowMode::Tiled);
        let lower = open(
            &mut desktop,
            WindowMode::Floating(logical(100, 100, 300, 200)),
        );
        let upper = open(
            &mut desktop,
            WindowMode::Floating(logical(150, 150, 300, 200)),
        );
        desktop
            .apply(Command::FocusWindow { window: lower })
            .unwrap();

        let scene = Scene::build(&desktop, GAPS);
        let order: Vec<WindowId> = scene.windows().iter().map(|window| window.id).collect();

        assert_eq!(order, [tile, upper, lower]);
    }

    #[test]
    fn windows_of_hidden_workspaces_are_not_part_of_the_scene() {
        let mut desktop = desktop();
        let window = open(&mut desktop, WindowMode::Tiled);
        let transition = desktop
            .apply(Command::CreateWorkspace { name: "2".into() })
            .unwrap();
        let hidden = transition
            .effects
            .iter()
            .find_map(|effect| match effect {
                Effect::WorkspaceCreated(workspace) => Some(*workspace),
                _ => None,
            })
            .unwrap();
        desktop
            .apply(Command::SwitchWorkspace { workspace: hidden })
            .unwrap();

        let scene = Scene::build(&desktop, GAPS);

        assert_eq!(scene.window(window), None);
        assert!(scene.windows().is_empty());
    }
}
