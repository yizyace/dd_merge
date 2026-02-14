# Sublime Merge Tech Stack Research

## CONFIRMED: C++ with Skia (Option #1)

Sublime Merge is **definitively a C++ application with a custom GUI toolkit built on Skia + OpenGL**.
This has been validated through binary analysis, third-party attributions, forum posts, and multiple independent sources.

---

## Core Language: C++ (CONFIRMED)

**Evidence:**
- Binary is a native **Mach-O universal binary** (x86_64 + arm64) — no Electron, no JVM, no managed runtime
- Links against **LLVM C++ Standard Library** (listed in attribution.txt)
- Uses **Boost** (C++ library) — confirmed in attribution and binary strings
- C++ mangled symbols visible in binary (e.g., `N4sgit23allocated_file_contentsE` → `sgit::allocated_file_contents`)
- Source paths embedded in binary: `third_party/skia/...`, `third_party/crashpad/...` — typical C++ project
- Sublime Forum confirms: "Sublime Text is written in C++" with embedded Python for plugins
- Jon Skinner (creator) left Google to build it — consistent with a C++ systems programmer background

**Sources:**
- [Sublime Forum: Frameworks and Technology](https://forum.sublimetext.com/t/sublime-text-frameworks-and-technology-used/14760)
- [Disassembling Sublime Text (Tristan Hume)](https://thume.ca/2016/12/03/disassembling-sublime-text/)

---

## Rendering: Skia + OpenGL (CONFIRMED)

**Evidence:**
- **Skia** paths embedded in binary: `third_party/skia/src/core/SkDraw.cpp`, `third_party/skia/include/core/SkBitmap.h`, etc.
- Skia listed in attribution.txt
- Jon Skinner confirmed on HN: "SublimeText uses Skia Graphics Library to draw the entire user interface"
- HN commenter confirms: "Sublime uses skia as its rendering library, which is what blink (chromium frontend) uses for rendering"
- **OpenGL 4.1** for GPU-accelerated rendering — official blog: "it's the only truly cross-platform GPU API"
- Skia used for **CPU-based rendering** as fallback; OpenGL for hardware acceleration
- Binary contains fallback logic: `"detected macOS 10.14.4: disabling OpenGL rendering"`
- OpenGL integration was "just under 9000 lines of code"
- **Earcut** library for polygon triangulation (GPU rendering of complex shapes)

**Rendering Pipeline:**
- Abstraction called **"render context"** — most widgets use basic primitives from it
- Text rendering uses glyph batching (improved from 52ms to 3ms with batching)
- Gradients, squiggly underlines, text fading all routed through render context

**Sources:**
- [Sublime HQ Blog: Hardware Accelerated Rendering](https://www.sublimetext.com/blog/articles/hardware-accelerated-rendering)
- [HN: Skia confirmation](https://news.ycombinator.com/item?id=14112426)
- [Disassembling Sublime Text](https://thume.ca/2016/12/03/disassembling-sublime-text/)

---

## Custom GUI Framework: "px" + "skyline" (CONFIRMED)

Sublime does NOT use Qt, GTK, or any off-the-shelf GUI toolkit for its main UI. They built two custom frameworks:

- **px** — windowing and platform integration framework for event handling, file management, and OS integration across Windows, Linux, and macOS
- **skyline** — widgets framework; centerpiece is `skyline_text_control`
- `PXApplication` is the NSPrincipalClass in Info.plist (confirms "px" framework)
- Themes use "PNGs and a custom CSS-like JSON structure" — no native widgets

**Platform-specific native layers (thin wrappers only):**
- macOS: Cocoa/AppKit (windows, menus, events), CoreText (text shaping), CoreGraphics, QuartzCore
- Linux: GTK (for file dialogs, menus only), X11 for display
- Windows: Win32 API (USER32, GDI32)

**Sources:**
- [Sublime Forum: The Custom UI](https://forum.sublimetext.com/t/the-custom-ui/14860)
- [Disassembling Sublime Text](https://thume.ca/2016/12/03/disassembling-sublime-text/)
- Info.plist: `NSPrincipalClass = PXApplication`

---

## Git: Custom "sgit" Library + libgit2 + git binary (CLARIFIED)

The git story is more nuanced than initially thought:

- **Custom C++ git reading library** — namespace `sgit` (visible in mangled symbols: `sgit::allocated_file_contents`, `sgit_repository_source`)
- **libgit2** listed in attribution.txt — likely used as foundation/borrowed code for their custom implementation
- **Read operations**: handled by custom `sgit` library for maximum performance
- **Write/mutating operations** (staging, committing, checkout): delegate to the actual **git binary** for correctness and compatibility
- No libgit2 dynamic linking detected (not in `otool -L` output) — it's statically linked or code was incorporated

**Sources:**
- Binary symbol analysis: `N4sgit23allocated_file_contentsE`
- attribution.txt lists libgit2
- [sublimemerge.com](https://www.sublimemerge.com/): "custom high-performance Git reading library"

---

## Other Key Components (from attribution.txt + binary analysis)

| Library | Purpose | Notes |
|---------|---------|-------|
| **Skia** | 2D rendering engine | Confirmed via binary + Jon Skinner |
| **libgit2** | Git operations (base) | Incorporated into custom `sgit` |
| **Boost** | C++ utilities, regex | Confirmed in binary strings |
| **OpenSSL** | Crypto/TLS | For remote git operations |
| **LevelDB** | Key-value storage | Symbol indexing, caching |
| **SQLite3** | Database storage | |
| **Oniguruma** | Regex engine | Syntax highlighting fallback |
| **sregex** | Custom regex engine | Searches multiple regexes simultaneously |
| **Python** | Plugin/extension scripting | Embedded interpreter |
| **Hunspell** | Spell checking | |
| **Crashpad** | Crash reporting | From Chromium project |
| **jemalloc** | Memory allocator | Performance optimization |
| **Google densehash** | Fast hash maps | Used extensively |
| **libpng / libwebp / stb_image** | Image handling | Icons, themes |
| **RapidXml / rapidyaml** | Config parsing | |
| **lz4 / zlib-ng / XZ** | Compression | |
| **Earcut** | Polygon triangulation | GPU rendering |
| **LLVM libc++** | C++ standard library | |
| **bsdiff/bspatch** | Binary diffing | Auto-update system |
| **LibTomCrypt / LibTomMath** | Cryptography | Licensing/verification |

---

## Architecture Summary

```
┌─────────────────────────────────────────────────┐
│              Sublime Merge (C++)                │
├─────────────────────────────────────────────────┤
│  Plugin Layer: Embedded Python interpreter      │
├─────────────────────────────────────────────────┤
│  UI Framework: "skyline" (custom widgets)       │
│  - skyline_text_control (text editing)          │
│  - CSS-like JSON theming                        │
├─────────────────────────────────────────────────┤
│  Rendering: Skia (CPU) + OpenGL 4.1 (GPU)      │
│  - Render context abstraction                   │
│  - Glyph batching for text                      │
│  - Earcut for polygon triangulation             │
├─────────────────────────────────────────────────┤
│  Platform: "px" framework                       │
│  - macOS: Cocoa/AppKit + CoreText               │
│  - Linux: X11 + GTK (dialogs only)             │
│  - Windows: Win32 API                           │
├─────────────────────────────────────────────────┤
│  Git: custom "sgit" (reads) + git binary (writes)│
│  Storage: LevelDB + SQLite3                     │
│  Text: Oniguruma + sregex (syntax highlighting) │
│  Crypto: OpenSSL + LibTomCrypt                  │
└─────────────────────────────────────────────────┘
```

---

## Potential Approaches for Building Something Similar

1. **C++ with Skia** (closest match) — very fast, very hard to build. This is exactly what Sublime does.
2. **Rust + custom GPU rendering** (e.g., Zed editor's GPUI) — modern take on same approach
3. **C++ or Rust + Qt** — cross-platform native GUI framework, less custom work
4. **Tauri (Rust + web frontend)** — lighter than Electron, still uses webview for UI
5. **Flutter** (uses Skia under the hood) — cross-platform with Dart, easier than raw Skia
