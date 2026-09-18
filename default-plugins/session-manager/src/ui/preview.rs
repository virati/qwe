use unicode_width::UnicodeWidthChar;
use zellij_tile::prelude::*;

use crate::ui::components::Colors;

/// What the preview panel knows about the currently selected session.
///
/// Previews are published by each running session about once per second and read from that
/// session's info cache folder, so what we show here can lag the live session by that much.
#[derive(Debug, Default)]
pub enum SessionPreviewState {
    /// Nothing is selected in the list, so there is nothing to preview
    #[default]
    NoSelection,
    /// The selected session is not running (it is resurrectable) and so has no live contents
    NotRunning { session_name: String },
    /// We have a preview for the selected session
    Loaded {
        session_name: String,
        preview: SessionPreview,
    },
    /// The selected session is running but we could not read a preview for it
    Unavailable {
        session_name: String,
        reason: String,
    },
}

/// The width the preview panel is given when there is room for it, including its separator
/// column. Below [`MIN_WIDTH_FOR_PREVIEW`] total columns we drop the panel entirely rather than
/// squeeze the session list.
///
/// The threshold is deliberately low: the session manager is usually opened as a floating pane,
/// and a new floating pane is half the width of the screen, so a preview that only appeared on
/// very wide terminals would hardly ever appear at all.
pub const PREVIEW_PANEL_MAX_WIDTH: usize = 48;
pub const PREVIEW_PANEL_MIN_WIDTH: usize = 20;
pub const MIN_WIDTH_FOR_PREVIEW: usize = 60;

/// How wide the preview panel should be, given the total width available to the screen.
/// Returns 0 when the screen is too narrow to carry a preview panel at all.
pub fn preview_panel_width(total_width: usize) -> usize {
    if total_width < MIN_WIDTH_FOR_PREVIEW {
        return 0;
    }
    let one_third = total_width / 3;
    std::cmp::max(
        PREVIEW_PANEL_MIN_WIDTH,
        std::cmp::min(PREVIEW_PANEL_MAX_WIDTH, one_third),
    )
}

/// Renders the preview panel, including the vertical rule separating it from the session list.
///
/// `x` is the column of the vertical rule; the preview contents start two columns to its right.
pub fn render_session_preview(
    state: &SessionPreviewState,
    colors: Colors,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    if width < 4 || height == 0 {
        return;
    }
    // draw the rule first, clearing everything to the right of it, then write the contents on
    // top - so that a previous, wider render cannot leave anything behind in this column
    for row in 0..height {
        print!("\u{1b}[m\u{1b}[{};{}H│\u{1b}[K", y + row + 1, x + 1);
    }
    let content_x = x + 2;
    let content_width = width.saturating_sub(2);
    let mut lines: Vec<String> = vec![];

    match state {
        SessionPreviewState::NoSelection => {
            lines.push(colors.shortcuts("<↓↑>").to_string());
            lines.push("Select a session to preview it".to_owned());
        },
        SessionPreviewState::NotRunning { session_name } => {
            lines.push(colors.bold(&clip(session_name, content_width)));
            lines.push(String::new());
            lines.push("Exited session - nothing is running,".to_owned());
            lines.push("so there is nothing to preview.".to_owned());
        },
        SessionPreviewState::Unavailable {
            session_name,
            reason,
        } => {
            lines.push(colors.bold(&clip(session_name, content_width)));
            lines.push(String::new());
            for line in wrap(reason, content_width) {
                lines.push(line);
            }
        },
        SessionPreviewState::Loaded {
            session_name,
            preview,
        } => {
            lines.push(colors.bold(&clip(session_name, content_width)));
            if !preview.tab_name.is_empty() {
                lines.push(colors.tab_count(&clip(&preview.tab_name, content_width)));
            }
            lines.push(String::new());
            if preview.panes.is_empty() {
                lines.push("No panes to preview.".to_owned());
            } else if preview.is_empty() {
                lines.push(colors.exit_code_error("This session looks empty."));
                lines.push(String::new());
            }
            for pane in &preview.panes {
                let title = if pane.title.is_empty() {
                    "(untitled pane)".to_owned()
                } else {
                    pane.title.clone()
                };
                lines.push(
                    colors.session_and_folder_entry(&clip(&format!("▸ {}", title), content_width)),
                );
                for line in &pane.lines {
                    lines.push(clip(line, content_width));
                }
                lines.push(String::new());
            }
        },
    }

    for (row, line) in lines.iter().take(height).enumerate() {
        print!("\u{1b}[m\u{1b}[{};{}H{}", y + row + 1, content_x + 1, line);
    }
}

/// Truncates `text` to `max_width` terminal columns, dropping anything the terminal should not
/// be asked to draw (the contents come from another session's screen, so we do not trust them
/// to be free of control characters).
fn clip(text: &str, max_width: usize) -> String {
    let mut clipped = String::new();
    let mut current_width = 0;
    for character in text.chars() {
        if character.is_control() {
            continue;
        }
        let character_width = character.width().unwrap_or(0);
        if current_width + character_width > max_width {
            break;
        }
        current_width += character_width;
        clipped.push(character);
    }
    clipped
}

/// Breaks `text` into lines of at most `max_width` columns, on whitespace where possible.
fn wrap(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = vec![];
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word = clip(word, max_width);
        if current.is_empty() {
            current = word;
        } else if current.chars().count() + 1 + word.chars().count() <= max_width {
            current.push(' ');
            current.push_str(&word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
