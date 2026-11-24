//! Logo harness/provider dari `web/assets/providers/`.
//!
//! Aset yang sama dipakai GUI web, jadi menambah logo di sana otomatis ikut
//! ter-bundle: daftar id, ekstensi PNG, dan varian `-dark` dihitung dari isi
//! folder, sama seperti `scripts/sync-logos.mjs`. Id yang tidak punya file
//! memakai mark OpenAI-compatible (`custom.svg`), bukan monogram.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex};

use gpui_kit::*;

#[derive(rust_embed::RustEmbed)]
#[folder = "../../web/assets/providers"]
#[include = "*.svg"]
#[include = "*.png"]
struct Files;

/// Asset source aplikasi: logo provider + aset ikon/font gpui-kit.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("providers/") {
            return Ok(Files::get(name).map(|file| file.data));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if let Some(prefix) = path.strip_prefix("providers/") {
            return Ok(Files::iter()
                .filter(|name| name.starts_with(prefix))
                .map(|name| SharedString::from(name.to_string()))
                .collect());
        }
        gpui_kit::assets::Assets.list(path)
    }
}

struct Index {
    known: HashSet<String>,
    png: HashSet<String>,
    dual: HashSet<String>,
}

static INDEX: LazyLock<Index> = LazyLock::new(|| {
    let mut known = HashSet::new();
    let mut png = HashSet::new();
    let mut dual = HashSet::new();
    for name in Files::iter() {
        let name = name.as_ref();
        let is_png = name.ends_with(".png");
        let Some(stem) = name
            .strip_suffix(".png")
            .or_else(|| name.strip_suffix(".svg"))
        else {
            continue;
        };
        if let Some(base) = stem.strip_suffix("-dark") {
            dual.insert(base.to_string());
            continue;
        }
        known.insert(stem.to_string());
        if is_png {
            png.insert(stem.to_string());
        }
    }
    Index { known, png, dual }
});

/// Raster SVG penuh warna. Elemen `svg()` GPUI hanya memakai alpha mask dan
/// mewarnai seluruh glyph dengan satu warna, jadi logo bergradien (OMP,
/// Mistral, Hugging Face) akan jadi kotak/siluet. Di sini SVG dirasterkan
/// lewat renderer yang sama dengan cache per (berkas, ukuran).
fn svg_image(cx: &gpui_kit::App, file: &str, size_px: u32) -> Option<Arc<RenderImage>> {
    static CACHE: LazyLock<Mutex<HashMap<(String, u32), Arc<RenderImage>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let key = (file.to_string(), size_px);
    if let Some(image) = CACHE.lock().ok()?.get(&key).cloned() {
        return Some(image);
    }

    let bytes = Files::get(file.strip_prefix("providers/")?)?;
    let renderer = cx.svg_renderer();
    let parsed = renderer.parse_svg(bytes.data.as_ref()).ok()?;
    let exact = size(
        DevicePixels(size_px as i32),
        DevicePixels(size_px as i32),
    );
    let image = renderer.render_parsed(&parsed, SvgSize::ExactSize(exact)).ok()?;

    CACHE.lock().ok()?.insert(key, image.clone());
    Some(image)
}

/// Elemen logo berukuran `size` persegi. `dark` memilih varian `-dark` untuk
/// mark yang glyph-nya gelap (pi, hermes, ollama).
///
/// Menerima context view mana pun (`Context<App>`, `Context<AddView>`, …)
/// karena renderer SVG diambil dari `App`.
pub fn logo<C: std::ops::Deref<Target = gpui_kit::App>>(
    cx: &C,
    dark: bool,
    id: &str,
    size: f32,
) -> AnyElement {
    let asset = if INDEX.known.contains(id) { id } else { "custom" };
    let is_png = INDEX.png.contains(asset);
    let use_dark = dark && INDEX.dual.contains(asset);

    let extension = if is_png { "png" } else { "svg" };
    let file = if use_dark {
        format!("providers/{asset}-dark.{extension}")
    } else {
        format!("providers/{asset}.{extension}")
    };

    if is_png {
        return img(file).size(px(size)).into_any_element();
    }

    // 2x supaya tetap tajam saat jendela di-HiDPI; `.size()` mengecilkannya.
    let pixels = (size * 2.0).round().max(1.0) as u32;
    match svg_image(cx, &file, pixels) {
        Some(image) => img(image).size(px(size)).into_any_element(),
        None => div().size(px(size)).into_any_element(),
    }
}
