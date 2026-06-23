use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ImageMeta {
    pub format: String,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_make: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_gps: Option<bool>,
}

fn read_u16_be(data: &[u8], off: usize) -> u16 {
    ((data[off] as u16) << 8) | (data[off + 1] as u16)
}

fn read_u32_be(data: &[u8], off: usize) -> u32 {
    ((data[off] as u32) << 24)
        | ((data[off + 1] as u32) << 16)
        | ((data[off + 2] as u32) << 8)
        | (data[off + 3] as u32)
}

fn read_u16_le(data: &[u8], off: usize) -> u16 {
    data[off] as u16 | ((data[off + 1] as u16) << 8)
}

fn read_u32_le(data: &[u8], off: usize) -> u32 {
    (data[off] as u32)
        | ((data[off + 1] as u32) << 8)
        | ((data[off + 2] as u32) << 16)
        | ((data[off + 3] as u32) << 24)
}

fn parse_jpeg(data: &[u8]) -> Option<ImageMeta> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }

    let mut meta = ImageMeta {
        format: "jpeg".into(),
        width: 0,
        height: 0,
        orientation: None,
        camera_make: None,
        camera_model: None,
        has_gps: None,
    };

    let mut pos: usize = 2;
    while pos + 4 <= data.len() {
        if data[pos] != 0xFF { break; }
        let marker = data[pos + 1];

        if marker == 0xD9 { break; }

        if marker == 0x00 || marker == 0xD0 || marker == 0xD1
            || marker == 0xD2 || marker == 0xD3 || marker == 0xD4
            || marker == 0xD5 || marker == 0xD6 || marker == 0xD7
            || marker == 0xD8
        {
            pos += if marker == 0xD8 { 1 } else { 2 };
            continue;
        }

        if pos + 4 > data.len() { break; }
        let seg_len = read_u16_be(data, pos + 2) as usize;
        if seg_len < 2 || pos + 2 + seg_len > data.len() { break; }

        if marker >= 0xC0 && marker <= 0xC3 && seg_len >= 7 {
            meta.height = read_u16_be(data, pos + 5) as u32;
            meta.width = read_u16_be(data, pos + 7) as u32;
        }

        if marker == 0xE1 && seg_len >= 8 {
            let tag = &data[pos + 4..pos + 4 + 4.min(seg_len.saturating_sub(2))];
            if tag == b"Exif" {
                parse_exif(&data[pos + 4..pos + 2 + seg_len], &mut meta);
            }
        }

        pos += 2 + seg_len;
    }

    if meta.width > 0 && meta.height > 0 { Some(meta) } else { None }
}

fn parse_exif(data: &[u8], meta: &mut ImageMeta) {
    if data.len() < 12 { return; }
    // skip "Exif\0\0" (6 bytes)
    let tiff = if data.starts_with(b"Exif\0\0") { 6 } else { 0 };
    if tiff + 8 > data.len() { return; }

    let endian = &data[tiff..tiff + 2];
    let le = endian == b"II";
    let be = endian == b"MM";
    if !le && !be { return; }

    let r16: fn(&[u8], usize) -> u16 = if le { read_u16_le } else { read_u16_be };
    let r32: fn(&[u8], usize) -> u32 = if le { read_u32_le } else { read_u32_be };

    let ifd0_off = r32(data, tiff + 4) as usize;
    if ifd0_off + 2 > data.len() - tiff { return; }
    let mut ifd = tiff + ifd0_off;
    let entries = r16(data, ifd) as usize;
    ifd += 2;

    for _ in 0..entries {
        if ifd + 12 > data.len() { break; }
        let tag = r16(data, ifd);
        let typ = r16(data, ifd + 2);
        let value_off = ifd + 8;

        let read_str = |off: usize, max: usize| -> Option<String> {
            if off + 2 > data.len() { return None; }
            let end = (off + max).min(data.len());
            let s = std::str::from_utf8(&data[off..end]).ok()?;
            Some(s.trim_end_matches('\0').to_string())
        };

        match (tag, typ) {
            (0x010F, 2) => meta.camera_make = read_str(r32(data, value_off) as usize, 64),
            (0x0110, 2) => meta.camera_model = read_str(r32(data, value_off) as usize, 64),
            (0x0112, 3) => meta.orientation = Some(r16(data, value_off)),
            _ => {}
        }

        if tag == 0x8825 && typ == 4 {
            let gps_off = r32(data, value_off) as usize;
            let gps_ifd = tiff + gps_off;
            if gps_ifd + 2 <= data.len() {
                meta.has_gps = Some(true);
            }
        }

        ifd += 12;
    }
}

