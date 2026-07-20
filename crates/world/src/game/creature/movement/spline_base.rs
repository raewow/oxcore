//! Spline geometry primitives.
//!
//! Evaluation, derivatives and segment lengths for the three curve types the client
//! understands. This is pure geometry: no world, no timing, no packets - those live in
//! [`super::spline`].

use std::fmt;

/// Steps used to approximate the arc length of a curved segment.
const STEPS_PER_SEGMENT: u32 = 3;

/// Basis matrix rows for Catmull-Rom, applied to (t³, t², t, 1).
const CATMULL_ROM_COEFFS: [[f32; 4]; 4] = [
    [-0.5, 1.5, -1.5, 0.5],
    [1.0, -2.5, 2.0, -0.5],
    [-0.5, 0.0, 0.5, 0.0],
    [0.0, 1.0, 0.0, 0.0],
];

/// Basis matrix rows for a cubic Bezier, applied to (t³, t², t, 1).
const BEZIER3_COEFFS: [[f32; 4]; 4] = [
    [-1.0, 3.0, -3.0, 1.0],
    [3.0, -6.0, 3.0, 0.0],
    [-3.0, 3.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
];

/// A point or direction in world space.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Interpolate towards `other`. `t` outside 0..1 extrapolates, which is how the
    /// virtual control points at the ends of a Catmull-Rom spline are built.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    fn scale(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor, self.z * factor)
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

/// How the control points are interpreted between indices.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMode {
    Linear,
    CatmullRom,
    Bezier3,
    #[default]
    Uninitialized,
}

impl fmt::Display for EvaluationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Linear => "Linear",
            Self::CatmullRom => "CatmullRom",
            Self::Bezier3 => "Bezier3",
            Self::Uninitialized => "Uninitialized",
        };
        f.write_str(name)
    }
}

/// Weighted sum of four control points against a basis matrix.
///
/// `weights[j]` is the dot product of (t³, t², t, 1) with column `j`, which is the
/// row-vector-times-matrix product the C++ core performs.
fn evaluate_basis(vertices: &[Vec3], t: f32, coeffs: &[[f32; 4]; 4]) -> Vec3 {
    let tvec = [t * t * t, t * t, t, 1.0];
    combine(vertices, &tvec, coeffs)
}

/// Same as [`evaluate_basis`] but against the derivative of the parameter vector.
fn evaluate_basis_derivative(vertices: &[Vec3], t: f32, coeffs: &[[f32; 4]; 4]) -> Vec3 {
    let tvec = [3.0 * t * t, 2.0 * t, 1.0, 0.0];
    combine(vertices, &tvec, coeffs)
}

fn combine(vertices: &[Vec3], tvec: &[f32; 4], coeffs: &[[f32; 4]; 4]) -> Vec3 {
    let mut result = Vec3::default();

    for (column, vertex) in vertices.iter().enumerate().take(4) {
        let weight = (0..4).map(|row| tvec[row] * coeffs[row][column]).sum::<f32>();
        result = result.add(vertex.scale(weight));
    }

    result
}

/// A sequence of control points plus the virtual points the basis functions need.
#[derive(Debug, Default, Clone)]
pub struct SplineBase {
    points: Vec<Vec3>,
    index_lo: usize,
    index_hi: usize,
    mode: EvaluationMode,
    cyclic: bool,
}

