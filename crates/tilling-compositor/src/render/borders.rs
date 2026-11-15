use std::collections::BTreeMap;

use smithay::backend::renderer::Color32F;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::utils::{Logical, Point, Rectangle, Scale};
use tilling_core::WindowId;

use crate::geometry::rectangle;
use crate::scene::{Scene, SceneWindow};

const FOCUSED: Color32F = Color32F::new(0.36, 0.62, 0.95, 1.0);
const UNFOCUSED: Color32F = Color32F::new(0.24, 0.26, 0.30, 1.0);
const EDGES: usize = 4;

type Edge = Rectangle<i32, Logical>;

#[derive(Debug, Default)]
pub(super) struct BorderCache {
    frames: BTreeMap<WindowId, [SolidColorBuffer; EDGES]>,
}

impl BorderCache {
    pub(super) fn elements(
        &mut self,
        placed: &SceneWindow,
        output_origin: Point<i32, Logical>,
        scale: Scale<f64>,
    ) -> Vec<SolidColorRenderElement> {
        let color = if placed.focused { FOCUSED } else { UNFOCUSED };
        let buffers = self
            .frames
            .entry(placed.id)
            .or_insert_with(|| std::array::from_fn(|_| SolidColorBuffer::new((0, 0), color)));
        edges(rectangle(placed.frame), rectangle(placed.content))
            .into_iter()
            .zip(buffers)
            .filter(|(edge, _)| !edge.is_empty())
            .map(|(edge, buffer)| {
                buffer.update(edge.size, color);
                let location = (edge.loc - output_origin).to_physical_precise_round(scale);
                SolidColorRenderElement::from_buffer(
                    buffer,
                    location,
                    scale,
                    1.0,
                    Kind::Unspecified,
                )
            })
            .collect()
    }

    pub(super) fn forget_windows_outside(&mut self, scene: &Scene) {
        self.frames
            .retain(|window, _| scene.window(*window).is_some());
    }
}

fn edges(frame: Edge, content: Edge) -> [Edge; EDGES] {
    let frame_end = frame.loc + frame.size;
    let content_end = content.loc + content.size;
    let top = Edge::new(
        frame.loc,
        (frame.size.w, content.loc.y - frame.loc.y).into(),
    );
    let bottom = Edge::new(
        (frame.loc.x, content_end.y).into(),
        (frame.size.w, frame_end.y - content_end.y).into(),
    );
    let left = Edge::new(
        (frame.loc.x, content.loc.y).into(),
        (content.loc.x - frame.loc.x, content.size.h).into(),
    );
    let right = Edge::new(
        (content_end.x, content.loc.y).into(),
        (frame_end.x - content_end.x, content.size.h).into(),
    );
    [top, bottom, left, right]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: i32, y: i32, width: i32, height: i32) -> Edge {
        Edge::new((x, y).into(), (width, height).into())
    }

    fn covered(edges: &[Edge]) -> i32 {
        edges.iter().map(|edge| edge.size.w * edge.size.h).sum()
    }

    #[test]
    fn edges_surround_the_content_without_overlapping() {
        let frame = area(8, 8, 400, 300);
        let content = area(10, 10, 396, 296);

        let drawn = edges(frame, content);

        assert_eq!(
            drawn,
            [
                area(8, 8, 400, 2),
                area(8, 306, 400, 2),
                area(8, 10, 2, 296),
                area(406, 10, 2, 296)
            ]
        );
        assert_eq!(covered(&drawn), 400 * 300 - 396 * 296);
        assert!(drawn.iter().all(|edge| !edge.overlaps(content)));
    }

    #[test]
    fn frame_without_a_border_has_only_empty_edges() {
        let fullscreen = area(0, 0, 1280, 720);

        let drawn = edges(fullscreen, fullscreen);

        assert!(drawn.iter().all(Edge::is_empty));
    }
}
