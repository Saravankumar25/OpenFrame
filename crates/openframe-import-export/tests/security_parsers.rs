//! Regression tests for the 2026-09-30 security review: hostile XML (PARSE-01) and PDF
//! (PARSE-02) input must fail fast and safely — never hang, exhaust memory or overflow
//! the stack.

use std::io::Write;
use std::time::{Duration, Instant};

use openframe_import_export::{SourceFormat, import_bytes, pdf_import, xml};

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn stream_obj(num: u32, dict_extra: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{num} 0 obj\n<< /Length {} {dict_extra} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

fn pdf_with(body: &[u8]) -> Vec<u8> {
    let mut v = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    v.extend_from_slice(body);
    v.extend_from_slice(b"trailer\n<< /Size 3 >>\n%%EOF\n");
    v
}

fn code(r: openframe_domain::AppResult<()>) -> String {
    r.unwrap_err().code_str().to_string()
}

#[test]
fn xml_attribute_flood_fails_fast() {
    // One tag with 200k attributes: quadratic duplicate checking used to hang here.
    let mut doc = String::from("<?xml version=\"1.0\"?><w:document><w:p ");
    for i in 0..200_000 {
        doc.push_str(&format!("a{i}=\"\" "));
    }
    doc.push_str("/></w:document>");
    let started = Instant::now();
    assert!(xml::parse(doc.as_bytes()).is_err());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    // A normal element with a few attributes still parses.
    let ok = xml::parse(br#"<a x="1" y="2" z="3"><b w:val="q"/></a>"#).unwrap();
    assert_eq!(ok.attr("y"), Some("2"));
}

#[test]
fn fdx_with_attribute_flood_is_rejected_not_hung() {
    let mut doc = String::from(
        "<?xml version=\"1.0\"?><FinalDraft DocumentType=\"Script\"><Content><Paragraph ",
    );
    for i in 0..150_000 {
        doc.push_str(&format!("k{i}=\"v\" "));
    }
    doc.push_str("Type=\"Action\"><Text>Hi</Text></Paragraph></Content></FinalDraft>");
    let started = Instant::now();
    assert!(import_bytes(doc.as_bytes(), SourceFormat::Fdx).is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn pdf_deep_nesting_is_rejected_before_lopdf() {
    let body = format!(
        "1 0 obj\n{}{}\nendobj\n",
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    let pdf = pdf_with(body.as_bytes());
    assert_eq!(code(pdf_import::preflight(&pdf)), "import.too_large");
    // Through the public entry point too (would overflow lopdf's recursive parser).
    assert_eq!(
        import_bytes(&pdf, SourceFormat::Pdf)
            .unwrap_err()
            .code_str(),
        "import.too_large"
    );
    let dicts = format!(
        "1 0 obj\n{}{}\nendobj\n",
        "<<".repeat(200),
        ">>".repeat(200)
    );
    assert_eq!(
        code(pdf_import::preflight(&pdf_with(dicts.as_bytes()))),
        "import.too_large"
    );
}

#[test]
fn pdf_decompression_bomb_is_rejected() {
    let bomb = zlib(&vec![
        0u8;
        (pdf_import::MAX_STREAM_INFLATED + (1 << 20)) as usize
    ]);
    assert!(bomb.len() < 2 << 20, "the bomb itself is small");
    let pdf = pdf_with(&stream_obj(1, "/Filter /FlateDecode", &bomb));
    let started = Instant::now();
    assert_eq!(code(pdf_import::preflight(&pdf)), "import.too_large");
    assert!(started.elapsed() < Duration::from_secs(20));
    // Many medium streams that exceed the total budget together.
    let medium = zlib(&vec![b' '; 40 << 20]);
    let mut body = Vec::new();
    for i in 0..8 {
        body.extend(stream_obj(i + 1, "/Filter /FlateDecode", &medium));
    }
    assert_eq!(
        code(pdf_import::preflight(&pdf_with(&body))),
        "import.too_large"
    );
}

#[test]
fn pdf_unbounded_filters_are_refused() {
    let lzw = pdf_with(&stream_obj(1, "/Filter /LZWDecode", b"\x80\x0b\x60\x50"));
    assert_eq!(code(pdf_import::preflight(&lzw)), "import.too_large");
    let brotli = pdf_with(&stream_obj(1, "/Filter /BrotliDecode", b"\x0b\x02\x80"));
    assert_eq!(code(pdf_import::preflight(&brotli)), "import.too_large");
    let hex = pdf_with(&stream_obj(
        1,
        "/Filter [/ASCIIHexDecode /FlateDecode]",
        b"789c>",
    ));
    assert_eq!(code(pdf_import::preflight(&hex)), "import.too_large");
    let chained = pdf_with(&stream_obj(
        1,
        "/Filter [/ASCII85Decode /FlateDecode]",
        b"<~9jqo^~>",
    ));
    assert_eq!(code(pdf_import::preflight(&chained)), "import.too_large");
}

#[test]
fn pdf_object_stream_nesting_is_screened_after_inflating() {
    let inner = format!("{}{}", "[".repeat(5_000), "]".repeat(5_000));
    let data = zlib(inner.as_bytes());
    let pdf = pdf_with(&stream_obj(
        1,
        "/Type /ObjStm /N 1 /First 0 /Filter /FlateDecode",
        &data,
    ));
    assert_eq!(code(pdf_import::preflight(&pdf)), "import.too_large");
}

#[test]
fn pdf_preflight_accepts_ordinary_structure() {
    let content = zlib(b"BT /F1 12 Tf 72 700 Td (INT. HOUSE - DAY) Tj ET");
    let mut body = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec();
    body.extend(
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    body.extend(stream_obj(4, "/Filter /FlateDecode", &content));
    // Binary stream data that happens to contain brackets and "stream" must not confuse it.
    body.extend(stream_obj(
        5,
        "/Filter /DCTDecode",
        b"[[[[[[ ((( stream <<<<",
    ));
    assert!(pdf_import::preflight(&pdf_with(&body)).is_ok());
}

#[test]
fn pdf_page_content_amplification_is_bounded() {
    use lopdf::{Document, Object, Stream, dictionary};
    // One ~1 MiB content stream referenced by the page 1,500 times: lopdf's own
    // get_page_content would concatenate ~1.5 GiB. OpenFrame refuses the page instead.
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut text = Vec::new();
    while text.len() < (1 << 20) {
        text.extend_from_slice(b"BT /F1 12 Tf 72 700 Td (INT. HOUSE - DAY) Tj ET\n");
    }
    let mut stream = Stream::new(dictionary! {}, text);
    stream.compress().unwrap();
    let content_id = doc.add_object(stream);
    let refs: Vec<Object> = (0..1_500).map(|_| Object::Reference(content_id)).collect();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Contents" => refs,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let started = Instant::now();
    let r = import_bytes(&bytes, SourceFormat::Pdf);
    assert!(r.is_err(), "the amplified page is not read");
    assert!(started.elapsed() < Duration::from_secs(30));
}