impl SplineBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> EvaluationMode {
        self.mode
    }

    pub fn is_cyclic(&self) -> bool {
        self.cyclic
    }

    pub fn points(&self) -> &[Vec3] {
        &self.points
    }

    pub fn index_lo(&self) -> usize {
        self.index_lo
    }

    pub fn index_hi(&self) -> usize {
        self.index_hi
    }

    /// Whether `index` names a segment that can be evaluated.
    fn is_valid_segment(&self, index: usize) -> bool {
        index >= self.index_lo && index < self.index_hi
    }

    /// Position at parameter `u` within segment `index`, or `None` if out of range.
    ///
    /// The C++ core asserts on a bad index; returning `None` keeps a malformed path from
    /// taking the server down.
    pub fn evaluate(&self, index: usize, u: f32) -> Option<Vec3> {
        match self.mode {
            EvaluationMode::Linear => self.evaluate_linear(index, u),
            EvaluationMode::CatmullRom => self.evaluate_catmull_rom(index, u),
            EvaluationMode::Bezier3 => self.evaluate_bezier3(index, u),
            EvaluationMode::Uninitialized => None,
        }
    }

    /// Tangent at parameter `u` within segment `index`.
    pub fn evaluate_derivative(&self, index: usize, u: f32) -> Option<Vec3> {
        match self.mode {
            EvaluationMode::Linear => self.evaluate_derivative_linear(index),
            EvaluationMode::CatmullRom => self.evaluate_derivative_catmull_rom(index, u),
            EvaluationMode::Bezier3 => self.evaluate_derivative_bezier3(index, u),
            EvaluationMode::Uninitialized => None,
        }
    }

    /// Arc length of segment `index`.
    pub fn seg_length(&self, index: usize) -> Option<f32> {
        match self.mode {
            EvaluationMode::Linear => self.seg_length_linear(index),
            EvaluationMode::CatmullRom => self.seg_length_catmull_rom(index),
            EvaluationMode::Bezier3 => self.seg_length_bezier3(index),
            EvaluationMode::Uninitialized => None,
        }
    }

    fn evaluate_linear(&self, index: usize, u: f32) -> Option<Vec3> {
        if !self.is_valid_segment(index) {
            return None;
        }

        let start = self.points[index];
        Some(start.add(self.points[index + 1].sub(start).scale(u)))
    }

    fn evaluate_catmull_rom(&self, index: usize, t: f32) -> Option<Vec3> {
        if !self.is_valid_segment(index) || index < 1 {
            return None;
        }

        Some(evaluate_basis(
            self.points.get(index - 1..index + 3)?,
            t,
            &CATMULL_ROM_COEFFS,
        ))
    }

    fn evaluate_bezier3(&self, index: usize, t: f32) -> Option<Vec3> {
        let index = index * 3;
        if !self.is_valid_segment(index) {
            return None;
        }

        Some(evaluate_basis(
            self.points.get(index..index + 4)?,
            t,
            &BEZIER3_COEFFS,
        ))
    }

    fn evaluate_derivative_linear(&self, index: usize) -> Option<Vec3> {
        if !self.is_valid_segment(index) {
            return None;
        }

        Some(self.points[index + 1].sub(self.points[index]))
    }

    fn evaluate_derivative_catmull_rom(&self, index: usize, t: f32) -> Option<Vec3> {
        if !self.is_valid_segment(index) || index < 1 {
            return None;
        }

        Some(evaluate_basis_derivative(
            self.points.get(index - 1..index + 3)?,
            t,
            &CATMULL_ROM_COEFFS,
        ))
    }

    fn evaluate_derivative_bezier3(&self, index: usize, t: f32) -> Option<Vec3> {
        let index = index * 3;
        if !self.is_valid_segment(index) {
            return None;
        }

        Some(evaluate_basis_derivative(
            self.points.get(index..index + 4)?,
            t,
            &BEZIER3_COEFFS,
        ))
    }

    fn seg_length_linear(&self, index: usize) -> Option<f32> {
        if !self.is_valid_segment(index) {
            return None;
        }

        Some(self.points[index].sub(self.points[index + 1]).length())
    }

    /// Walk the curve in fixed steps and sum the chord lengths.
    fn stepped_length(vertices: &[Vec3], coeffs: &[[f32; 4]; 4], start_at_zero: bool) -> f32 {
        let mut current = if start_at_zero {
            evaluate_basis(vertices, 0.0, coeffs)
        } else {
            vertices[1]
        };

        let mut length = 0.0f64;
        for step in 1..=STEPS_PER_SEGMENT {
            let next = evaluate_basis(vertices, step as f32 / STEPS_PER_SEGMENT as f32, coeffs);
            length += next.sub(current).length() as f64;
            current = next;
        }

        length as f32
    }

    fn seg_length_catmull_rom(&self, index: usize) -> Option<f32> {
        if !self.is_valid_segment(index) || index < 1 {
            return None;
        }

        // The walk starts from the segment's first real control point, not from an
        // evaluation at t = 0.
        Some(Self::stepped_length(
            self.points.get(index - 1..index + 3)?,
            &CATMULL_ROM_COEFFS,
            false,
        ))
    }

    fn seg_length_bezier3(&self, index: usize) -> Option<f32> {
        let index = index * 3;
        if !self.is_valid_segment(index) {
            return None;
        }

        Some(Self::stepped_length(
            self.points.get(index..index + 4)?,
            &BEZIER3_COEFFS,
            true,
        ))
    }

    /// Build a one-shot spline through `controls`.
    pub fn init_spline(&mut self, controls: &[Vec3], mode: EvaluationMode) {
        self.mode = mode;
        self.cyclic = false;
        self.init_points(controls, 0);
    }

    /// Build a looping spline that returns to `cyclic_point` after the last control.
    pub fn init_cyclic_spline(
        &mut self,
        controls: &[Vec3],
        mode: EvaluationMode,
        cyclic_point: usize,
    ) {
        self.mode = mode;
        self.cyclic = true;
        self.init_points(controls, cyclic_point);
    }

    fn init_points(&mut self, controls: &[Vec3], cyclic_point: usize) {
        match self.mode {
            EvaluationMode::Linear => self.init_linear(controls, cyclic_point),
            EvaluationMode::CatmullRom => self.init_catmull_rom(controls, cyclic_point),
            EvaluationMode::Bezier3 => self.init_bezier3(controls),
            EvaluationMode::Uninitialized => self.clear(),
        }
    }

    fn init_linear(&mut self, controls: &[Vec3], cyclic_point: usize) {
        let count = controls.len();
        if count < 2 {
            self.clear();
            return;
        }

        self.points.clear();
        self.points.extend_from_slice(controls);
        // One trailing point closes the loop, or repeats the end for a one-shot path.
        self.points.push(if self.cyclic {
            controls[cyclic_point.min(count - 1)]
        } else {
            controls[count - 1]
        });

        self.index_lo = 0;
        self.index_hi = if self.cyclic { count } else { count - 1 };
    }

    fn init_catmull_rom(&mut self, controls: &[Vec3], cyclic_point: usize) {
        let count = controls.len();
        if count < 2 {
            self.clear();
            return;
        }

        let real_size = count + if self.cyclic { 3 } else { 2 };
        self.points = vec![Vec3::default(); real_size];

        let lo_index = 1usize;
        let high_index = lo_index + count - 1;
        self.points[lo_index..lo_index + count].copy_from_slice(controls);

        // The leading and trailing virtual points exist so the basis functions always
        // have four controls to work with.
        if self.cyclic {
            self.points[0] = if cyclic_point == 0 {
                controls[count - 1]
            } else {
                controls[0].lerp(controls[1], -1.0)
            };
            self.points[high_index + 1] = controls[cyclic_point.min(count - 1)];
            self.points[high_index + 2] = controls[(cyclic_point + 1).min(count - 1)];
        } else {
            self.points[0] = controls[0].lerp(controls[1], -1.0);
            self.points[high_index + 1] = controls[count - 1];
        }

        self.index_lo = lo_index;
        self.index_hi = high_index + if self.cyclic { 1 } else { 0 };
    }

    fn init_bezier3(&mut self, controls: &[Vec3]) {
        // Bezier controls come in threes; a trailing partial group is dropped.
        let usable = controls.len() / 3 * 3;
        let segments = usable / 3;

        self.points.clear();
        self.points.extend_from_slice(&controls[..usable]);

        self.index_lo = 0;
        self.index_hi = segments.saturating_sub(1);
    }

    pub fn clear(&mut self) {
        self.index_lo = 0;
        self.index_hi = 0;
        self.points.clear();
    }
}

