#![allow(clippy::implicit_hasher)]

mod corefont;
mod stencil_map;
mod sys_update_world_bboxes;

pub use stencil_map::StencilMap;
pub use sys_update_world_bboxes::sys_update_world_bboxes;

use kurbo::{BezPath, Line, Point, Rect, TranslateScale, Vec2};

/// A path with precomputed bounds.
#[derive(Debug, Clone)]
pub struct Path {
    pub(crate) outline: BezPath,
    pub(crate) bounds: Rect,
    pub(crate) advance: f64,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub(crate) text: String,
    pub(crate) font_size: f64,
    pub(crate) width: f64,
}

#[derive(Debug, Clone)]
pub struct CombineStencil(pub Vec<Stencil>);

#[derive(Debug, Clone)]
pub enum Stencil {
    Path(Path),
    Text(Text),
    Combine(CombineStencil),
    TranslateScale(TranslateScale, bool, Box<Stencil>),
}

/// Normalized normal vector of line.
fn normal(line: Line) -> Vec2 {
    let n = tangent(line);
    Vec2::new(n.y, -n.x)
}

/// Normalized tangent vector of line.
fn tangent(line: Line) -> Vec2 {
    (line.p1 - line.p0).normalize()
}

const BEZIER_CIRCLE_FACTOR: f64 = 0.552_284_8;

fn escape(s: &str) -> String {
    // From https://doc.rust-lang.org/1.1.0/src/rustdoc/html/escape.rs.html#20
    let mut parts = vec![];

    // Because the internet is always right, turns out there's not that many
    // characters to escape: http://stackoverflow.com/questions/7381974
    let pile_o_bits = s;
    let mut last = 0;
    for (i, ch) in s.bytes().enumerate() {
        match ch as char {
            '<' | '>' | '&' | '\'' | '"' => {
                parts.push(&pile_o_bits[last..i]);
                let replace = match ch as char {
                    '>' => "&gt;",
                    '<' => "&lt;",
                    '&' => "&amp;",
                    '\'' => "&#39;",
                    '"' => "&quot;",
                    _ => unreachable!(),
                };
                parts.push(replace);
                last = i + 1;
            }
            _ => {}
        }
    }

    if last < s.len() {
        parts.push(&pile_o_bits[last..]);
    }

    parts.concat()
}

impl Stencil {
    pub fn padding(advance: f64) -> Stencil {
        Stencil::Path(Path {
            outline: BezPath::default(),
            bounds: Rect::new(0.0, 0.0, advance, 0.0),
            advance,
        })
    }

    pub fn text(text: &str, font_size: f64, width: f64) -> Stencil {
        Stencil::Text(Text {
            text: text.to_owned(),
            font_size,
            width,
        })
    }

    /// Draw a reasonable approximation of a circle.
    ///
    /// The radial error is about 0.0273%.
    pub fn circle(radius: f64, center: Point) -> Stencil {
        let rx = Vec2::new(radius, 0.0);
        let ry = Vec2::new(0.0, radius);
        let cx = rx * BEZIER_CIRCLE_FACTOR;
        let cy = ry * BEZIER_CIRCLE_FACTOR;

        let mut path = BezPath::new();
        path.move_to(center + rx);
        path.curve_to(center + rx + cy, center + ry + cx, center + ry);
        path.curve_to(center + ry - cx, center - rx + cy, center - rx);
        path.curve_to(center - rx - cy, center - ry - cx, center - ry);
        path.curve_to(center - ry + cx, center + rx - cy, center + rx);
        path.close_path();
        Stencil::Path(Path {
            bounds: Rect::new(
                center.x - radius,
                center.y - radius,
                center.x + radius,
                center.y + radius,
            ),
            outline: path,
            advance: center.x + radius,
        })
    }

