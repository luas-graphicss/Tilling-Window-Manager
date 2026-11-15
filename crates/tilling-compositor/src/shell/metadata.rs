use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;
use tilling_core::WindowMetadata;

const MAX_FIELD_BYTES: usize = 512;

pub(crate) fn read(surface: &WlSurface) -> WindowMetadata {
    with_states(surface, |states| {
        let Some(attributes) = states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .and_then(|data| data.lock().ok())
        else {
            return WindowMetadata::default();
        };
        WindowMetadata {
            app_id: attributes.app_id.as_deref().map(bounded),
            title: attributes.title.as_deref().map(bounded),
        }
    })
}

fn bounded(text: &str) -> String {
    let end = text
        .char_indices()
        .map(|(start, character)| start + character.len_utf8())
        .take_while(|end| *end <= MAX_FIELD_BYTES)
        .last()
        .unwrap_or(0);
    text.get(..end).unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_kept_whole() {
        assert_eq!(bounded("org.example.Editor"), "org.example.Editor");
        assert_eq!(bounded(""), "");
    }

    #[test]
    fn long_text_is_cut_at_a_character_boundary() {
        let title = "é".repeat(MAX_FIELD_BYTES);

        let cut = bounded(&title);

        assert_eq!(cut.len(), MAX_FIELD_BYTES);
        assert_eq!(cut.chars().count(), MAX_FIELD_BYTES / 2);
    }

    #[test]
    fn multibyte_character_crossing_the_limit_is_dropped() {
        let title = format!("{}é", "a".repeat(MAX_FIELD_BYTES - 1));

        assert_eq!(bounded(&title), "a".repeat(MAX_FIELD_BYTES - 1));
    }
}
