<p align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=auto&height=200&section=header&text=mcsrw&fontSize=70&fontAlignY=35&animation=twinkling">
</p>

<p align="center">
  <b>mcsrw</b><br>
  metadata scraper · windows · rust
</p>

<p align="center">
  <img src="https://skillicons.dev/icons?i=windows,rust">
</p>

scans files recursively, extracts metadata, and can strip it in-place. no external dependencies — just the binary.

## features

- recursive directory scanning with windows file attributes (hidden, system, readonly)
- image metadata: jpeg (exif), png, gif, bmp, webp
- text analysis: word/line/char counts, encoding detection
- in-place metadata stripping: exif, png ancillary chunks, gif comments, bom/whitespace
- json output with pretty-print
- multi-threaded scanning

## usage

```
mcsrw --pretty image.jpg
mcsrw --strip -v image.jpg
mcsrw --threads 8 --pretty C:\Users\you\Pictures
```

## install

```
mcsrw --install
```

copies itself to `%LOCALAPPDATA%\mcsrw\mcsrw.exe` (windows) or `~/.local/bin/mcsrw` (linux) and adds it to your PATH automatically.

## options

| flag | description |
|------|-------------|
| `--pretty` | pretty-print json |
| `--strip` | strip metadata in-place |
| `--install` | install to path automatically |
| `-v`, `--verbose` | verbose output |
| `-t`, `--threads` | worker threads |
| `--help` | show help |

## example output

```json
{
  "path": "C:\\Users\\user\\photo.jpg",
  "name": "photo.jpg",
  "extension": ".jpg",
  "size": 421356,
  "is_hidden": false,
  "is_system": false,
  "is_readonly": false,
  "image": {
    "format": "jpeg",
    "width": 4032,
    "height": 3024,
    "orientation": 1,
    "camera_make": "Apple",
    "camera_model": "iPhone 15 Pro",
    "has_gps": true
  }
}
```

## install

download `mcsrw.exe` from the [releases page](https://github.com/kroown/mcsrw/releases).

## build from source

```powershell
git clone https://github.com/kroown/mcsrw.git
cd mcsrw
cargo build --release
.\target\release\mcsrw.exe --help
```

<p align="center">
  <img src="https://capsule-render.vercel.app/api?type=waving&color=auto&height=100&section=footer">
</p>
