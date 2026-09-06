#!/usr/bin/env sh
# Downloads the Supersonic 2 model (ONNX files + voice styles + LICENSE, ~234 MB)
# from the GitHub release archive and extracts it into ./supersonic2-model
# (or the directory given as $1).
#
#   sh scripts/download-model.sh [destination-dir]
set -eu

DEST="${1:-./supersonic2-model}"
URL="https://github.com/DavidValin/supersonic2-tts/releases/download/1.1.0/supersonic2-model.tgz"

if [ -s "$DEST/onnx/tts.json" ] && [ -s "$DEST/voice_styles/M1.json" ]; then
  echo "exists   $DEST (delete it to download again)"
  echo "Supersonic 2 model ready at $DEST"
  exit 0
fi

TMP="$(mktemp "${TMPDIR:-/tmp}/supersonic2-model.XXXXXX")"
trap 'rm -f "$TMP"' EXIT INT TERM

echo "download $URL"
curl -L --fail --progress-bar -o "$TMP" "$URL"

# The archive contains a single top level folder "supersonic2-model/";
# strip it so the files land directly in $DEST. "._*" are macOS metadata files.
mkdir -p "$DEST"
echo "extract  $DEST"
tar -xzf "$TMP" -C "$DEST" --strip-components=1 --exclude='._*' --exclude='.DS_Store'

echo "Supersonic 2 model ready at $DEST"
echo "The model is licensed under the BigScience Open RAIL-M License, see $DEST/LICENSE"
