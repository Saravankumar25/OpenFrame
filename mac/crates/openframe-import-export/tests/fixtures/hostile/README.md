# Hostile file collection

Hostile and malformed input files for the screenplay parsers and the archive layer. Every
file in this folder is listed below; `m1_15_corpus_and_readme_list_the_same_cases` in
`tests/hostile_corpus.rs` fails when a file is added without a row or a row names a missing
file.

How the files are used:

- `fdx`, `fountain`, `docx`, `pdf`: each file is imported through `import_file` by
  `tests/hostile_corpus.rs` with the expectation in the table. Every case also checks that
  nothing panics, that nothing is written next to the input and that no file escapes.
- `zip`, `packages`: archive and exchange-package cases. The expected result is the
  outcome of `openframe_security::archive::extract_all`.
- Binary files are written by the ignored test `regenerate_hostile_corpus`
  (`cargo test -p openframe-import-export --test hostile_corpus -- --ignored`). Review the
  changes before committing them. Multi-megabyte inputs are generated at test time instead.

Time bounds (debug build): 10 s per case, 45 s for decompression bombs and generated
multi-megabyte inputs. A case that runs past twice its bound stops the test process with
the case name.

Threats (doc 20): T3 traversal and links, T6 malicious ZIP, T6a hostile documents, T7
oversized input.

## Final Draft (.fdx)

| Case | Threat | Expected result |
|---|---|---|
| `fdx/normal.fdx` | baseline | Parses: two scenes, `&amp;` decoded |
| `fdx/malformed_unclosed.fdx` | T6a | `import.fdx_malformed` |
| `fdx/malformed_mismatched_tags.fdx` | T6a | `import.fdx_malformed` |
| `fdx/malformed_wrong_root.fdx` | T6a | `import.fdx_invalid` |
| `fdx/malformed_binary.fdx` | T6a | `import.fdx_malformed` |
| `fdx/malformed_utf16.fdx` | T6a | `import.fdx_malformed` |
| `fdx/empty_content.fdx` | T6a | `import.fdx_empty` |
| `fdx/entity_billion_laughs.fdx` | T6a | Parses; entities are never expanded |
| `fdx/entity_external_xxe.fdx` | T6a | Parses; the external entity is never resolved |
| `fdx/entity_parameter.fdx` | T6a | Parses; parameter entities are never expanded |
| `fdx/nested_deep.fdx` | T6a, T7 | `import.fdx_malformed` (nesting over the depth limit) |
| `fdx/unicode.fdx` | baseline | Parses; accents, CJK, RTL, emoji, combining marks and bidi controls kept |
| `fdx/control_chars.fdx` | T6a | Parses; control characters never break the structure |

## Fountain

| Case | Threat | Expected result |
|---|---|---|
| `fountain/normal.fountain` | baseline | Parses: title page, two scenes, dialogue |
| `fountain/unicode.fountain` | baseline | Parses; every script and mark kept |
| `fountain/nested_boneyard.fountain` | T6a | Parses; text after the unclosed `/*` stays hidden and is reported |
| `fountain/nested_notes.fountain` | T6a | Parses; one scene |
| `fountain/emphasis_storm.fountain` | T7 | Parses; one scene |
| `fountain/legacy_encoding.fountain` | baseline | Parses as Windows-1252 with a `legacy_encoding` warning |
| `fountain/utf16le_bom.fountain` | baseline | Parses as UTF-16 |
| `fountain/control_chars.fountain` | T6a | Parses; NUL and other control characters in UTF-8 text are kept |
| `fountain/malformed_binary.fountain` | T6a | `import.not_a_screenplay` (binary: not UTF-8 or UTF-16 and contains NUL bytes) |
| `fountain/title_only.fountain` | baseline | Parses: a title page with no scenes is valid Fountain |
| `fountain/empty.fountain` | T6a | `import.empty_source` |
| `fountain/whitespace_only.fountain` | T6a | `import.not_a_screenplay` |

## Word (.docx)

