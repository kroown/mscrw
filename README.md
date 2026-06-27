# mscrw

metadata scraper · windows · rust

<p align="center">
  <img src="https://skillicons.dev/icons?i=windows,rust">
</p>

scans files recursively, extracts metadata, and can strip it in-place. no external dependencies 

## features

- recursive directory scanning with windows file attributes (hidden, system, readonly)
- image metadata: jpeg (exif), png, gif, bmp, webp
- text analysis: word/line/char counts, encoding detection
- in-place metadata stripping: exif, png ancillary chunks, gif comments, bom/whitespace
- json output with pretty-print
- multi-threaded scanning

## usage

```
mscrw --pretty image.jpg
mscrw --strip -v image.jpg
mscrw --threads 8 --pretty C:\Users\you\Pictures
```

## install

download `mscrw.exe` from the [releases page](https://github.com/kroown/mscrw/releases), or build from source — it auto-installs to `%LOCALAPPDATA%\mscrw\mscrw.exe` and adds itself to your PATH on first run.

## options

| flag | description |
|------|-------------|
| `--pretty` | pretty-print json |
| `--strip` | strip metadata in-place |
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

## build from source

```powershell
git clone https://github.com/kroown/mscrw.git
cd mscrw
cargo build --release
.\target\release\mscrw.exe --help
```


