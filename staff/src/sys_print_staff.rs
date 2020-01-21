use std::collections::HashMap;

use crate::Staff;
use entity::{EntitiesRes, Entity, Join};
use kurbo::{TranslateScale, Vec2};
use rhythm::{Bar, RelativeRhythmicSpacing};
use stencil::{Stencil, StencilMap};

pub fn sys_print_staff(
    entities: &EntitiesRes,
    staffs: &mut HashMap<Entity, Staff>,
    bars: &HashMap<Entity, Bar>,
    spacing: &HashMap<Entity, RelativeRhythmicSpacing>,
    stencils: &HashMap<Entity, Stencil>,
    stencil_maps: &mut HashMap<Entity, StencilMap>,
    children: &HashMap<Entity, Vec<Entity>>,
) {
    for (staff_entity, (staff, children)) in (staffs, children).join() {
        let mut staff_advance = 0.0f64;
        let mut staff_stencil = StencilMap::default();

        for child in children {
            if let Some(bar) = bars.get(&child) {
                let mut advance_step = 0.0f64;
                for (_, _, entity, _) in bar.children() {
                    let stencil = &stencils[&entity];
                    let relative_spacing = spacing[&entity];
                    advance_step =
                        advance_step.max(stencil.rect().x1 / relative_spacing.relative());
                }

                let advance_step = advance_step + 100.0; // freeze

                let mut bar_stencil = StencilMap::default();
                let mut advance = 200.0;
                for (_, _, entity, _) in bar.children() {
                    let relative_spacing = spacing[&entity];

                    bar_stencil = bar_stencil.and(
                        entity,
                        Some(TranslateScale::translate(Vec2::new(advance, 0.0))),
                    );
                    advance += advance_step * relative_spacing.relative();
                }

                stencil_maps.insert(*child, bar_stencil);

                staff_stencil = staff_stencil.and(
                    *child,
                    Some(TranslateScale::translate(Vec2::new(staff_advance, 0.0))),
                );
                staff_advance += advance;
            } else if let Some(stencil) = stencils.get(&child) {
                staff_stencil = staff_stencil.and(
                    *child,
                    Some(TranslateScale::translate(Vec2::new(staff_advance, 0.0))),
                );
                staff_advance += stencil.advance();
            }
        }

        staff.width = staff_advance;

        let staff_lines = staff.staff_lines.get_or_insert_with(|| entities.create());
        staff_stencil = staff_stencil.and(*staff_lines, None);

        stencil_maps.insert(staff_entity, staff_stencil);
    }
}
