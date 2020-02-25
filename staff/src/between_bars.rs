use entity::{EntitiesRes, Entity};
use kurbo::Vec2;
use pitch::{Clef, NoteName, Pitch};
use rest_note_chord::Context;
use std::cmp::Ordering;
use stencil::Stencil;
use wasm_bindgen::prelude::*;

pub fn get_pitches(key: i8, clef: Clef) -> Vec<Pitch> {
    if clef == Clef::Percussion {
        return vec![];
    }

    let octave_offset = match clef {
        Clef::G => 0,
        Clef::F => -2,
        Clef::Percussion => panic!("Unexpected percussion clef"),
    };

    let sharps = [
        Pitch::new(NoteName::F, None, 5 + octave_offset),
        Pitch::new(NoteName::C, None, 5 + octave_offset),
        Pitch::new(NoteName::G, None, 5 + octave_offset),
        Pitch::new(NoteName::D, None, 5 + octave_offset),
        Pitch::new(NoteName::A, None, 4 + octave_offset),
        Pitch::new(NoteName::E, None, 5 + octave_offset),
        Pitch::new(NoteName::B, None, 4 + octave_offset),
    ];

    let flats = [
        Pitch::new(NoteName::B, None, 4 + octave_offset),
        Pitch::new(NoteName::E, None, 5 + octave_offset),
        Pitch::new(NoteName::A, None, 4 + octave_offset),
        Pitch::new(NoteName::D, None, 5 + octave_offset),
        Pitch::new(NoteName::G, None, 4 + octave_offset),
        Pitch::new(NoteName::C, None, 5 + octave_offset),
        Pitch::new(NoteName::F, None, 4 + octave_offset),
    ];

    match key.cmp(&0) {
        Ordering::Greater => sharps[0..(key as usize)].iter().cloned().collect(),
        Ordering::Less => flats[0..(-key as usize)].iter().cloned().collect(),
        Ordering::Equal => Vec::new(),
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Barline {
    Normal,
    Final,
}

#[derive(Debug)]
pub struct BetweenBars {
    pub clef: Option<Clef>,
    pub time: Option<(u8, u8)>,
    pub key: Option<i8>,
    pub barline: Option<Barline>,

    /// Stencil if this is at the start of a line.
    pub stencil_start: Entity,

    /// Stencil if this is in the middle of a line.
    pub stencil_middle: Entity,

    /// Stencil if this is at the end of a line.
    pub stencil_end: Entity,
}

impl BetweenBars {
    pub fn new(&self, entities: &EntitiesRes) -> BetweenBars {
        BetweenBars {
            clef: None,
            time: None,
            key: None,
            barline: None,
            stencil_start: entities.create(),
            stencil_middle: entities.create(),
            stencil_end: entities.create(),
        }
    }

    pub fn render_start(&self, context: &Context) -> Stencil {
        let mut stencil = Stencil::default();

        let clef = self.clef.unwrap_or(context.clef);
        let key = self.key.unwrap_or(context.key);

        stencil = stencil
            .and_right(Stencil::padding(100.0))
            .and_right(match clef {
                Clef::G => Stencil::clef_g().with_translation(Vec2::new(0f64, 250f64)),
                Clef::F => Stencil::clef_f().with_translation(Vec2::new(0f64, -250f64)),
                Clef::Percussion => Stencil::clef_unpitched(),
            })
            .and_right(Stencil::padding(100.0));

        if key != 0 && clef != Clef::Percussion {
            stencil = stencil.and_right(Stencil::padding(100.0));
            for pitch in get_pitches(key, clef) {
                stencil = stencil.and_right(
                    if key < 0 {
                        Stencil::flat()
                    } else {
                        Stencil::sharp()
                    }
                    .with_translation(Vec2::new(0.0, pitch.y(clef))),
                );
            }

            stencil = stencil.and_right(Stencil::padding(100.0));
        }

        if let Some((num, den)) = self.time {
            stencil = stencil.and_right(Stencil::time_sig_fraction(num, den));
        }

        stencil
    }

    pub fn render_mid(&self, _context: &Context) -> Stencil {
        let mut stencil = Stencil::default();

        match self.barline {
            Some(Barline::Normal) => {
                stencil = stencil
                    .and_right(Stencil::padding(100.0))
                    .and_right(Stencil::barline_thin(0.0, -500.0, 500.0))
            }
            Some(Barline::Final) => {
                stencil = stencil
                    .and_right(Stencil::padding(100.0))
                    .and_right(Stencil::barline_thin(0.0, -500.0, 500.0))
                    .and_right(Stencil::padding(125.0))
                    .and_right(Stencil::barline_thick(0.0, -500.0, 500.0));
            }
            None => {}
        }

        if let Some(clef) = self.clef {
            stencil = stencil
                .and_right(Stencil::padding(100.0))
                .and_right(match clef {
                    Clef::G => Stencil::clef_g().with_translation(Vec2::new(0f64, 250f64)),
                    Clef::F => Stencil::clef_f().with_translation(Vec2::new(0f64, -250f64)),
                    Clef::Percussion => Stencil::clef_unpitched(),
                })
                .and_right(Stencil::padding(100.0));
        }

        if let (Some(key), Some(clef)) = (self.key, self.clef) {
            if key != 0 && clef != Clef::Percussion {
                stencil = stencil.and_right(Stencil::padding(100.0));
                for pitch in get_pitches(key, clef) {
                    stencil = stencil.and_right(
                        if key < 0 {
                            Stencil::flat()
                        } else {
                            Stencil::sharp()
                        }
                        .with_translation(Vec2::new(0.0, pitch.y(clef))),
                    );
                }

                stencil = stencil.and_right(Stencil::padding(100.0));
            }
        }

        if let Some((num, den)) = self.time {
            stencil = stencil.and_right(Stencil::time_sig_fraction(num, den));
        }

        stencil
    }

    pub fn render_end(&self, _context: &Context) -> Stencil {
        let mut stencil = Stencil::default();

        match self.barline {
            Some(Barline::Normal) => {
                stencil = stencil
                    .and_right(Stencil::padding(100.0))
                    .and_right(Stencil::barline_thin(0.0, -500.0, 500.0))
            }
            Some(Barline::Final) => {
                stencil = stencil
                    .and_right(Stencil::padding(100.0))
                    .and_right(Stencil::barline_thin(0.0, -500.0, 500.0))
                    .and_right(Stencil::padding(125.0))
                    .and_right(Stencil::barline_thick(0.0, -500.0, 500.0));
            }
            None => {}
        }

        // TODO: warnings!

        stencil
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_rational::Rational;

    #[test]
    fn signatures() {
        use stencil::snapshot;

        let context = Context {
            bar: 0,
            beat: Rational::new(0, 1),
            natural_beat: Rational::new(0, 1),
            clef: Clef::G,
            key: 0,
            time: (4, 4),
        };

        snapshot(
            "./snapshots/signatures.svg",
            &BetweenBars {
                clef: Some(Clef::G),
                time: Some((4, 4)),
                key: Some(0),
                barline: Some(Barline::Normal),
                stencil_start: Entity::new(0),
                stencil_middle: Entity::new(1),
                stencil_end: Entity::new(2),
            }
            .render_start(&context)
            .and_right(
                BetweenBars {
                    clef: Some(Clef::G),
                    time: Some((4, 4)),
                    key: Some(6),
                    barline: Some(Barline::Normal),
                    stencil_start: Entity::new(0),
                    stencil_middle: Entity::new(1),
                    stencil_end: Entity::new(2),
                }
                .render_start(&context),
            )
            .and_right(
                BetweenBars {
                    clef: Some(Clef::G),
                    time: Some((4, 4)),
                    key: Some(-6),
                    barline: Some(Barline::Normal),
                    stencil_start: Entity::new(0),
                    stencil_middle: Entity::new(1),
                    stencil_end: Entity::new(2),
                }
                .render_start(&context),
            )
            .and_right(
                BetweenBars {
                    clef: Some(Clef::F),
                    time: Some((6, 8)),
                    key: Some(6),
                    barline: Some(Barline::Normal),
                    stencil_start: Entity::new(0),
                    stencil_middle: Entity::new(1),
                    stencil_end: Entity::new(2),
                }
                .render_start(&context),
            )
            .and_right(
                BetweenBars {
                    clef: Some(Clef::F),
                    time: Some((6, 8)),
                    key: Some(-6),
                    barline: Some(Barline::Normal),
                    stencil_start: Entity::new(0),
                    stencil_middle: Entity::new(1),
                    stencil_end: Entity::new(2),
                }
                .render_start(&context),
            )
            .with_translation(Vec2::new(0f64, 1000f64))
            .to_svg_doc_for_testing(),
        );
    }
}
