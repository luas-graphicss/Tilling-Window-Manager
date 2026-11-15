use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};
use smithay::input::pointer::CursorImageStatus;
use smithay::output::Output;
use smithay::utils::{Scale, Transform};

use super::elements::SceneElement;
use crate::runtime::Compositor;

const WIDTH: i32 = 12;
const HEIGHT: i32 = 17;
const UNSCALED: i32 = 1;
const OUTLINE: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
const FILL: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
const TRANSPARENT: [u8; 4] = [0x00; 4];
const SHAPE: [&str; 17] = [
    "#           ",
    "##          ",
    "#.#         ",
    "#..#        ",
    "#...#       ",
    "#....#      ",
    "#.....#     ",
    "#......#    ",
    "#.......#   ",
    "#........#  ",
    "#.........# ",
    "#......#####",
    "#...#..#    ",
    "#..# #..#   ",
    "#.#  #..#   ",
    "##    #..#  ",
    "#      ##   ",
];

pub(crate) struct PointerSprite {
    image: MemoryRenderBuffer,
}

impl PointerSprite {
    pub(crate) fn new() -> Self {
        Self {
            image: MemoryRenderBuffer::from_slice(
                &pixels(),
                Fourcc::Argb8888,
                (WIDTH, HEIGHT),
                UNSCALED,
                Transform::Normal,
                None,
            ),
        }
    }

    pub(crate) fn elements<R>(
        &self,
        compositor: &Compositor,
        renderer: &mut R,
        output: &Output,
    ) -> Vec<SceneElement<R>>
    where
        R: Renderer + ImportAll + ImportMem,
        R::TextureId: Send + Clone + 'static,
    {
        if !matches!(compositor.cursor, CursorImageStatus::Named(_)) {
            return Vec::new();
        }
        let Some(pointer) = compositor.seat.get_pointer() else {
            return Vec::new();
        };
        let scale = Scale::from(output.current_scale().fractional_scale());
        let tip = pointer.current_location() - output.current_location().to_f64();
        let drawn = MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            tip.to_physical(scale),
            &self.image,
            None,
            None,
            None,
            Kind::Cursor,
        );
        match drawn {
            Ok(element) => vec![SceneElement::Sprite(element)],
            Err(error) => {
                tracing::warn!(%error, "upload the pointer image");
                Vec::new()
            }
        }
    }
}

fn pixels() -> Vec<u8> {
    SHAPE
        .iter()
        .flat_map(|row| row.bytes())
        .flat_map(|cell| match cell {
            b'#' => OUTLINE,
            b'.' => FILL,
            _ => TRANSPARENT,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_fills_the_declared_buffer_size() {
        let rows = i32::try_from(SHAPE.len()).unwrap();
        let widths: Vec<i32> = SHAPE
            .iter()
            .map(|row| i32::try_from(row.len()).unwrap())
            .collect();
        let bytes = usize::try_from(WIDTH * HEIGHT).unwrap() * OUTLINE.len();

        assert_eq!(rows, HEIGHT);
        assert!(widths.iter().all(|width| *width == WIDTH));
        assert_eq!(pixels().len(), bytes);
    }

    #[test]
    fn tip_is_opaque_and_the_far_corner_is_transparent() {
        let image = pixels();

        assert_eq!(image.first_chunk::<4>(), Some(&OUTLINE));
        assert_eq!(image.last_chunk::<4>(), Some(&TRANSPARENT));
    }
}
