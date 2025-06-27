# extract-blobs

Extract blobs from a green-screen scanned images and store into multiple images.

## Usage

```
Usage: extract-blobs [OPTIONS] [FILES]...

Arguments:
  [FILES]...  Input image files

Options:
  -c, --chroma-key-color <CHROMA_KEY_COLOR>
          Chroma key color [default: #71AA5D]
  -f, --floodfill-fuzz <FLOODFILL_FUZZ>
          Floodfill fuzz (euclidean distance) [default: 17]
  -t, --trim-edges <TRIM_EDGES>
          Trim edges (pixels) [default: 10]
  -g, --grow-edges <GROW_EDGES>
          Grow edges (pixels) [default: 6]
  -b, --blur-edge-factor <BLUR_EDGE_FACTOR>
          Blur edge factor [default: 2]
  -p, --min-pixels-touching-line <MIN_PIXELS_TOUCHING_LINE>
          Minimum pixels touching detected line [default: 225]
  -l, --max-lines <MAX_LINES>
          Maximum detected lines [default: 4]
  -r, --max-blob-rotation <MAX_BLOB_ROTATION>
          Maximum blob rotation [default: 10]
  -d, --dpi <DPI>
          Output image pixel density in inches [default: 150]
  -i, --ignore-detected-dpi
          Ignore detected DPI in input images
  -s, --save-intermediary-images
          Save intermediary images
  -v, --verbose
          Verbose messages
  -h, --help
          Print help
  -V, --version
          Print version
```

The filenames support glob patterns in them, which enables globbing for more
filenames than your shell supports.

## Installing build dependencies

### Linux

```sh
sudo apt install libleptonica-dev libtesseract-dev clang
```

### Windows

We need the tool [vcpkg](https://github.com/microsoft/vcpkg) to compile all of the external C/C++ dependencies.

We use the tool [cargo-vcpkg](https://crates.io/crates/cargo-vcpkg) to manage the `vcpkg` installation.

The [leptonica-sys](https://crates.io/crates/leptonica-sys) crate requires [LLVM](https://llvm.org)'s `libclang.dll` to properly compile.

#### Install LLVM and make sure you add it to the PATH.

```powershell
winget install --interactive LLVM.LLVM
```

Restart your terminal to ensure LLVM is in the path.

#### Install cargo-vcpkg and use it to build all dependencies:

```powershell
cargo install cargo-vcpkg
cargo vcpkg -v build
```

## Building the release binary

```sh
cargo build --release
```

## Installing the release binary

```sh
cargo install --path .
```

## Install tesseract language support files

### Linux

```sh
# Install any tesseract languages you need
sudo apt install tesseract-ocr-eng tesseract-ocr-nor
```

### All other operating systems

```sh
git clone https://github.com/tesseract-ocr/tessdata_best
```
