mod rect;
mod scale;
mod split;

pub use rect::{Logical, LogicalRect, Physical, PhysicalRect, Rect};
pub use scale::Scale;
pub use split::{Orientation, Ratio};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("rectangle edges exceed the coordinate space")]
    OutOfBounds,
    #[error(
        "scale must be a finite factor between {} and {}",
        Scale::MIN_FACTOR,
        Scale::MAX_FACTOR
    )]
    InvalidScale,
    #[error(
        "split ratio must be between {} and {} basis points",
        Ratio::MIN_BASIS_POINTS,
        Ratio::MAX_BASIS_POINTS
    )]
    InvalidRatio,
}
