use tilling_core::Command;
use tilling_core::layout::LayoutKind;

use crate::runtime::Compositor;

impl Compositor {
    pub(super) fn set_active_layout(&mut self, layout: LayoutKind) {
        let workspace = self.desktop.active_workspace();
        self.run(Command::SetLayout { workspace, layout });
    }

    pub(super) fn cycle_layout(&mut self) {
        let active = self.desktop.workspace(self.desktop.active_workspace());
        if let Some(current) = active.map(|workspace| workspace.layout().kind()) {
            self.set_active_layout(following(current));
        }
    }

    pub(super) fn stash_focused(&mut self) {
        let Some(window) = self.desktop.focused_window() else {
            return;
        };
        let area = self
            .desktop
            .window(window)
            .zip(self.windows.by_id(window))
            .and_then(|(state, managed)| self.floating_area(state.workspace(), managed));
        if let Some(area) = area {
            self.run(Command::StashWindow { window, area });
        }
    }
}

fn following(layout: LayoutKind) -> LayoutKind {
    match layout {
        LayoutKind::Bsp => LayoutKind::MasterStack,
        LayoutKind::MasterStack => LayoutKind::Monocle,
        LayoutKind::Monocle => LayoutKind::Bsp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycling_visits_every_layout_before_returning_to_bsp() {
        let visited = [
            following(LayoutKind::Bsp),
            following(following(LayoutKind::Bsp)),
            following(following(following(LayoutKind::Bsp))),
        ];

        assert_eq!(
            visited,
            [
                LayoutKind::MasterStack,
                LayoutKind::Monocle,
                LayoutKind::Bsp
            ]
        );
    }
}
