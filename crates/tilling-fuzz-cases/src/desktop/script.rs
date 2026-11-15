use tilling_core::geometry::{LogicalRect, Scale};
use tilling_core::layout::{Direction, LayoutKind};
use tilling_core::{
    Command, DesktopState, ModeChange, OutputId, RestoreMode, WindowId, WindowMetadata, WindowMode,
    WorkspaceId,
};

const COMMAND_KINDS: u8 = 21;
const STALE_WINDOW_SELECTOR: usize = 224;
const SCALE_DENOMINATOR: f64 = 120.0;
const APPLICATIONS: u8 = 5;
const DIRECTIONS: [Direction; 4] = [
    Direction::Left,
    Direction::Right,
    Direction::Up,
    Direction::Down,
];
const LAYOUTS: [LayoutKind; 3] = [
    LayoutKind::Bsp,
    LayoutKind::MasterStack,
    LayoutKind::Monocle,
];

pub(super) struct Script<'input> {
    unread: &'input [u8],
}

struct Targets<'state> {
    windows: Vec<WindowId>,
    closed: &'state [WindowId],
    workspaces: Vec<WorkspaceId>,
    outputs: Vec<OutputId>,
}

impl<'input> Script<'input> {
    pub(super) fn new(input: &'input [u8]) -> Self {
        Self { unread: input }
    }

    pub(super) fn next_command(
        &mut self,
        state: &DesktopState,
        closed: &[WindowId],
    ) -> Option<Command> {
        let targets = Targets {
            windows: state.windows().map(|(id, _)| id).collect(),
            closed,
            workspaces: state.workspaces().map(|(id, _)| id).collect(),
            outputs: state.outputs().map(|(id, _)| id).collect(),
        };
        loop {
            let kind = self.byte()? % COMMAND_KINDS;
            if let Some(command) = self.command(kind, &targets) {
                return Some(command);
            }
        }
    }

    fn command(&mut self, kind: u8, targets: &Targets<'_>) -> Option<Command> {
        Some(match kind {
            0 => Command::AddOutput {
                name: self.name()?,
                area: self.area()?,
                scale: self.scale()?,
            },
            1 => Command::ConfigureOutput {
                output: self.output(targets)?,
                area: self.area()?,
                scale: self.scale()?,
            },
            2 => Command::RemoveOutput {
                output: self.output(targets)?,
            },
            3 => Command::CreateWorkspace { name: self.name()? },
            4 => Command::SwitchWorkspace {
                workspace: self.workspace(targets)?,
            },
            5 => Command::SetLayout {
                workspace: self.workspace(targets)?,
                layout: self.layout()?,
            },
            6 => Command::SetDefaultLayout {
                layout: self.layout()?,
            },
            7 => self.open_window(targets)?,
            8 => Command::CloseWindow {
                window: self.window(targets)?,
            },
            9 => Command::UpdateWindowMetadata {
                window: self.window(targets)?,
                metadata: self.metadata()?,
            },
            10 => Command::FocusWindow {
                window: self.window(targets)?,
            },
            11 => Command::FocusDirection {
                direction: self.direction()?,
            },
            12 => Command::MoveWindow {
                window: self.window(targets)?,
                direction: self.direction()?,
            },
            13 => Command::MoveWindowToWorkspace {
                window: self.window(targets)?,
                workspace: self.workspace(targets)?,
            },
            14 => Command::ChangeWindowMode {
                window: self.window(targets)?,
                change: self.mode_change()?,
            },
            15 => Command::StashWindow {
                window: self.window(targets)?,
                area: self.area()?,
            },
            16 => Command::ToggleScratchpad,
            17 => Command::FocusOutput {
                direction: self.direction()?,
            },
            18 => Command::MoveWindowToOutput {
                window: self.window(targets)?,
                direction: self.direction()?,
            },
            19 => Command::ToggleSplit {
                window: self.window(targets)?,
            },
            _ => Command::ResizeSplit {
                window: self.window(targets)?,
                delta: i16::from_be_bytes(self.bytes()?),
            },
        })
    }

    fn open_window(&mut self, targets: &Targets<'_>) -> Option<Command> {
        let metadata = self.metadata()?;
        let choices = self.byte()?;
        let workspace = match choices & 0b0001 {
            0 => None,
            _ => Some(self.workspace(targets)?),
        };
        let transient_for = match choices & 0b0010 {
            0 => None,
            _ => Some(self.window(targets)?),
        };
        let mode = match (choices >> 2) & 0b0011 {
            0 => WindowMode::Tiled,
            1 => WindowMode::Floating(self.area()?),
            2 => WindowMode::Fullscreen(RestoreMode::Tiled),
            _ => WindowMode::Fullscreen(RestoreMode::Floating(self.area()?)),
        };
        Some(Command::OpenWindow {
            metadata,
            workspace,
            transient_for,
            mode,
        })
    }