| Case | Threat | Expected result |
|---|---|---|
| `docx/normal.docx` | baseline | Parses: two scenes |
| `docx/unicode_names.docx` | T6 | Parses; extra parts with Unicode and bidi names are ignored |
| `docx/zip_slip.docx` | T3, T6 | Parses; nothing is extracted, traversal names are inert |
| `docx/absolute_paths.docx` | T3, T6 | Parses; nothing is extracted, absolute names are inert |
| `docx/case_collision.docx` | T6 | Parses; only the exact part name `word/document.xml` is read |
| `docx/duplicate_entry.docx` | T6 | `import.unsafe_archive` (two parts with one name: readers would disagree) |
| `docx/symlink_document.docx` | T3 | `import.docx_invalid` (the link text is read as bytes, never followed) |
| `docx/malformed_not_zip.docx` | T6 | `import.unsafe_archive` |
| `docx/malformed_truncated.docx` | T6 | `import.unsafe_archive` |
| `docx/malformed_xml.docx` | T6a | `import.docx_invalid` |
| `docx/missing_document.docx` | T6a | `import.docx_invalid` |
| `docx/nested_deep.docx` | T6a, T7 | `import.docx_invalid` (nesting over the depth limit) |
| `docx/entity_billion_laughs.docx` | T6a | `import.not_a_screenplay`; entities are never expanded |
| `docx/entity_external_xxe.docx` | T6a | `import.not_a_screenplay`; the external entity is never resolved |
| `docx/bomb_ratio.docx` | T6, T7 | `import.unsafe_archive` (inflated part over its cap) |
| `docx/bomb_lying_header.docx` | T6, T7 | `import.unsafe_archive` (real stream counted, the header is not trusted) |
| `docx/directory_flood.docx` | T6, T7 | `import.unsafe_archive` (declared entry count over the limit) |

## PDF

| Case | Threat | Expected result |
|---|---|---|
| `pdf/normal.pdf` | baseline | Parses: two scenes |
| `pdf/malformed_header_only.pdf` | T6a | `import.pdf_unreadable` |
| `pdf/malformed_truncated.pdf` | T6a | `import.pdf_unreadable` |
| `pdf/malformed_no_header.pdf` | T6a | `import.pdf_unreadable` (plain text with a .pdf extension) |
| `pdf/malformed_bad_xref.pdf` | T6a | Parses; the cross-reference table is rebuilt from the objects |
| `pdf/nested_arrays.pdf` | T6a, T7 | `import.too_large` (nesting over the limit, refused before parsing) |
| `pdf/nested_object_stream.pdf` | T6a, T7 | `import.too_large` (nesting inside a compressed object stream) |
| `pdf/bomb_flate.pdf` | T7 | `import.too_large` (stream inflates past its cap) |
| `pdf/bomb_lzw.pdf` | T7 | `import.too_large` (LZW streams are refused) |
| `pdf/cycle_page_tree.pdf` | T6a | `import.pdf_unreadable`; the page-tree loop is cut and a PDF with no page is damaged |
| `pdf/cycle_self_reference.pdf` | T6a | `import.pdf_unreadable` (the catalog refers to itself: no page) |
| `pdf/cycle_xref_prev.pdf` | T6a | Parses its one scene heading; the `/Prev` loop is not followed forever |
| `pdf/huge_length.pdf` | T6a, T7 | `import.pdf_not_screenplay`; the absurd `/Length` is not trusted |
| `pdf/encrypted.pdf` | T6a | `import.pdf_unreadable` |

## ZIP (archive layer)

