use smithay::desktop::layer_map_for_output;
use smithay::output::Output;
use smithay::utils::{Logical, Rectangle};
use tilling_core::Command;
use tilling_core::geometry::{Insets, LogicalRect};

use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn arrange_layers(&mut self, output: &Output) {
        layer_map_for_output(output).arrange();
        self.reserve_layer_edges(output);
    }

    pub(crate) fn reserve_layer_edges(&mut self, output: &Output) {
        let Some(id) = self.outputs.id_of(output) else {
            return;
        };
        let Some(area) = self.desktop.output(id).map(|known| known.area()) else {
            return;
        };
        let free = layer_map_for_output(output).non_exclusive_zone();
        let command = Command::ReserveOutputEdges {
            output: id,
            edges: reserved_edges(area, free),
        };
        if let Err(error) = self.execute(command) {
            tracing::warn!(%error, output = %output.name(), "keep the previous reserved edges");
        }
    }
}

fn reserved_edges(area: LogicalRect, free: Rectangle<i32, Logical>) -> Insets {
    let (left, right) = margins(area.width(), free.loc.x, free.size.w);
    let (top, bottom) = margins(area.height(), free.loc.y, free.size.h);
    Insets {
        top,
        right,
        bottom,
        left,
    }
}

fn margins(length: u32, free_start: i32, free_length: i32) -> (u32, u32) {
    let before = u32::try_from(free_start).unwrap_or(0);
    let free_end = before.saturating_add(u32::try_from(free_length).unwrap_or(0));
    (before, length.saturating_sub(free_end))
}

#[cfg(test)]
mod tests {
    use smithay::utils::{Point, Size};

    use super::*;

    fn area(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
        LogicalRect::new(x, y, width, height).unwrap()
    }

    fn free(x: i32, y: i32, width: i32, height: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((width, height)))
    }

    #[test]
    fn a_zone_as_large_as_the_output_reserves_nothing() {
        let edges = reserved_edges(area(1920, 0, 1280, 720), free(0, 0, 1280, 720));

        assert_eq!(edges, Insets::default());
    }

    #[test]
    fn panels_on_opposite_edges_reserve_both_margins() {
        let edges = reserved_edges(area(0, 0, 1280, 720), free(48, 30, 1232, 650));

        assert_eq!(
            edges,
            Insets {
                top: 30,
                right: 0,
                bottom: 40,
                left: 48,
            }
        );
    }

    #[test]
    fn a_zone_beyond_the_output_never_yields_negative_margins() {
        let edges = reserved_edges(area(0, 0, 800, 600), free(-10, -10, 2000, 2000));

        assert_eq!(edges, Insets::default());
    }
}
