# Inputs

    # $1 = input file
    # $2 = chroma color (#AARRGGBB), my default is #72B34B
    # $3 = fuzz (percent, integer), my default is 11
    # $4 = resolution (DPI, integer), my default is 150
    # Usually executed like this:
    # ls -1 *.jpg | sort | while read name;  do ~/src/public/extract-images/extract_blobs.sh "$name" "#72B34B" 11 150; done

# Removing background and cleaning up dust particles

    convert "$1" \
    -bordercolor "$2" -border 20x20 \
    -alpha set -channel RGBA \
    -fuzz "${3}%" -fill transparent -floodfill +0+0 "$2" \
    -channel A \
    -morphology Erode Disk \
    -adaptive-blur 0x3 \
    -morphology Dilate Disk \
    -morphology Dilate Disk \
    -morphology Dilate Disk \
    -morphology Erode Disk \
    -morphology Erode Disk \
    -channel RGBA \
    "$outfile"

# Extract alpha channel

Extract alpha channel from output image and save as mask, discarding 80%
of the blurred region (towards white), giving us a two-color image where
the fuzzy edge is mostly part of background

    convert "$outfile" -alpha extract -threshold '80%' -depth 2 "$mask"

# Disable alpha channel in extracted image

    bgcolor='gray(185)'
    convert "$outfile" -background "$bgcolor" -alpha remove "$outfile"


# Discover blobs in mask and save into a text file with geometry

    convert "$mask" \
    -define connected-components:verbose=true \
    -define connected-components:mean-color=true \
    -define connected-components:area-threshold=100 \
    -connected-components 4 \
    null: >"$components"

# Save each blob in image into its own file

Save each blob into its own file, using geometry extracted from mask.
Straighten and trim excess borders

    cat "$components" |\
    perl -lane 'chop $F[0]; print join(" ", $F[0], $F[1]) if $F[4] eq "gray(255)";' |\
    while read blob_index blob_geometry; \
    do
        out_index="$dir/$base-${blob_index}.png";
        echo "$out_index: Extracting blob ${blob_index} from region ${blob_geometry}..."
        convert "$outfile" \
        -background "$bgcolor" \
        -crop "$blob_geometry" \
        -deskew '40%' \
        -border 1 -fuzz '15%' -trim \
        +repage \
        -units PixelsPerInch -density "$4" \
        "$out_index"
    done

# Removing temporary files

    rm "$mask" "$components" "$outfile"
