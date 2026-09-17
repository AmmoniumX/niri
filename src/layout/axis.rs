use niri_config::MainAxis;
use smithay::utils::{Coordinate, Logical, Point, Rectangle, Size};

use crate::utils::ResizeEdge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisDirection {
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisEdge {
    Start,
    End,
}

/// Maps physical coordinates to scrolling coordinates (X = main, Y = cross), and back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AxisMap {
    main_axis: MainAxis,
}

impl AxisMap {
    pub const fn new(main_axis: MainAxis) -> Self {
        Self { main_axis }
    }

    pub const fn main_axis(self) -> MainAxis {
        self.main_axis
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self.main_axis, MainAxis::Vertical)
    }

    pub fn point_in<N: Coordinate>(self, point: Point<N, Logical>) -> Point<N, Logical> {
        if self.is_vertical() {
            Point::from((point.y, point.x))
        } else {
            point
        }
    }

    pub fn point_out<N: Coordinate>(self, point: Point<N, Logical>) -> Point<N, Logical> {
        self.point_in(point)
    }

    pub fn size_in<N: Coordinate>(self, size: Size<N, Logical>) -> Size<N, Logical> {
        if self.is_vertical() {
            Size::from((size.h, size.w))
        } else {
            size
        }
    }

    pub fn size_out<N: Coordinate>(self, size: Size<N, Logical>) -> Size<N, Logical> {
        self.size_in(size)
    }

    pub fn point_main<N: Copy>(self, point: Point<N, Logical>) -> N {
        if self.is_vertical() {
            point.y
        } else {
            point.x
        }
    }

    pub fn point_cross<N: Copy>(self, point: Point<N, Logical>) -> N {
        if self.is_vertical() {
            point.x
        } else {
            point.y
        }
    }

    pub fn map_main<T, U>(
        self,
        target: &mut T,
        physical_horizontal: impl FnOnce(&mut T) -> U,
        physical_vertical: impl FnOnce(&mut T) -> U,
    ) -> U {
        if self.is_vertical() {
            physical_vertical(target)
        } else {
            physical_horizontal(target)
        }
    }

    pub fn map_cross<T, U>(
        self,
        target: &mut T,
        physical_horizontal: impl FnOnce(&mut T) -> U,
        physical_vertical: impl FnOnce(&mut T) -> U,
    ) -> U {
        if self.is_vertical() {
            physical_horizontal(target)
        } else {
            physical_vertical(target)
        }
    }

    pub fn size_main<N: Copy>(self, size: Size<N, Logical>) -> N {
        if self.is_vertical() {
            size.h
        } else {
            size.w
        }
    }

    pub fn size_cross<N: Copy>(self, size: Size<N, Logical>) -> N {
        if self.is_vertical() {
            size.w
        } else {
            size.h
        }
    }

    pub fn rect_in<N: Coordinate>(self, rect: Rectangle<N, Logical>) -> Rectangle<N, Logical> {
        Rectangle::new(self.point_in(rect.loc), self.size_in(rect.size))
    }

    pub fn rect_out<N: Coordinate>(self, rect: Rectangle<N, Logical>) -> Rectangle<N, Logical> {
        self.rect_in(rect)
    }

    pub fn point_from_main_cross<N: Coordinate>(self, main: N, cross: N) -> Point<N, Logical> {
        self.point_out(Point::from((main, cross)))
    }

    pub fn size_from_main_cross<N: Coordinate>(self, main: N, cross: N) -> Size<N, Logical> {
        self.size_out(Size::from((main, cross)))
    }

    pub fn main_vec<N: Coordinate>(self, main: N) -> Point<N, Logical> {
        self.point_from_main_cross(main, N::from_f64(0.))
    }

    pub fn cross_vec<N: Coordinate>(self, cross: N) -> Point<N, Logical> {
        self.point_from_main_cross(N::from_f64(0.), cross)
    }

    pub fn resize_edges_in(self, edges: ResizeEdge) -> ResizeEdge {
        if !self.is_vertical() {
            return edges;
        }

        let mut mapped = ResizeEdge::empty();
        if edges.contains(ResizeEdge::LEFT) {
            mapped |= ResizeEdge::TOP;
        }
        if edges.contains(ResizeEdge::RIGHT) {
            mapped |= ResizeEdge::BOTTOM;
        }
        if edges.contains(ResizeEdge::TOP) {
            mapped |= ResizeEdge::LEFT;
        }
        if edges.contains(ResizeEdge::BOTTOM) {
            mapped |= ResizeEdge::RIGHT;
        }
        mapped
    }

    pub fn resize_edges_out(self, edges: ResizeEdge) -> ResizeEdge {
        self.resize_edges_in(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_cross_accessors_respect_axis() {
        let horizontal = AxisMap::new(MainAxis::Horizontal);
        let vertical = AxisMap::new(MainAxis::Vertical);

        let point = Point::<f64, Logical>::from((3., 7.));
        let size = Size::<f64, Logical>::from((11., 13.));

        assert_eq!(horizontal.point_main(point), 3.);
        assert_eq!(horizontal.point_cross(point), 7.);
        assert_eq!(vertical.point_main(point), 7.);
        assert_eq!(vertical.point_cross(point), 3.);

        assert_eq!(horizontal.size_main(size), 11.);
        assert_eq!(horizontal.size_cross(size), 13.);
        assert_eq!(vertical.size_main(size), 13.);
        assert_eq!(vertical.size_cross(size), 11.);
    }

    #[test]
    fn main_cross_constructors_respect_axis() {
        let horizontal = AxisMap::new(MainAxis::Horizontal);
        let vertical = AxisMap::new(MainAxis::Vertical);

        assert_eq!(
            horizontal.point_from_main_cross(3., 7.),
            Point::<f64, Logical>::from((3., 7.))
        );
        assert_eq!(
            vertical.point_from_main_cross(3., 7.),
            Point::<f64, Logical>::from((7., 3.))
        );

        assert_eq!(
            horizontal.size_from_main_cross(11., 13.),
            Size::<f64, Logical>::from((11., 13.))
        );
        assert_eq!(
            vertical.size_from_main_cross(11., 13.),
            Size::<f64, Logical>::from((13., 11.))
        );
    }

    #[test]
    fn map_main_and_cross_select_expected_branch() {
        let horizontal = AxisMap::new(MainAxis::Horizontal);
        let vertical = AxisMap::new(MainAxis::Vertical);

        let mut value = 0;
        horizontal.map_main(&mut value, |v| *v = 1, |v| *v = 2);
        assert_eq!(value, 1);

        horizontal.map_cross(&mut value, |v| *v = 3, |v| *v = 4);
        assert_eq!(value, 4);

        vertical.map_main(&mut value, |v| *v = 5, |v| *v = 6);
        assert_eq!(value, 6);

        vertical.map_cross(&mut value, |v| *v = 7, |v| *v = 8);
        assert_eq!(value, 7);
    }
}
