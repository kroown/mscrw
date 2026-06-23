use std::path::Path;

fn read_u16_be(data: &[u8], off: usize) -> u16 {
    ((data[off] as u16) << 8) | (data[off + 1] as u16)
}

fn read_u32_be(data: &[u8], off: usize) -> u32 {
    ((data[off] as u32) << 24)
        | ((data[off + 1] as u32) << 16)
        | ((data[off + 2] as u32) << 8)
        | (data[off + 3] as u32)
}

fn strip_jpeg(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..2]);

    let mut pos = 2usize;
    let mut changed = false;

    while pos + 2 <= data.len() {
        if data[pos] != 0xFF { break; }
        let marker = data[pos + 1];

        if marker == 0xD9 {
            out.extend_from_slice(&data[pos..pos + 2]);
            break;
        }

        if marker == 0x00 || marker == 0xD0 || marker == 0xD1
            || marker == 0xD2 || marker == 0xD3 || marker == 0xD4
            || marker == 0xD5 || marker == 0xD6 || marker == 0xD7
            || marker == 0xD8
        {
            out.extend_from_slice(&data[pos..pos + 2]);
            pos += if marker == 0xD8 { 1 } else { 2 };
            continue;
        }

        if pos + 4 > data.len() { break; }
        let seg_len = read_u16_be(data, pos + 2) as usize;
        if seg_len < 2 || pos + 2 + seg_len > data.len() { break; }

        let keep = matches!(marker,
            0xE0 | 0xC0 | 0xC1 | 0xC2 | 0xC3 | 0xC4 | 0xDA | 0xDB | 0xDD
        );

        if keep {
            out.extend_from_slice(&data[pos..pos + 2 + seg_len]);
        } else {
            changed = true;
        }

        pos += 2 + seg_len;

        if marker == 0xDA {
            if let Some(eoi) = data[pos..].windows(2).position(|w| w == [0xFF, 0xD9]) {
                out.extend_from_slice(&data[pos..pos + eoi]);
                out.extend_from_slice(&[0xFF, 0xD9]);
            } else {
                out.extend_from_slice(&data[pos..]);
            }
            break;
        }
    }

    if changed { Some(out) } else { None }
}

fn strip_png(data: &[u8]) -> Option<Vec<u8>> {
    let sig = b"\x89PNG\r\n\x1a\n";
    if !data.starts_with(sig) || data.len() < 8 {
        return None;
    }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(sig);

    let mut pos = 8usize;
    let mut changed = false;

    while pos + 12 <= data.len() {
        let len = read_u32_be(data, pos) as usize;
        let mut chunk_type = [0u8; 4];
        chunk_type.copy_from_slice(&data[pos + 4..pos + 8]);
        let chunk_end = pos + 12 + len;
        if chunk_end > data.len() { break; }

        let keep = match &chunk_type {
            b"IHDR" | b"PLTE" | b"IDAT" | b"IEND" => true,
            _ => false,
        };

        if keep {
            out.extend_from_slice(&data[pos..chunk_end]);
        } else {
            changed = true;
        }

        pos = chunk_end;
        if &chunk_type == b"IEND" { break; }
    }

    if changed { Some(out) } else { None }
}

fn strip_gif(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 6 { return None; }
    if &data[..6] != b"GIF87a" && &data[..6] != b"GIF89a" { return None; }

    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..13]);

    let packed = data[10];
    let gct_size = if packed & 0x80 != 0 {
        3 * (1 << ((packed & 0x07) + 1)) as usize
    } else { 0 };

    let end = 13 + gct_size;
    if end > data.len() { return None; }
    out.extend_from_slice(&data[13..end]);
    let mut pos = end;
    let mut changed = false;

    while pos < data.len() {
        match data[pos] {
            0x2C => {
                // image descriptor
                let img_start = pos;
                pos += 1;
                if pos + 9 > data.len() { break; }
                out.extend_from_slice(&data[img_start..pos + 9]);
                let lct_packed = data[pos + 8];
                let lct_size = if lct_packed & 0x80 != 0 {
                    3 * (1 << ((lct_packed & 0x07) + 1)) as usize
                } else { 0 };
                if pos + 9 + lct_size > data.len() { break; }
                out.extend_from_slice(&data[pos + 9..pos + 9 + lct_size]);
                pos += 9 + lct_size;
                if pos >= data.len() { break; }
                out.push(data[pos]);
                pos += 1;
                while pos < data.len() {
                    let block_size = data[pos] as usize;
                    out.push(data[pos]);
                    pos += 1;
                    if block_size == 0 { break; }
                    if pos + block_size > data.len() { break; }
                    out.extend_from_slice(&data[pos..pos + block_size]);
                    pos += block_size;
                }
            }
            0x21 => {
                // extension
                if pos + 2 > data.len() { break; }
                let label = data[pos + 1];
                if label == 0xF9 {
                    // keep graphics control
                    let ext_start = pos;
                    pos += 2;
                    if pos >= data.len() { break; }
                    let block_size = data[pos] as usize;
                    let mut ext_end = pos + 1 + block_size;
                    while ext_end < data.len() && data[ext_end] != 0 {
                        ext_end += 1 + data[ext_end] as usize;
                    }
                    ext_end += 1;
                    out.extend_from_slice(&data[ext_start..ext_end]);
                    pos = ext_end;
                } else {
                    changed = true;
                    pos += 2;
                    while pos < data.len() {
                        let block_size = data[pos] as usize;
                        pos += 1;
                        if block_size == 0 { break; }
                        pos += block_size;
                    }
                }
            }
            0x3B => { out.push(0x3B); break; }
            _ => { break; }
        }
    }

    if changed { Some(out) } else { None }
}

fn strip_text(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut pos = 0;
    let mut changed = false;

    // strip BOM
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        pos = 3;
        changed = true;
    }

    let mut prev_cr = false;
    for &b in data[pos..].iter() {
        if b == b'\r' {
            out.push(b'\n');
            prev_cr = true;
            changed = true;
        } else if b == b'\n' {
            if !prev_cr {
                out.push(b'\n');
            }
            prev_cr = false;
        } else {
            out.push(b);
            prev_cr = false;
        }
    }

    if changed { Some(out) } else { None }
}

fn write_file(path: &Path, data: &[u8]) -> bool {
    use std::io::Write;
    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(data))
        .is_ok()
}

pub fn strip_file(path: &Path, ext: &str) -> bool {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let original_len = data.len();

    let result = match ext {
        ".jpg" | ".jpeg" => strip_jpeg(&data),
        ".png" => strip_png(&data),
        ".gif" => strip_gif(&data),
        ".txt" | ".md" | ".csv" | ".xml" | ".html" | ".htm"
        | ".json" | ".yaml" | ".yml" | ".toml" | ".ini"
        | ".cfg" | ".conf" | ".sh" | ".py" | ".js"
        | ".css" | ".cpp" | ".c" | ".hpp" | ".h"
        | ".rs" | ".go" | ".java" => strip_text(&data),
        _ => return false,
    };

    match result {
        Some(new_data) => {
            if new_data.len() == original_len && new_data == data {
                return false;
            }
            write_file(path, &new_data)
        }
        None => false,
    }
}
