#!/usr/bin/env bash
set -euo pipefail

BASE="${1:-./retromount-testdata}"

echo "Creating test dataset at: $BASE"
rm -rf "$BASE"
mkdir -p "$BASE"

# ----------------------------
# 1. Simple ROM directory
# ----------------------------
mkdir -p "$BASE/roms/snes"

echo "fake rom data" > "$BASE/roms/snes/Super Mario World.sfc"
echo "fake rom data" > "$BASE/roms/snes/Zelda.sfc"

# junk files
echo "Some release notes" > "$BASE/roms/snes/game.nfo"
echo "cover image" > "$BASE/roms/snes/cover.jpg"

# ----------------------------
# 2. ZIP archive with ROMs
# ----------------------------
mkdir -p "$BASE/zips/tmp"

echo "rom1" > "$BASE/zips/tmp/game1.gb"
echo "rom2" > "$BASE/zips/tmp/game2.gb"
echo "readme" > "$BASE/zips/tmp/readme.txt"

(
  cd "$BASE/zips/tmp"
  zip -q ../gameboy_collection.zip *
)

rm -rf "$BASE/zips/tmp"

# ----------------------------
# 3. Single-disc CUE/BIN
# ----------------------------
mkdir -p "$BASE/discs/ps1_single"

echo "binarydata" > "$BASE/discs/ps1_single/game.bin"

cat > "$BASE/discs/ps1_single/game.cue" <<EOF
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
EOF

# ----------------------------
# 4. Multi-disc set
# ----------------------------
mkdir -p "$BASE/discs/ps1_multi"

for i in 1 2; do
  echo "disc$i" > "$BASE/discs/ps1_multi/game_disc${i}.bin"

  cat > "$BASE/discs/ps1_multi/game_disc${i}.cue" <<EOF
FILE "game_disc${i}.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
EOF
done

# ----------------------------
# 5. ZIP containing CUE/BIN
# ----------------------------
mkdir -p "$BASE/zips_ps1/tmp"

echo "discdata" > "$BASE/zips_ps1/tmp/game.bin"

cat > "$BASE/zips_ps1/tmp/game.cue" <<EOF
FILE "game.bin" BINARY
  TRACK 01 MODE2/2352
    INDEX 01 00:00:00
EOF

(
  cd "$BASE/zips_ps1/tmp"
  zip -q ../ps1_game.zip *
)

rm -rf "$BASE/zips_ps1/tmp"

# ----------------------------
# 6. Mixed messy directory
# ----------------------------
mkdir -p "$BASE/mixed"

echo "rom" > "$BASE/mixed/game1.nes"
echo "random text" > "$BASE/mixed/notes.txt"
echo "image" > "$BASE/mixed/screenshot.png"

# nested zip
mkdir -p "$BASE/mixed/tmp"
echo "nested rom" > "$BASE/mixed/tmp/nested.gb"
(
  cd "$BASE/mixed/tmp"
  zip -q ../nested.zip *
)
rm -rf "$BASE/mixed/tmp"

# ----------------------------
# Done
# ----------------------------
echo "✅ Test dataset created."
echo ""
echo "Try:"
echo "  cargo run -- inspect $BASE"