    fn mode_change(&mut self) -> Option<ModeChange> {
        Some(match self.byte()? & 0b0011 {
            0 => ModeChange::Tile,
            1 => ModeChange::Float(self.area()?),
            2 => ModeChange::EnterFullscreen,
            _ => ModeChange::LeaveFullscreen,
        })
    }

    fn window(&mut self, targets: &Targets<'_>) -> Option<WindowId> {
        let selector = usize::from(self.byte()?);
        if selector >= STALE_WINDOW_SELECTOR && !targets.closed.is_empty() {
            return pick(targets.closed, selector);
        }
        pick(&targets.windows, selector)
    }

    fn workspace(&mut self, targets: &Targets<'_>) -> Option<WorkspaceId> {
        pick(&targets.workspaces, usize::from(self.byte()?))
    }

    fn output(&mut self, targets: &Targets<'_>) -> Option<OutputId> {
        pick(&targets.outputs, usize::from(self.byte()?))
    }

    fn direction(&mut self) -> Option<Direction> {
        pick(&DIRECTIONS, usize::from(self.byte()?))
    }

    fn layout(&mut self) -> Option<LayoutKind> {
        pick(&LAYOUTS, usize::from(self.byte()?))
    }

    fn name(&mut self) -> Option<String> {
        Some(match self.byte()? {
            0 => String::new(),
            suffix => format!("scripted-{suffix}"),
        })
    }

    fn metadata(&mut self) -> Option<WindowMetadata> {
        let choices = self.byte()?;
        let application = choices % APPLICATIONS;
        Some(WindowMetadata {
            app_id: (choices & 0b1000_0000 == 0).then(|| format!("app-{application}")),
            title: (choices & 0b0100_0000 == 0).then(|| format!("title {choices}")),
        })
    }

    fn area(&mut self) -> Option<LogicalRect> {
        let x = i32::from_be_bytes(self.bytes()?);
        let y = i32::from_be_bytes(self.bytes()?);
        let width = u32::from_be_bytes(self.bytes()?);
        let height = u32::from_be_bytes(self.bytes()?);
        LogicalRect::new(x, y, width, height).ok()
    }

    fn scale(&mut self) -> Option<Scale> {
        let numerator = u16::from_be_bytes(self.bytes()?);
        Scale::from_factor(f64::from(numerator) / SCALE_DENOMINATOR).ok()
    }

    fn byte(&mut self) -> Option<u8> {
        let (&first, rest) = self.unread.split_first()?;
        self.unread = rest;
        Some(first)
    }

    fn bytes<const COUNT: usize>(&mut self) -> Option<[u8; COUNT]> {
        let (taken, rest) = self.unread.split_first_chunk::<COUNT>()?;
        self.unread = rest;
        Some(*taken)
    }
}

fn pick<T: Copy>(options: &[T], selector: usize) -> Option<T> {
    options.get(selector.checked_rem(options.len())?).copied()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::desktop) const ADD_OUTPUT: &[u8] = &[
        0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 128, 0, 0, 4, 56, 0, 120,
    ];
    pub(in crate::desktop) const OPEN_TILED_WINDOW: &[u8] = &[7, 2, 0];

    fn only_command(input: &[u8]) -> Option<Command> {
        Script::new(input).next_command(&DesktopState::new("1"), &[])
    }

    #[test]
    fn output_is_read_with_its_area_and_scale() {
        let expected = Command::AddOutput {
            name: "scripted-1".to_owned(),
            area: LogicalRect::new(0, 0, 1920, 1080).unwrap(),
            scale: Scale::ONE,
        };

        assert_eq!(only_command(ADD_OUTPUT), Some(expected));
    }

    #[test]
    fn window_opens_tiled_with_metadata_taken_from_one_byte() {
        let expected = Command::OpenWindow {
            metadata: WindowMetadata {
                app_id: Some("app-2".to_owned()),
                title: Some("title 2".to_owned()),
            },
            workspace: None,
            transient_for: None,
            mode: WindowMode::Tiled,
        };

        assert_eq!(only_command(OPEN_TILED_WINDOW), Some(expected));
    }

    #[test]
    fn command_without_a_target_is_skipped_for_the_next_one() {
        let close_then_toggle = [8, 0, 16];

        assert_eq!(
            only_command(&close_then_toggle),
            Some(Command::ToggleScratchpad)
        );
    }

    #[test]
    fn rejected_geometry_skips_the_command_instead_of_ending_the_script() {
        let empty_output_then_toggle = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];

        assert_eq!(
            only_command(&empty_output_then_toggle),
            Some(Command::ToggleScratchpad)
        );
    }

    #[test]
    fn exhausted_input_ends_the_script() {
        assert_eq!(only_command(&[]), None);
        assert_eq!(only_command(&[0, 1, 0, 0]), None);
    }
}
