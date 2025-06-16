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