impl fmt::Display for SplineBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "mode: {}", self.mode)?;
        writeln!(f, "points count: {}", self.points.len())?;
        for (index, point) in self.points.iter().enumerate() {
            writeln!(f, "point {index} : ({}, {}, {})", point.x, point.y, point.z)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn assert_close(actual: Vec3, expected: Vec3) {
        let tolerance = 1e-4;
        assert!(
            (actual.x - expected.x).abs() < tolerance
                && (actual.y - expected.y).abs() < tolerance
                && (actual.z - expected.z).abs() < tolerance,
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn line() -> SplineBase {
        let mut spline = SplineBase::new();
        spline.init_spline(
            &[v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(10.0, 10.0, 0.0)],
            EvaluationMode::Linear,
        );
        spline
    }

    #[test]
    fn linear_init_appends_a_closing_point_and_sets_the_segment_range() {
        let spline = line();

        // 3 controls plus the repeated end point.
        assert_eq!(spline.points().len(), 4);
        assert_eq!(spline.points()[3], v(10.0, 10.0, 0.0));
        assert_eq!(spline.index_lo(), 0);
        assert_eq!(spline.index_hi(), 2);
        assert!(!spline.is_cyclic());
    }

    #[test]
    fn cyclic_linear_init_closes_the_loop_back_to_the_cyclic_point() {
        let mut spline = SplineBase::new();
        spline.init_cyclic_spline(
            &[v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(10.0, 10.0, 0.0)],
            EvaluationMode::Linear,
            0,
        );

        assert_eq!(spline.points()[3], v(0.0, 0.0, 0.0));
        // The extra segment back to the start is evaluable.
        assert_eq!(spline.index_hi(), 3);
        assert!(spline.is_cyclic());
    }

    #[test]
    fn linear_evaluation_interpolates_along_the_segment() {
        let spline = line();

        assert_close(spline.evaluate(0, 0.0).unwrap(), v(0.0, 0.0, 0.0));
        assert_close(spline.evaluate(0, 0.5).unwrap(), v(5.0, 0.0, 0.0));
        assert_close(spline.evaluate(1, 0.25).unwrap(), v(10.0, 2.5, 0.0));
    }

    #[test]
    fn linear_derivative_is_the_segment_vector_and_length_its_magnitude() {
        let spline = line();

        assert_close(spline.evaluate_derivative(0, 0.7).unwrap(), v(10.0, 0.0, 0.0));
        assert_eq!(spline.seg_length(0).unwrap(), 10.0);
        assert_eq!(spline.seg_length(1).unwrap(), 10.0);
    }

    #[test]
    fn out_of_range_segments_return_none_instead_of_panicking() {
        let spline = line();

        assert!(spline.evaluate(2, 0.5).is_none());
        assert!(spline.evaluate(99, 0.5).is_none());
        assert!(spline.evaluate_derivative(2, 0.5).is_none());
        assert!(spline.seg_length(2).is_none());

        // An uninitialized spline evaluates to nothing at all.
        let empty = SplineBase::new();
        assert!(empty.evaluate(0, 0.0).is_none());
        assert!(empty.seg_length(0).is_none());
    }

    #[test]
    fn catmull_rom_init_builds_the_mirrored_virtual_endpoints() {
        let mut spline = SplineBase::new();
        spline.init_spline(
            &[v(0.0, 0.0, 0.0), v(10.0, 0.0, 0.0), v(20.0, 0.0, 0.0)],
            EvaluationMode::CatmullRom,
        );

        // Leading virtual point mirrors control 0 across control 1: 2*c0 - c1.
        assert_close(spline.points()[0], v(-10.0, 0.0, 0.0));
        // Trailing virtual point repeats the final control.
        assert_close(*spline.points().last().unwrap(), v(20.0, 0.0, 0.0));
        assert_eq!(spline.index_lo(), 1);
        assert_eq!(spline.index_hi(), 3);
    }

    #[test]
    fn catmull_rom_passes_through_its_control_points() {
        let mut spline = SplineBase::new();
        let controls = [
            v(0.0, 0.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(20.0, 10.0, 0.0),
            v(30.0, 10.0, 0.0),
        ];
        spline.init_spline(&controls, EvaluationMode::CatmullRom);

        // t=0 and t=1 of a segment land exactly on its bounding controls.
        assert_close(spline.evaluate(1, 0.0).unwrap(), controls[0]);
        assert_close(spline.evaluate(1, 1.0).unwrap(), controls[1]);
        assert_close(spline.evaluate(2, 0.0).unwrap(), controls[1]);
        assert_close(spline.evaluate(2, 1.0).unwrap(), controls[2]);
    }

    #[test]
    fn catmull_rom_segment_length_is_at_least_the_straight_line_distance() {
        let mut spline = SplineBase::new();
        spline.init_spline(
            &[
                v(0.0, 0.0, 0.0),
                v(10.0, 0.0, 0.0),
                v(20.0, 10.0, 0.0),
                v(30.0, 10.0, 0.0),
            ],
            EvaluationMode::CatmullRom,
        );

        let curved = spline.seg_length(2).unwrap();
        let straight = v(20.0, 10.0, 0.0).sub(v(10.0, 0.0, 0.0)).length();
        assert!(
            curved >= straight * 0.99,
            "curve {curved} shorter than chord {straight}"
        );
    }

    #[test]
    fn bezier3_evaluates_the_standard_cubic_basis() {
        let mut spline = SplineBase::new();
        let controls = [
            v(0.0, 0.0, 0.0),
            v(0.0, 10.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(10.0, 0.0, 0.0),
            // A trailing partial group is dropped by init.
            v(99.0, 99.0, 99.0),
        ];
        spline.init_spline(&controls, EvaluationMode::Bezier3);

        // 5 controls -> only the first 3 are usable (5 / 3 * 3).
        assert_eq!(spline.points().len(), 3);
        assert!(spline.evaluate(0, 0.5).is_none());

        // With a full group of 6, two segments exist.
        let controls = [
            v(0.0, 0.0, 0.0),
            v(0.0, 10.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(20.0, 0.0, 0.0),
            v(20.0, 10.0, 0.0),
        ];
        spline.init_spline(&controls, EvaluationMode::Bezier3);
        assert_eq!(spline.points().len(), 6);

        // Endpoints of a cubic Bezier are its first and fourth control points.
        assert_close(spline.evaluate(0, 0.0).unwrap(), controls[0]);
        assert_close(spline.evaluate(0, 1.0).unwrap(), controls[3]);
        // Midpoint of this symmetric curve.
        assert_close(spline.evaluate(0, 0.5).unwrap(), v(5.0, 7.5, 0.0));
    }

    #[test]
    fn clear_resets_points_and_segment_range() {
        let mut spline = line();
        spline.clear();

        assert!(spline.points().is_empty());
        assert_eq!(spline.index_lo(), 0);
        assert_eq!(spline.index_hi(), 0);
        assert!(spline.evaluate(0, 0.0).is_none());
    }

    #[test]
    fn display_lists_the_mode_and_every_point() {
        let spline = line();
        let text = spline.to_string();

        assert!(text.starts_with("mode: Linear\npoints count: 4\n"));
        assert!(text.contains("point 0 : (0, 0, 0)"));
        assert!(text.contains("point 3 : (10, 10, 0)"));
        assert_eq!(EvaluationMode::CatmullRom.to_string(), "CatmullRom");
    }
}