| Case | Threat | Expected result |
|---|---|---|
| `zip/normal.zip` | baseline | Extracts two files |
| `zip/slip_dotdot.zip` | T3, T6 | `import.unsafe_archive`, nothing written |
| `zip/slip_backslash.zip` | T3, T6 | `import.unsafe_archive`, nothing written |
| `zip/slip_nested.zip` | T3, T6 | `import.unsafe_archive`, nothing written |
| `zip/slip_dot_segments.zip` | T3, T6 | `import.unsafe_archive`, nothing written |
| `zip/abs_unix.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/abs_drive.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/abs_drive_backslash.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/drive_relative.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/unc.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/device_verbatim.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/device_namespace.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/reserved_con.zip` | T6 | `import.unsafe_archive` (Windows device name) |
| `zip/reserved_nul_ext.zip` | T6 | `import.unsafe_archive` (Windows device name with an extension) |
| `zip/reserved_com_superscript.zip` | T6 | `import.unsafe_archive` (`COM¹` is a device name too) |
| `zip/ads_stream.zip` | T6 | `import.unsafe_archive` (NTFS alternate data stream) |
| `zip/trailing_dot.zip` | T6 | `import.unsafe_archive` (Windows drops the dot) |
| `zip/trailing_space.zip` | T6 | `import.unsafe_archive` (Windows drops the space) |
| `zip/empty_name.zip` | T6 | `import.unsafe_archive` |
| `zip/nul_in_name.zip` | T3, T6 | `import.unsafe_archive` |
| `zip/symlink_entry.zip` | T3 | `import.unsafe_archive` (link entries are refused) |
| `zip/symlink_then_write.zip` | T3 | `import.unsafe_archive` (link entries are refused) |
| `zip/case_collision.zip` | T6 | `import.unsafe_archive` (`Readme.txt` and `README.TXT` are one file on Windows) |
| `zip/case_collision_folder.zip` | T6 | `import.unsafe_archive` (`Assets/a.txt` and `assets/A.TXT` are one file) |
| `zip/short_name_alias.zip` | T6 | `import.unsafe_archive` (`CONTEN~1.JSO` is the 8.3 alias of `content.json`) |
| `zip/duplicate_name.zip` | T6 | `import.unsafe_archive` (two entries with one name) |
| `zip/unicode_names.zip` | baseline | Extracts seven files; NFC and NFD names stay separate files |
| `zip/bomb_ratio.zip` | T6, T7 | `import.unsafe_archive` (compression ratio) |
| `zip/bomb_lying_header.zip` | T6, T7 | `import.unsafe_archive`; extraction stops at the declared size |
| `zip/bomb_total_size.zip` | T6, T7 | `import.unsafe_archive` (declared total over the limit) |
| `zip/directory_flood.zip` | T6, T7 | `import.unsafe_archive` (directory count does not match) |
| `zip/nested_zip.zip` | T6 | Extracts three files; the inner archive is stored as a file and never opened |
| `zip/malformed_truncated.zip` | T6 | `import.unsafe_archive` |
| `zip/malformed_garbage.zip` | T6 | `import.unsafe_archive` |

## Exchange packages (.ofstory)

The archive layer refuses the archive-level attacks. The other cases extract and must be
refused by package validation in `openframe-application`; that layer is not yet covered by
a test that uses these files.

| Case | Threat | Expected result |
|---|---|---|
| `packages/slip_dotdot.ofstory` | T3, T6 | `import.unsafe_archive` |
| `packages/abs_drive.ofstory` | T3, T6 | `import.unsafe_archive` |
| `packages/symlink_entry.ofstory` | T3 | `import.unsafe_archive` |
| `packages/case_collision.ofstory` | T6 | `import.unsafe_archive` (`Content.json` collides with `content.json`) |
| `packages/short_name_alias.ofstory` | T6 | `import.unsafe_archive` (8.3 alias) |
| `packages/duplicate_manifest.ofstory` | T6 | `import.unsafe_archive` (two manifests) |
| `packages/bomb_ratio.ofstory` | T6, T7 | `import.unsafe_archive` (compression ratio) |
| `packages/bomb_lying_header.ofstory` | T6, T7 | `import.unsafe_archive`; extraction stops at the declared size |
| `packages/manifest_huge.ofstory` | T7 | `import.unsafe_archive` (compression ratio) |
| `packages/directory_flood.ofstory` | T6, T7 | `import.unsafe_archive` (directory count does not match) |
| `packages/malformed_truncated.ofstory` | T6 | `import.unsafe_archive` |
| `packages/manifest_deep.ofstory` | T6a, T7 | Extracts; package validation must refuse the deeply nested manifest |
| `packages/content_deep.ofstory` | T6a, T7 | Extracts; package validation must refuse the deeply nested content |
| `packages/manifest_traversal_id.ofstory` | T3 | Extracts; package validation must refuse the traversal package ID |
| `packages/checksums_traversal.ofstory` | T3 | Extracts; package validation must refuse the traversal checksum path |
| `packages/nested_package.ofstory` | T6 | Extracts; the inner package is stored as a file and never opened |
| `packages/unicode_names.ofstory` | baseline | Extracts; Unicode asset names kept |
