//! Small egui helpers for the two-column parameter grids used throughout the UI.

use egui::{Ui, Widget};
use glam::Vec3;

/// Draws the label cell, attaching `hover` as a tooltip when it is non-empty.
fn label(ui: &mut Ui, text: &str, hover: &str) {
    let response = ui.label(text);
    if !hover.is_empty() {
        response.on_hover_text(hover);
    }
}

/// Label + widget row. Returns true if the widget was edited
pub fn row(ui: &mut Ui, text: &str, widget: impl Widget) -> bool {
    row_hover(ui, text, "", widget)
}

/// As [`row`], with an explanatory tooltip on the label
pub fn row_hover(ui: &mut Ui, text: &str, hover: &str, widget: impl Widget) -> bool {
    label(ui, text, hover);
    let changed = ui.add(widget).changed();
    ui.end_row();
    changed
}

/// Row whose right-hand cell is built by `content`
pub fn custom_row<R>(ui: &mut Ui, text: &str, content: impl FnOnce(&mut Ui) -> R) -> R {
    custom_row_hover(ui, text, "", content)
}

/// As [`custom_row`], with an explanatory tooltip on the label
pub fn custom_row_hover<R>(
    ui: &mut Ui,
    text: &str,
    hover: &str,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    label(ui, text, hover);
    let result = ui.horizontal(content).inner;
    ui.end_row();
    result
}

/// Row holding a checkbox
pub fn check_row(ui: &mut Ui, text: &str, hover: &str, value: &mut bool) -> bool {
    label(ui, text, hover);
    let changed = ui.checkbox(value, "").changed();
    ui.end_row();
    changed
}

/// Row holding an RGB colour picker
pub fn rgb_row(ui: &mut Ui, text: &str, rgb: &mut [f32; 3]) -> bool {
    label(ui, text, "");
    let changed = ui.color_edit_button_rgb(rgb).changed();
    ui.end_row();
    changed
}

/// As [`rgb_row`], bound to a linear `Vec3`
pub fn colour_row(ui: &mut Ui, text: &str, colour: &mut Vec3) -> bool {
    let mut rgb = colour.to_array();
    let changed = rgb_row(ui, text, &mut rgb);
    if changed {
        *colour = Vec3::from(rgb);
    }
    changed
}

/// Row holding three `DragValue`s for the components of an `[f32; 3]`
pub fn vec3_row(ui: &mut Ui, text: &str, hover: &str, value: &mut [f32; 3], speed: f32) -> bool {
    custom_row_hover(ui, text, hover, |ui| {
        ["x: ", "y: ", "z: "]
            .iter()
            .zip(value)
            .fold(false, |changed, (prefix, v)| {
                changed
                    | ui.add(egui::DragValue::new(v).prefix(*prefix).speed(speed))
                        .changed()
            })
    })
}
