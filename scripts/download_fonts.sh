#!/bin/bash
# Download Google Noto fonts for multilingual support

FONTS_DIR="../fonts"
mkdir -p "$FONTS_DIR"

echo "Downloading Noto Sans CJK (Chinese, Japanese, Korean)..."
curl -L "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf" \
  -o "$FONTS_DIR/NotoSansSC-Regular.otf"

echo "Downloading Noto Sans Arabic..."
curl -L "https://github.com/notofonts/noto-fonts/raw/main/hinted/ttf/NotoSansArabic/NotoSansArabic-Regular.ttf" \
  -o "$FONTS_DIR/NotoSansArabic-Regular.ttf"

echo "Converting OTF to TTF if needed..."
# For CJK, we'll use a subset version that's more reasonable in size
curl -L "https://github.com/googlefonts/noto-cjk/raw/main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf" \
  -o "$FONTS_DIR/NotoSansSC-Regular.otf"

echo "Font download complete!"
echo "Note: OTF fonts work with most systems. If you need TTF, please convert manually."

