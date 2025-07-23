thanks: https://github.com/zmwangx/rust-ffmpeg-sys

### distributions

windows: https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n7.1-latest-win64-gpl-shared-7.1.zip

linux: https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n7.1-latest-linux64-gpl-shared-7.1.tar.xz

### Custom overwrite FFmpeg directory

```rust
if std::env::var("USE_CUSTOM_FFMPEG_PREFIX").is_err() {
    println!(
        "cargo:rustc-link-search=all={}",
        find_ffmpeg_prefix(&out_dir)?
    );
}

for lib in [
    #[cfg(feature = "avcodec")]
    std::env::var("LIBAVCODEC_PATH").unwrap_or_else(|_| "avcodec".to_string()),
    #[cfg(feature = "avdevice")]
    std::env::var("LIBAVDEVICE_PATH").unwrap_or_else(|_| "avdevice".to_string()),
    #[cfg(feature = "avfilter")]
    std::env::var("LIBAVFILTER_PATH").unwrap_or_else(|_| "avfilter".to_string()),
    #[cfg(feature = "avformat")]
    std::env::var("LIBAVFORMAT_PATH").unwrap_or_else(|_| "avformat".to_string()),
    #[cfg(feature = "avutil")]
    std::env::var("LIBAVUTIL_PATH").unwrap_or_else(|_| "avutil".to_string()),
    #[cfg(feature = "swresample")]
    std::env::var("LIBSWRESAMPLE_PATH").unwrap_or_else(|_| "swresample".to_string()),
    #[cfg(feature = "swscale")]
    std::env::var("LIBSWSCALE_PATH").unwrap_or_else(|_| "swscale".to_string()),
    #[cfg(feature = "postproc")]
    std::env::var("LIBPOSTPROC_PATH").unwrap_or_else(|_| "postproc".to_string()),
] {
    println!("cargo:rustc-link-lib={}", lib);
}
```