fn parse_png(data: &[u8]) -> Option<ImageMeta> {
    let sig = b"\x89PNG\r\n\x1a\n";
    if !data.starts_with(sig) || data.len() < 33 {
        return None;
    }

    let len = read_u32_be(data, 8);
    if len != 13 { return None; }

    let mut chunk_type = [0u8; 4];
    chunk_type.copy_from_slice(&data[12..16]);
    if &chunk_type != b"IHDR" { return None; }

    Some(ImageMeta {
        format: "png".into(),
        width: read_u32_be(data, 16),
        height: read_u32_be(data, 20),
        orientation: None,
        camera_make: None,
        camera_model: None,
        has_gps: None,
    })
}

fn parse_gif(data: &[u8]) -> Option<ImageMeta> {
    if data.len() < 10 { return None; }
    let sig = &data[..6];
    if sig != b"GIF87a" && sig != b"GIF89a" { return None; }

    let width = read_u16_le(data, 6) as u32;
    let height = read_u16_le(data, 8) as u32;
    if width == 0 || height == 0 { return None; }

    Some(ImageMeta {
        format: "gif".into(),
        width,
        height,
        orientation: None,
        camera_make: None,
        camera_model: None,
        has_gps: None,
    })
}

fn parse_bmp(data: &[u8]) -> Option<ImageMeta> {
    if data.len() < 26 { return None; }
    if &data[..2] != b"BM" { return None; }

    let width = read_u32_le(data, 18);
    let height = (read_u32_le(data, 22) as i32).unsigned_abs();
    if width == 0 || height == 0 { return None; }

    Some(ImageMeta {
        format: "bmp".into(),
        width,
        height,
        orientation: None,
        camera_make: None,
        camera_model: None,
        has_gps: None,
    })
}

fn parse_webp(data: &[u8]) -> Option<ImageMeta> {
    if data.len() < 30 { return None; }
    if &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" { return None; }

    let chunk_type = &data[12..16];
    let chunk_size = read_u32_be(data, 4) as usize;
    if chunk_size + 8 > data.len() { return None; }

    match chunk_type {
        b"VP8 " => {
            if data.len() < 26 { return None; }
            let w = read_u16_le(data, 22) & 0x3FFF;
            let h = read_u16_le(data, 24) & 0x3FFF;
            if w == 0 || h == 0 { return None; }
            Some(ImageMeta {
                format: "webp".into(),
                width: w as u32,
                height: h as u32,
                orientation: None,
                camera_make: None,
                camera_model: None,
                has_gps: None,
            })
        }
        b"VP8L" => {
            if data.len() < 25 { return None; }
            let bits = read_u32_le(data, 21);
            let w = (bits & 0x3FFF) + 1;
            let h = ((bits >> 14) & 0x3FFF) + 1;
            Some(ImageMeta {
                format: "webp".into(),
                width: w,
                height: h,
                orientation: None,
                camera_make: None,
                camera_model: None,
                has_gps: None,
            })
        }
        b"VP8X" => {
            if data.len() < 30 { return None; }
            let w = read_u24_le(data, 24) + 1;
            let h = read_u24_le(data, 27) + 1;
            Some(ImageMeta {
                format: "webp".into(),
                width: w,
                height: h,
                orientation: None,
                camera_make: None,
                camera_model: None,
                has_gps: None,
            })
        }
        _ => None,
    }
}

fn read_u24_le(data: &[u8], off: usize) -> u32 {
    data[off] as u32 | ((data[off + 1] as u32) << 8) | ((data[off + 2] as u32) << 16)
}

pub fn parse(data: Option<&[u8]>) -> Option<ImageMeta> {
    let data = match data {
        Some(d) if !d.is_empty() => d,
        _ => return None,
    };

    // sniff by first bytes
    match data.get(0..4) {
        Some(b"\xFF\xD8\xFF\xE0") | Some(b"\xFF\xD8\xFF\xE1") | Some(b"\xFF\xD8\xFF") => parse_jpeg(data),
        Some(b"\x89PNG") => parse_png(data),
        Some(b"GIF8") => parse_gif(data),
        Some(b"RIFF") => parse_webp(data),
        _ => {
            if data.starts_with(b"BM") { parse_bmp(data) }
            else { None }
        }
    }
}