    /// Draw a line with rounded edges.
    ///
    /// This stencil has a blot diameter equal to the thickness, which is included in the
    /// bounding box.
    pub fn line(line: Line, thickness: f64) -> Stencil {
        if line.p0 == line.p1 {
            return Self::circle(thickness / 2.0, line.p0);
        }

        let normal = normal(line);
        if normal.x.is_nan() || normal.y.is_nan() {
            return Stencil::default();
        }

        let tangent = tangent(line);

        let rx = tangent * (thickness / 2.0);
        let ry = normal * (thickness / 2.0);
        let cx = rx * BEZIER_CIRCLE_FACTOR;
        let cy = ry * BEZIER_CIRCLE_FACTOR;

        let mut path = BezPath::new();
        // Top
        path.move_to(line.p0 + ry);
        path.line_to(line.p1 + ry);

        // Right blot
        path.curve_to(line.p1 + ry + cx, line.p1 + rx + cy, line.p1 + rx);
        path.curve_to(line.p1 + rx - cy, line.p1 - ry + cx, line.p1 - ry);

        // Bottom
        path.line_to(line.p0 - ry);

        // Left blot
        path.curve_to(line.p0 - ry - cx, line.p0 - rx - cy, line.p0 - rx);
        path.curve_to(line.p0 - rx + cy, line.p0 + ry - cx, line.p0 + ry);

        // Done!
        path.close_path();
        Stencil::Path(Path {
            bounds: Rect::new(
                line.p0.x - thickness / 2.0,
                line.p0.y - thickness / 2.0,
                line.p1.x + thickness / 2.0,
                line.p1.y + thickness / 2.0,
            ),
            outline: path,
            advance: line.p1.x,
        })
    }

    /// Draw a rounded rectangle
    ///
    /// This stencil has user-specified blot. The total thickness includes blot.
    pub fn round_filled_box(rect: Rect, mut blot_diameter: f64) -> Stencil {
        let Rect { mut x0, mut y0, .. } = rect;

        blot_diameter = blot_diameter.min(rect.height()).min(rect.width());

        if blot_diameter < 0.0 {
            return Stencil::default();
        }

        x0 += blot_diameter / 2.0;
        y0 += blot_diameter / 2.0;

        let width = Vec2::new(rect.width() - blot_diameter, 0.0);
        let height = Vec2::new(0.0, rect.height() - blot_diameter);

        let rx = Vec2::new(blot_diameter / 2.0, 0.0);
        let ry = Vec2::new(0.0, blot_diameter / 2.0);
        let cx = rx * BEZIER_CIRCLE_FACTOR;
        let cy = ry * BEZIER_CIRCLE_FACTOR;

        let mut path = BezPath::new();
        let origin = Point::new(x0, y0);

        // Start at the bottom right.
        path.move_to(origin - ry);

        // Bottom-left blot
        path.curve_to(origin - ry - cx, origin - rx - cy, origin - rx);

        // Left
        path.line_to(origin + height - rx);

        // Top-left blot
        if blot_diameter > 0.0 {
            path.curve_to(
                origin + height - rx + cy,
                origin + height + ry - cx,
                origin + height + ry,
            );
        }

        // Top
        path.line_to(origin + height + width + ry);

        // Top-right blot
        if blot_diameter > 0.0 {
            path.curve_to(
                origin + height + width + ry + cx,
                origin + height + width + rx + cy,
                origin + height + width + rx,
            );
        }

        // Right
        path.line_to(origin + width + rx);

        // Bottom-right blot
        if blot_diameter > 0.0 {
            path.curve_to(
                origin + width + rx - cy,
                origin + width - ry + cx,
                origin + width - ry,
            );
        }

        // Bottom
        path.line_to(origin - ry);
        path.close_path();
        Stencil::Path(Path {
            bounds: rect,
            outline: path,
            advance: rect.x1,
        })
    }

    pub fn staff_line(width: f64) -> Stencil {
        Self::line(
            Line::new(
                Point::new(corefont::STAFF_LINE_THICKNESS / 2.0, 0.0),
                Point::new(width - corefont::STAFF_LINE_THICKNESS / 2.0, 0.0),
            ),
            corefont::STAFF_LINE_THICKNESS,
        )
    }

    /// Includes blot in height.
    pub fn stem_line(x: f64, mut y1: f64, mut y2: f64) -> Stencil {
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }

