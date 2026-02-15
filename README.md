# DD Merge

A high-performance Git GUI client built with Rust and GPUI, modeled after Sublime Merge.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- macOS (currently the only supported platform)

## Development

Run the app in debug mode:

```bash
cargo run -p dd_merge
```

## Building the macOS App Bundle

To create a distributable `DD Merge.app`:

1. Install [librsvg](https://formulae.brew.sh/formula/librsvg) (needed for icon generation):

   ```bash
   brew install librsvg
   ```

2. Run the bundle script:

   ```bash
   ./scripts/bundle-macos.sh
   ```

   This will:
   - Generate the app icon from `assets/icon.svg` (SVG → .icns)
   - Build the release binary (`cargo build --release`)
   - Assemble the `.app` bundle at `target/release/DD Merge.app/`

3. Launch the app:

   ```bash
   open "target/release/DD Merge.app"
   ```
