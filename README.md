thanks: https://github.com/zmwangx/rust-ffmpeg-sys

# Please note! This library is locked to version 7.1 of FFMPEG!

### distributions

windows: https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n7.1-latest-win64-gpl-shared-7.1.zip

linux: https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n7.1-latest-linux64-gpl-shared-7.1.tar.xz

### Custom overwrite FFmpeg directory

```rust
if let Ok(libs_str) = std::env::var("FFMPEG_LINK_LIBS") {
    for lib in libs_str.split(",") {
        println!("cargo:rustc-link-lib={}", lib);
    }
} else {
    println!(
        "cargo:rustc-link-search=all={}",
        find_ffmpeg_prefix(&out_dir)?
    );

    for lib in [
        #[cfg(feature = "avcodec")]
        "avcodec",
        #[cfg(feature = "avdevice")]
        "avdevice",
        #[cfg(feature = "avfilter")]
        "avfilter",
        #[cfg(feature = "avformat")]
        "avformat",
        #[cfg(feature = "avutil")]
        "avutil",
        #[cfg(feature = "swresample")]
        "swresample",
        #[cfg(feature = "swscale")]
        "swscale",
        #[cfg(feature = "postproc")]
        "postproc",
    ] {
        println!("cargo:rustc-link-lib={}", lib);
    }
}
```