        let thickness = corefont::STEM_THICKNESS;
        Self::line(
            Line::new(
                Point::new(x, y1 + thickness / 2.0),
                Point::new(x, y2 - thickness / 2.0),
            ),
            thickness,
        )
    }

    pub fn barline_thick(x: f64, mut y1: f64, mut y2: f64) -> Stencil {
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }

        let thickness = corefont::THICK_BARLINE_THICKNESS;
        Self::round_filled_box(
            Rect::new(x - thickness / 2.0, y1, x + thickness / 2.0, y2),
            corefont::THIN_BARLINE_THICKNESS,
        )
    }

    pub fn barline_thin(x: f64, mut y1: f64, mut y2: f64) -> Stencil {
        if y1 > y2 {
            std::mem::swap(&mut y1, &mut y2);
        }

        let thickness = corefont::THIN_BARLINE_THICKNESS;
        Self::line(
            Line::new(
                Point::new(x, y1 + thickness / 2.0),
                Point::new(x, y2 - thickness / 2.0),
            ),
            thickness,
        )
    }

    /// Initialize a stencil, in staff cordinates.
    fn from_corefont(corefont: &(f64, [f64; 4], &str)) -> Stencil {
        assert_eq!(corefont::UNITS_PER_EM, 1000);
        Stencil::Path(Path {
            outline: BezPath::from_svg(corefont.2).expect("Invalid corefont"),
            bounds: Rect::new(corefont.1[0], corefont.1[1], corefont.1[2], corefont.1[3]),
            advance: corefont.0,
        })
    }

    fn attachment(corefont: [f64; 2]) -> Point {
        Point::new(corefont[0], corefont[1])
    }

    pub fn time_sig_number(mut number: u8) -> Stencil {
        let mut digits = Vec::with_capacity(3);
        while number > 0 {
            digits.push(match number % 10 {
                0 => Self::from_corefont(&corefont::TIME_SIG0),
                1 => Self::from_corefont(&corefont::TIME_SIG1),
                2 => Self::from_corefont(&corefont::TIME_SIG2),
                3 => Self::from_corefont(&corefont::TIME_SIG3),
                4 => Self::from_corefont(&corefont::TIME_SIG4),
                5 => Self::from_corefont(&corefont::TIME_SIG5),
                6 => Self::from_corefont(&corefont::TIME_SIG6),
                7 => Self::from_corefont(&corefont::TIME_SIG7),
                8 => Self::from_corefont(&corefont::TIME_SIG8),
                9 => Self::from_corefont(&corefont::TIME_SIG9),
                _ => unreachable!(),
            });
            number /= 10;
        }

        digits.reverse();

        let mut advance = 0.0;
        let mut stencils = Vec::with_capacity(digits.len());
        for digit in digits {
            let digit_advance = digit.advance();
            stencils.push(digit.with_translation(Vec2::new(advance, 0.0)));
            advance += digit_advance;
        }

        Self::combine(stencils)
    }

    pub fn time_sig_fraction(num: u8, den: u8) -> Stencil {
        let mut num = Self::time_sig_number(num);
        let mut den = Self::time_sig_number(den);

        let num_adv = num.advance();
        let den_adv = den.advance();

        if num_adv > den_adv {
            num = num.with_translation(Vec2::new(0.0, 247.0));
            den = den.with_translation(Vec2::new((num_adv - den_adv) / 2.0, -247.0));
        } else {
            num = num.with_translation(Vec2::new((den_adv - num_adv) / 2.0, 247.0));
            den = den.with_translation(Vec2::new(0.0, -247.0));
        }

        Stencil::combine(vec![num, den])
    }

    pub fn time_sig_common() -> Stencil {
        Self::from_corefont(&corefont::TIME_SIG_COMMON)
    }

    pub fn time_sig_cut() -> Stencil {
        Self::from_corefont(&corefont::TIME_SIG_CUT_COMMON)
    }

    pub fn time_sig_cancel() -> Stencil {
        Self::from_corefont(&corefont::TIME_SIG_X)
    }

    pub fn clef_g() -> Stencil {
        Self::from_corefont(&corefont::G_CLEF)
    }

    pub fn clef_c() -> Stencil {
        Self::from_corefont(&corefont::C_CLEF)
    }

    pub fn clef_f() -> Stencil {
        Self::from_corefont(&corefont::F_CLEF)
    }

    pub fn clef_unpitched() -> Stencil {
        Self::from_corefont(&corefont::UNPITCHED_PERCUSSION_CLEF1)
    }

    pub fn rest_maxima() -> Stencil {
        Self::from_corefont(&corefont::REST_MAXIMA)
    }

    pub fn rest_longa() -> Stencil {
        Self::from_corefont(&corefont::REST_LONGA)
    }

    pub fn rest_double_whole() -> Stencil {
        Self::from_corefont(&corefont::REST_DOUBLE_WHOLE)
    }

    pub fn rest_whole() -> Stencil {
        Self::from_corefont(&corefont::REST_WHOLE)
    }

    pub fn rest_half() -> Stencil {
        Self::from_corefont(&corefont::REST_HALF)
    }

    pub fn rest_quarter() -> Stencil {
        Self::from_corefont(&corefont::REST_QUARTER)
    }

    pub fn rest_8() -> Stencil {
        Self::from_corefont(&corefont::REST8TH)
    }

    pub fn rest_16() -> Stencil {
        Self::from_corefont(&corefont::REST16TH)
    }

    pub fn rest_32() -> Stencil {
        Self::from_corefont(&corefont::REST32ND)
    }

    pub fn rest_64() -> Stencil {
        Self::from_corefont(&corefont::REST64TH)
    }

    pub fn rest_128() -> Stencil {
        Self::from_corefont(&corefont::REST128TH)
    }

    pub fn rest_256() -> Stencil {
        Self::from_corefont(&corefont::REST256TH)
    }

    pub fn notehead_x_double_whole() -> (Stencil, Option<Point>) {
        (
            Self::from_corefont(&corefont::NOTEHEAD_X_DOUBLE_WHOLE),
            None,
        )
    }

    pub fn notehead_x_whole() -> (Stencil, Option<Point>) {
        (Self::from_corefont(&corefont::NOTEHEAD_X_WHOLE), None)
    }

    pub fn notehead_x_half_up() -> (Stencil, Option<Point>) {
        (
            Self::from_corefont(&corefont::NOTEHEAD_X_HALF),
            Some(Self::attachment(corefont::NOTEHEAD_X_HALF_STEM_UP)),
        )
    }

    pub fn notehead_x_half_down() -> (Stencil, Option<Point>) {
        (
            Self::from_corefont(&corefont::NOTEHEAD_X_HALF),
            Some(Self::attachment(corefont::NOTEHEAD_X_HALF_STEM_DOWN)),
        )
    }

    pub fn notehead_x_black_up() -> (Stencil, Option<Point>) {
        (
            Self::from_corefont(&corefont::NOTEHEAD_X_BLACK),
            Some(Self::attachment(corefont::NOTEHEAD_X_BLACK_STEM_UP)),
        )
    }

    pub fn notehead_x_black_stem_down_attachment() -> (Stencil, Option<Point>) {
        (
            Self::from_corefont(&corefont::NOTEHEAD_X_BLACK),
            Some(Self::attachment(corefont::NOTEHEAD_X_BLACK_STEM_DOWN)),
        )
    }

    pub fn flag_up_8() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG8TH_UP),
            Self::attachment(corefont::FLAG8TH_UP_STEM_UP),
        )
    }

    pub fn flag_up_16() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG16TH_UP),
            Self::attachment(corefont::FLAG16TH_UP_STEM_UP),
        )
    }

    pub fn flag_up_32() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG32ND_UP),
            Self::attachment(corefont::FLAG32ND_UP_STEM_UP),
        )
    }

    pub fn flag_up_64() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG64TH_UP),
            Self::attachment(corefont::FLAG64TH_UP_STEM_UP),
        )
    }

    pub fn flag_up_128() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG128TH_UP),
            Self::attachment(corefont::FLAG128TH_UP_STEM_UP),
        )
    }

    pub fn flag_up_256() -> (Stencil, Point) {
        (
            Self::from_corefont(&corefont::FLAG256TH_UP),
            Self::attachment(corefont::FLAG256TH_UP_STEM_UP),
        )
    }

    pub fn augmentation_dot() -> Stencil {
        Self::from_corefont(&corefont::AUGMENTATION_DOT)
    }

    pub fn combine(stencils: Vec<Stencil>) -> Stencil {
        Stencil::Combine(CombineStencil(stencils))
    }

    pub fn with_translation(self, offset: Vec2) -> Stencil {
        Stencil::TranslateScale(TranslateScale::translate(offset), false, Box::new(self))
    }

    pub fn with_translation_and_flip(self, offset: Vec2) -> Stencil {
        Stencil::TranslateScale(TranslateScale::translate(offset), true, Box::new(self))
    }

    pub fn with_scale(self, scale: f64) -> Stencil {
        Stencil::TranslateScale(TranslateScale::scale(scale), false, Box::new(self))
    }

    pub fn with_scale_and_flip(self, scale: f64) -> Stencil {
        Stencil::TranslateScale(TranslateScale::scale(scale), true, Box::new(self))
    }

    pub fn and(self, other: Stencil) -> Stencil {
        match (self, other) {
            (
                Stencil::Combine(CombineStencil(mut mine)),
                Stencil::Combine(CombineStencil(mut theirs)),
            ) => {
                mine.append(&mut theirs);
                Stencil::Combine(CombineStencil(mine))
            }
            (Stencil::Combine(CombineStencil(mut stencils)), other) => {
                stencils.push(other);
                Stencil::Combine(CombineStencil(stencils))
            }
            (me, Stencil::Combine(CombineStencil(mut stencils))) => {
                stencils.push(me);
                Stencil::Combine(CombineStencil(stencils))
            }
            (inner, other) => Stencil::Combine(CombineStencil(vec![inner, other])),
        }
    }

    pub fn and_right(self, other: Stencil) -> Stencil {
        let advance = self.advance();
        self.and(other.with_translation(Vec2::new(advance, 0.0)))
    }

    pub fn rect(&self) -> Rect {
        match self {
            Stencil::Path(path) => path.bounds,
            Stencil::TranslateScale(ts, _flip, child) => *ts * child.rect(),
            Stencil::Combine(combine) => {
                let mut rect = combine.0.first().map(|f| f.rect()).unwrap_or_default();

                for child_rect in combine.0.iter().skip(1).map(|c| c.rect()) {
                    if child_rect.x0 < rect.x0 {
                        rect.x0 = child_rect.x0;
                    }
                    if child_rect.y0 < rect.y0 {
                        rect.y0 = child_rect.y0;
                    }
                    if child_rect.x1 > rect.x1 {
                        rect.x1 = child_rect.x1;
                    }
                    if child_rect.y1 > rect.y1 {
                        rect.y1 = child_rect.y1;
                    }
                }

                rect
            }
            // TODO(joshuan): The height is of course not accurate here.
            Stencil::Text(text) => Rect::new(0.0, -text.font_size, text.width, text.font_size),
        }
    }

    pub fn advance(&self) -> f64 {
        match self {
            Stencil::Path(path) => path.advance,
            Stencil::TranslateScale(ts, _flip, child) => {
                let (translation, scale) = ts.as_tuple();
                translation.x + scale * child.advance()
            }
            Stencil::Combine(combine) => {
                combine
                    .0
                    .iter()
                    .map(|c| c.advance())
                    .fold(0.0, |max_so_far, child_adv| {
                        if child_adv > max_so_far {
                            child_adv
                        } else {
                            max_so_far
                        }
                    })
            }
            Stencil::Text(text) => text.width,
        }
    }

    /// Convert from staff-size (1 unit is 1 staff) to paper-size (1 unit is 1 mm)
    ///
    /// Behind Bars, p483.
    ///
    /// Rastal sizes vary from 0 to 8, where 0 is large and 8 is small.
    ///  - 0 and 1 are used for educational music.
    ///  - 2 is not generally used, but is sometimes used for piano music/songs.
    ///  - 3-4 are commonly used for single-staff-parts, piano music, and songs.
    ///  - 5 is less commonly used for single-staff-parts, piano music, and songs.
    ///  - 6-7 are used for choral music, cue saves, or ossia.
    ///  - 8 is used for full scores.
    pub fn with_paper_size(self, rastal: u8) -> Stencil {
        match rastal {
            0 => self.with_scale(9.2 * 1000.0),
            1 => self.with_scale(7.9 * 1000.0),
            2 => self.with_scale(7.4 * 1000.0),
            3 => self.with_scale(7.0 * 1000.0),
            4 => self.with_scale(6.5 * 1000.0),
            5 => self.with_scale(6.0 * 1000.0),
            6 => self.with_scale(5.5 * 1000.0),
            7 => self.with_scale(4.8 * 1000.0),
            8 => self.with_scale(3.7 * 1000.0),
            _ => panic!("Expected rastal size <= 8"),
        }
    }

    /// Generate an SVG representation of the string, without newlines.
    pub fn to_svg(&self) -> String {
        match self {
            Stencil::Path(path) => [
                "<path d=\"",
                &path.outline.to_svg().replace('\n', ""),
                "\" />",
            ]
            .concat(),
            Stencil::TranslateScale(ts, flip, child) => {
                let (translation, scale) = ts.as_tuple();

                [
                    "<g transform=\"translate(",
                    &translation.x.to_string(),
                    ",",
                    &translation.y.to_string(),
                    ") ",
                    "scale(",
                    &scale.to_string(),
                    ",",
                    &(if *flip {
                       -scale
                    } else {
                        scale
                    }.to_string()),
                    ")\">",
                    &child.to_svg(),
                    "</g>",
                ]
                .concat()
            }
            Stencil::Combine(combine) => {
                let mut parts = Vec::with_capacity(combine.0.len() + 2);
                parts.push("<g>".to_owned());
                for part in &combine.0 {
                    parts.push(part.to_svg());
                }
                parts.push("</g>".to_owned());
                parts.concat()
            }
            Stencil::Text(text) => {
                [
                    "<text style=\"font-size: ",
                    &text.font_size.to_string(),
                    "px; font-family: Palatino, 'Palatino Linotype', 'Palatino LT STD', 'Book Antiqua', Georgia, serif; \">",
                    &escape(&text.text),
                    "</text>",
                ].concat()
            }
        }
    }

    pub fn to_svg_doc_for_testing(&self) -> String {
        [
            "<svg viewBox=\"0 0 215.9 279.4\" width=\"215.9mm\" height=\"279.4mm\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\"><g transform=\"scale(1, -1)\">",
            &self.to_svg(),
            "</g></svg>\n"
        ].concat()
    }
}

