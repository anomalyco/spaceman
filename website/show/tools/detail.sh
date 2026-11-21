#!/bin/sh
# Contact sheet of chosen moments from a capture clip. usage: detail.sh <film.mp4> <out.jpg> <t> [t ...]
set -e
film=$1; out=$2; shift 2
tmp=$(mktemp -d)
i=0
for t in "$@"; do
  n=$(printf %02d $i)
  ffmpeg -loglevel error -ss "$t" -i "$film" -frames:v 1 -vf "scale=640:-1,drawbox=x=0:y=0:w=64:h=26:color=black@0.6:t=fill" "$tmp/$n.png"
  magick "$tmp/$n.png" -font /System/Library/Fonts/Supplemental/Arial.ttf -fill white -pointsize 15 -annotate +6+18 "$t" "$tmp/$n.png"
  i=$((i + 1))
done
rows=""
set -- "$tmp"/*.png
r=0
while [ $# -gt 0 ]; do
  row="$tmp/row$r.png"
  files=""
  for k in 1 2 3 4; do [ $# -gt 0 ] && files="$files $1" && shift; done
  magick $files -bordercolor '#222' -border 3 +append "$row"
  rows="$rows $row"
  r=$((r + 1))
done
magick $rows -append -quality 85 "$out"
rm -rf "$tmp"
