# Installing FFmpeg

Scout requires FFmpeg to be installed on your system for video support.

## Windows

### Option 1: Chocolatey (Recommended)
```powershell
choco install ffmpeg
```

### Option 2: Manual Installation
1. Download FFmpeg from [ffmpeg.org](https://ffmpeg.org/download.html#build-windows)
2. Extract to `C:\ffmpeg`
3. Add `C:\ffmpeg\bin` to your PATH:
   - Press `Win + X` → System → Advanced system settings
   - Click "Environment Variables"
   - Under "System variables", find `Path`, click "Edit"
   - Click "New" and add: `C:\ffmpeg\bin`
   - Click "OK" on all dialogs
4. Verify: Open new terminal and run `ffmpeg -version`

## macOS

### Homebrew
```bash
brew install ffmpeg
```

## Linux

### Ubuntu/Debian
```bash
sudo apt update
sudo apt install ffmpeg
```

### Fedora
```bash
sudo dnf install ffmpeg
```

### Arch
```bash
sudo pacman -S ffmpeg
```

## Verifying Installation

After installation, verify FFmpeg is available:

```bash
ffmpeg -version
```

You should see output showing FFmpeg version 6.x, 7.x, or 8.x.

## Troubleshooting

**"FFmpeg not found" error when running Scout:**
- Make sure FFmpeg is in your PATH
- Restart your terminal after installation
- On Windows, you may need to reboot

**Video processing fails:**
- Ensure FFmpeg version is 6.0 or newer
- Check that video codecs are supported: `ffmpeg -codecs`
