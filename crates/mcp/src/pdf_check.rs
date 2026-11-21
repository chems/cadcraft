//! What a written PDF holds, read back from its bytes: an agent that plots a sheet learns the
//! number of pages, the paper size and orientation of each and whether anything was drawn on it,
//! instead of only the file's length.

use serde_json::{Value, json};

/// Millimetres per PDF point (1/72 inch).
const MM_PER_PT: f64 = 25.4 / 72.0;
/// At most this many pages are described one by one; `pageCount` gives the total.
const LISTED_PAGES: usize = 50;
/// Larger files are not read back.
pub const MAX_READ_BYTES: u64 = 256 * 1024 * 1024;

/// Page operators that draw something: path painting and text showing.
const DRAWING_OPERATORS: [&[u8]; 6] = [b"S", b"s", b"f", b"B", b"Tj", b"TJ"];

/// Whether a content stream paints anything: one of [`DRAWING_OPERATORS`] as a whole word.
fn draws(content: &[u8]) -> bool {
    content
        .split(|c| c.is_ascii_whitespace() || matches!(c, b'[' | b']' | b'(' | b')' | b'<' | b'>' | b'/'))
        .any(|w| DRAWING_OPERATORS.contains(&w) || w == b"f*" || w == b"B*" || w == b"b" || w == b"b*" || w == b"Do")
}

/// The pages of the PDF in `bytes`: `{pageCount, pages: [{widthMm, heightMm, landscape, drawn}]}`,
/// or `{error}` when it doesn't read as a PDF.
pub fn report(bytes: &[u8]) -> Value {
    let pdf = match craft_pdf::open_slice(bytes, None) {
        Ok(p) => p,
        Err(e) => return json!({ "error": format!("the file does not read back as a PDF: {e:?}") }),
    };
    let pages = pdf.pages();
    let listed: Vec<Value> = pages
        .iter()
        .take(LISTED_PAGES)
        .map(|page| {
            let m = page.media_box();
            let (w, h) = (((m.x1 - m.x0).abs() * MM_PER_PT * 10.0).round() / 10.0, ((m.y1 - m.y0).abs() * MM_PER_PT * 10.0).round() / 10.0);
            let drawn = page.page_stream().is_some_and(draws);
            json!({ "widthMm": w, "heightMm": h, "landscape": w > h, "drawn": drawn })
        })
        .collect();
    let blank = listed.iter().filter(|p| p["drawn"] == false).count();
    let mut out = json!({ "pageCount": pages.len(), "pages": listed });
    if blank > 0
        && let Some(o) = out.as_object_mut()
    {
        o.insert("warning".into(), json!(format!("{blank} page(s) draw nothing")));
    }
    out
}

/// [`report`] on the file at `path`, read back after it was written.
pub fn report_file(path: &str) -> Value {
    let file = std::path::Path::new(path);
    match std::fs::metadata(file) {
        Ok(m) if m.len() > MAX_READ_BYTES => json!({ "error": "the file is too large to read back" }),
        Ok(_) => match std::fs::read(file) {
            Ok(bytes) => report(&bytes),
            Err(e) => json!({ "error": format!("{path}: {e}") }),
        },
        Err(e) => json!({ "error": format!("{path}: {e}") }),
    }
}
