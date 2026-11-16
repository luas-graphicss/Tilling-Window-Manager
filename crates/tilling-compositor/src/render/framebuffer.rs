use smithay::output::{Output, OutputModeSource};
#[cfg(feature = "nested")]
use smithay::utils::Transform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FramebufferOrigin {
    TopLeft,
    #[cfg(feature = "nested")]
    BottomLeft,
}

impl FramebufferOrigin {
    pub(crate) fn mode_source(self, output: &Output) -> OutputModeSource {
        match self {
            Self::TopLeft => OutputModeSource::Auto(output.clone()),
            #[cfg(feature = "nested")]
            Self::BottomLeft => bottom_up_mode(output),
        }
    }
}

#[cfg(feature = "nested")]
fn bottom_up_mode(output: &Output) -> OutputModeSource {
    let Some(mode) = output.current_mode() else {
        return OutputModeSource::Auto(output.clone());
    };
    OutputModeSource::Static {
        size: mode.size,
        scale: output.current_scale().fractional_scale().into(),
        transform: painted_bottom_up(output.current_transform()),
    }
}

#[cfg(feature = "nested")]
const fn painted_bottom_up(transform: Transform) -> Transform {
    match transform {
        Transform::Normal => Transform::Flipped180,
        Transform::_90 => Transform::Flipped90,
        Transform::_180 => Transform::Flipped,
        Transform::_270 => Transform::Flipped270,
        Transform::Flipped => Transform::_180,
        Transform::Flipped90 => Transform::_90,
        Transform::Flipped180 => Transform::Normal,
        Transform::Flipped270 => Transform::_270,
    }
}

#[cfg(all(test, feature = "nested"))]
mod tests {
    use super::*;

    const TRANSFORMS: [Transform; 8] = [
        Transform::Normal,
        Transform::_90,
        Transform::_180,
        Transform::_270,
        Transform::Flipped,
        Transform::Flipped90,
        Transform::Flipped180,
        Transform::Flipped270,
    ];

    #[test]
    fn bottom_up_painting_cancels_the_flip_the_renderer_adds() {
        let renderer_flip = Transform::Flipped180.matrix();

        for transform in TRANSFORMS {
            let painted = painted_bottom_up(transform).matrix();

            assert_eq!(renderer_flip * painted, transform.matrix(), "{transform:?}");
        }
    }

    #[test]
    fn upright_output_is_painted_the_way_smithay_paints_a_host_window() {
        assert_eq!(painted_bottom_up(Transform::Normal), Transform::Flipped180);
    }
}
