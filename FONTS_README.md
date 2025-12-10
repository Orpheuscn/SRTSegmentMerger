# Font Support for Multilingual Display

## Default Font Support

The application uses **egui's default system fonts**, which provide excellent support for multiple languages including:

- **Chinese** (Simplified and Traditional)
- **Japanese** (Hiragana, Katakana, Kanji)
- **Korean** (Hangul)
- **Arabic**
- **Cyrillic** (Russian, etc.)
- **Latin** scripts (English, French, German, Spanish, etc.)

**No additional font installation is required.** The system fonts on macOS, Windows, and Linux should handle all these languages correctly.

## Testing Multilingual Support

When you run the application:
1. The Whisper output log will display in the right panel
2. Recognized text in various languages should render correctly
3. If you see boxes (□) instead of characters, it means your system is missing fonts for that language

## Troubleshooting

If you encounter display issues:

1. **macOS**: System fonts should work out of the box
2. **Windows**: Install language packs if needed via Settings > Language
3. **Linux**: Install fonts packages:
   ```bash
   # Ubuntu/Debian
   sudo apt install fonts-noto-cjk fonts-noto-cjk-extra
   
   # Fedora
   sudo dnf install google-noto-cjk-fonts
   ```

## Custom Fonts (Advanced)

If you want to bundle custom fonts with the application, you can:
1. Add font files to the `fonts/` directory
2. Modify `src/main.rs` to load them using `include_bytes!`
3. Rebuild the application

However, this is typically not necessary as system fonts work well.

