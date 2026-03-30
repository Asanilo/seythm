use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use image::codecs::png::PngEncoder;
use image::{ColorType, ImageEncoder};
use ratatui::layout::Rect;

use crate::app::{DemoApp, DemoMode};
use crate::ui::widgets::imported_song_select::imported_song_select_cover_image_rect;
use crate::ui::widgets::loading::loading_cover_image_rect;
use crate::ui::widgets::results::results_cover_image_rect;
use crate::ui::widgets::song_select::song_select_cover_image_rect;

const KITTY_CHUNK_SIZE: usize = 4096;
const KITTY_IMAGE_ID: u32 = 4242;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtworkRenderKey {
    mode: DemoMode,
    song_id: String,
    artwork_path: PathBuf,
    area: Rect,
}

#[derive(Default)]
pub struct ArtworkRenderer {
    last: Option<ArtworkRenderKey>,
}

impl ArtworkRenderer {
    pub fn sync<W: Write>(
        &mut self,
        writer: &mut W,
        app: &DemoApp,
        terminal_area: Rect,
    ) -> std::io::Result<()> {
        let next = current_artwork_target(app, terminal_area);
        if self.last == next {
            return Ok(());
        }

        self.clear(writer)?;

        if let Some(target) = next.as_ref() {
            let png = load_png_payload(&target.artwork_path)?;
            transmit_artwork(writer, &png, target.area)?;
        }

        self.last = next;
        writer.flush()
    }

    pub fn clear<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        queue!(writer, MoveTo(0, 0))?;
        write!(writer, "\x1b_Ga=d,d=I,i={KITTY_IMAGE_ID}\x1b\\")?;
        self.last = None;
        Ok(())
    }
}

pub fn terminal_supports_graphics() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|value| value.eq_ignore_ascii_case("ghostty"))
        .unwrap_or(false)
        || std::env::var("TERM")
            .map(|value| value.contains("kitty"))
            .unwrap_or(false)
}

fn current_artwork_target(app: &DemoApp, area: Rect) -> Option<ArtworkRenderKey> {
    let mode = app.mode();
    let (song_id, artwork_path, image_area) = match mode {
        DemoMode::SongSelect => {
            let song = app.selected_song();
            (
                song.id().to_string(),
                song.artwork_path().and_then(resolve_image_artwork_path)?,
                song_select_cover_image_rect(area)?,
            )
        }
        DemoMode::ImportedSelect => {
            let song = app
                .selected_imported_song()
                .unwrap_or_else(|| app.selected_song());
            (
                song.id().to_string(),
                song.artwork_path().and_then(resolve_image_artwork_path)?,
                imported_song_select_cover_image_rect(area)?,
            )
        }
        DemoMode::Loading => {
            let song = app.active_song();
            (
                song.id().to_string(),
                song.artwork_path().and_then(resolve_image_artwork_path)?,
                loading_cover_image_rect(area)?,
            )
        }
        DemoMode::Results => {
            let song = app.active_song();
            (
                song.id().to_string(),
                song.artwork_path().and_then(resolve_image_artwork_path)?,
                results_cover_image_rect(area)?,
            )
        }
        _ => return None,
    };

    Some(ArtworkRenderKey {
        mode,
        song_id,
        artwork_path,
        area: image_area,
    })
}

fn resolve_image_artwork_path(path: &Path) -> Option<PathBuf> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Some(path.to_path_buf()),
        _ => None,
    }
}

fn load_png_payload(path: &Path) -> std::io::Result<Vec<u8>> {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("png"))
        .unwrap_or(false)
    {
        return fs::read(path);
    }

    let image = image::open(path).map_err(std::io::Error::other)?;
    let rgba = image.to_rgba8();
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(std::io::Error::other)?;
    Ok(encoded)
}

fn transmit_artwork<W: Write>(writer: &mut W, png_bytes: &[u8], area: Rect) -> std::io::Result<()> {
    if area.width == 0 || area.height == 0 {
        return Ok(());
    }

    let payload = BASE64_STANDARD.encode(png_bytes);
    queue!(writer, MoveTo(area.x, area.y))?;

    let mut offset = 0;
    let mut first = true;
    while offset < payload.len() {
        let end = (offset + KITTY_CHUNK_SIZE).min(payload.len());
        let chunk = &payload[offset..end];
        let more = if end < payload.len() { 1 } else { 0 };

        if first {
            write!(writer, "{}", kitty_transmit_chunk(area, chunk, more))?;
            first = false;
        } else {
            write!(writer, "\x1b_Gm={more};{chunk}\x1b\\")?;
        }
        offset = end;
    }

    Ok(())
}

fn kitty_transmit_chunk(area: Rect, chunk: &str, more: u8) -> String {
    format!(
        "\x1b_Ga=T,f=100,i={KITTY_IMAGE_ID},q=2,c={},r={},C=1,m={more};{chunk}\x1b\\",
        area.width, area.height
    )
}

#[cfg(test)]
mod tests {
    use super::{current_artwork_target, kitty_transmit_chunk};
    use crate::app::{DemoApp, DemoMode};
    use crate::content::{load_imported_song_catalog, save_imported_song_catalog};
    use crate::osu::import::import_osu_mania_folder;
    use crate::runtime::GameTime;
    use ratatui::layout::Rect;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn kitty_transmit_chunk_uses_quiet_mode_for_fullscreen_apps() {
        let command = kitty_transmit_chunk(Rect::new(4, 5, 20, 10), "abcd", 0);

        assert!(command.contains("q=2"));
        assert!(command.contains("c=20"));
        assert!(command.contains("r=10"));
    }

    #[test]
    fn current_artwork_target_uses_results_cover_for_finished_imported_song() {
        let import_root = temp_import_root("results-artwork-target");
        let source_folder = fixture_copy("tests/fixtures/osu/valid-6k", "results-artwork-source");
        import_osu_mania_folder(&source_folder, &import_root).expect("seed import catalog");

        let artwork_path = import_root.join("cover.png");
        fs::write(&artwork_path, []).expect("write artwork placeholder");

        let mut catalog = load_imported_song_catalog(&import_root).expect("load imported catalog");
        let entry = catalog
            .songs
            .first_mut()
            .expect("imported song should be present");
        entry.artwork_path = Some(artwork_path.clone());
        save_imported_song_catalog(&import_root, &catalog).expect("save imported catalog");

        let mut app = DemoApp::from_runtime_chart_with_import_root(&import_root)
            .expect("runtime app should load imported catalog");
        app.open_imported_view();
        app.start_selected_chart()
            .expect("selected chart should start");
        app.skip_loading_intro();
        app.update(GameTime::from_millis(5_000));

        assert_eq!(app.mode(), DemoMode::Results);

        let target = current_artwork_target(&app, Rect::new(0, 0, 120, 40))
            .expect("results view should expose artwork target");
        assert_eq!(target.mode, DemoMode::Results);
        assert_eq!(target.artwork_path, artwork_path);
        assert!(target.area.width > 0);
        assert!(target.area.height > 0);
    }

    fn temp_import_root(test_name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "code_m-{}-{}-{}",
            test_name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_millis()
        ));
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn fixture_copy(folder: &str, suffix: &str) -> PathBuf {
        let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(folder);
        let temp_root = temp_import_root(suffix);
        fs::copy(source_root.join("map.osu"), temp_root.join("map.osu")).expect("copy map");
        fs::copy(source_root.join("song.ogg"), temp_root.join("song.ogg")).expect("copy audio");
        temp_root
    }
}