impl Default for Stencil {
    fn default() -> Stencil {
        Stencil::Combine(CombineStencil(vec![]))
    }
}

pub fn snapshot(path: &str, contents: &str) {
    if std::env::vars().any(|(key, _val)| key == "SIX_SNAPSHOT") {
        std::fs::write(path, contents).unwrap();
    } else {
        assert_eq!(std::fs::read_to_string(path).unwrap(), contents);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn time_signatures() {
        let times = Stencil::padding(200.0)
            .and_right(Stencil::time_sig_fraction(4, 4))
            .and_right(Stencil::time_sig_fraction(3, 4))
            .and_right(Stencil::time_sig_fraction(5, 4))
            .and_right(Stencil::time_sig_fraction(7, 4))
            .and_right(Stencil::time_sig_fraction(12, 8))
            .and_right(Stencil::time_sig_fraction(6, 16))
            .and_right(Stencil::time_sig_fraction(9, 8))
            .and_right(Stencil::time_sig_fraction(6, 8))
            .and_right(Stencil::time_sig_common())
            .and_right(Stencil::time_sig_cut())
            .and_right(Stencil::time_sig_cancel())
            .and_right(Stencil::padding(200.0));

        let right = times.advance();

        snapshot(
            "./snapshots/time_signature_stencils.svg",
            &Stencil::staff_line(right)
                .and(times)
                .with_translation(Vec2::new(0.0, -1000.0))
                .with_paper_size(3)
                .to_svg_doc_for_testing(),
        );
    }

    #[test]
    fn clefs() {
        let clefs = Stencil::padding(200.0)
            .and_right(Stencil::clef_c())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::clef_f().with_translation(Vec2::new(0.0, 250.0)))
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::clef_g().with_translation(Vec2::new(0.0, -250.0)))
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::clef_unpitched())
            .and_right(Stencil::padding(200.0));
        let right = clefs.rect().x1;

        let staff_lines: Vec<Stencil> = (-2..=2)
            .map(|i| {
                Stencil::staff_line(right).with_translation(Vec2::new(0.0, (i as f64) * 250.0))
            })
            .collect();

        snapshot(
            "./snapshots/clef_stencils.svg",
            &Stencil::combine(staff_lines)
                .and(clefs)
                .with_translation(Vec2::new(0.0, -1000.0))
                .with_paper_size(3)
                .to_svg_doc_for_testing(),
        );
    }

    #[test]
    fn rests() {
        let rests = Stencil::padding(200.0)
            .and_right(Stencil::rest_256())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_128())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_64())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_32())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_16())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_8())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_quarter())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_half())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_whole())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_double_whole())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_longa())
            .and_right(Stencil::padding(200.0))
            .and_right(Stencil::rest_maxima())
            .and_right(Stencil::padding(200.0));

        let right = rests.rect().x1;

        let staff_lines: Vec<Stencil> = (-2..=2)
            .map(|i| {
                Stencil::staff_line(right).with_translation(Vec2::new(0.0, (i as f64) * 250.0))
            })
            .collect();

        snapshot(
            "./snapshots/rest_stencils.svg",
            &Stencil::combine(staff_lines)
                .and(rests)
                .with_translation(Vec2::new(0.0, -1000.0))
                .with_paper_size(3)
                .to_svg_doc_for_testing(),
        );
    }
}
