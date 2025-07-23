#![allow(unused)]

use std::{collections::HashSet, env, fs, hash::Hash, path::Path, process::Command};

use anyhow::{anyhow, Result};
use bindgen::callbacks::{
    EnumVariantCustomBehavior, EnumVariantValue, IntKind, MacroParsingBehavior, ParseCallbacks,
};

fn is_exsit(dir: &str) -> bool {
    fs::metadata(dir).is_ok()
}

fn join(root: &str, next: &str) -> Result<String> {
    Ok(Path::new(root)
        .join(next)
        .to_str()
        .ok_or_else(|| anyhow!("Failed to path into string."))?
        .to_string())
}

fn exec(command: &str, work_dir: &str) -> Result<String> {
    let output = Command::new(if cfg!(target_os = "windows") {
        "powershell"
    } else {
        "bash"
    })
    .arg(if cfg!(target_os = "windows") {
        "-command"
    } else {
        "-c"
    })
    .arg(if cfg!(target_os = "windows") {
        format!("$ProgressPreference = 'SilentlyContinue';{}", command)
    } else {
        command.to_string()
    })
    .current_dir(work_dir)
    .output()?;

    if !output.status.success() {
        Err(anyhow!("{}", unsafe {
            String::from_utf8_unchecked(output.stderr)
        }))
    } else {
        Ok(unsafe { String::from_utf8_unchecked(output.stdout) })
    }
}

fn de_duplicate<T: Eq + Hash, I: Iterator<Item = T>>(input: I) -> Vec<T> {
    let mut set = HashSet::new();
    for it in input {
        set.insert(it);
    }

    set.into_iter().map(|it| it).collect()
}

fn search_include(include_prefix: &[String], header: &str) -> Result<String> {
    for dir in include_prefix {
        let include = join(dir, header)?;
        if fs::metadata(&include).is_ok() {
            return Ok(include);
        }
    }

    Err(anyhow!("not found header = {:?}", header))
}

#[cfg(target_os = "windows")]
fn find_ffmpeg_prefix(out_dir: &str) -> Result<String> {
    let prefix = join(out_dir, "./ffmpeg")?;
    if !is_exsit(&prefix) {
        exec(
            "Invoke-WebRequest -Uri https://github.com/mycrl/ffmpeg-rs/releases/download/ffmpeg-7.1/ffmpeg-windows-x64-7.1.zip -OutFile ffmpeg.zip", 
            out_dir
        )?;

        exec(
            "Expand-Archive -Path ffmpeg.zip -DestinationPath ./",
            out_dir,
        )?;

        exec("Remove-Item ./ffmpeg.zip", out_dir)?;
    }

    Ok(join(&prefix, "./lib")?)
}

#[cfg(target_os = "linux")]
fn find_ffmpeg_prefix(out_dir: &str) -> Result<String> {
    #[cfg(target_arch = "x86_64")]
    let name = "ffmpeg-linux-x64-7.1";

    #[cfg(target_arch = "aarch64")]
    let name = "ffmpeg-linux-aarch64-7.1";

    let prefix = join(out_dir, "./ffmpeg")?;
    if !is_exsit(&prefix) {
        exec(
            &format!(
                "wget https://github.com/mycrl/ffmpeg-rs/releases/download/ffmpeg-7.1/{}.zip",
                name
            ),
            out_dir,
        )?;

        exec(&format!("unzip {}.zip", name), out_dir)?;
        exec(&format!("rm -f {}.zip", name), out_dir)?;
    }

    Ok(join(&prefix, "./lib")?)
}

#[cfg(target_os = "macos")]
fn find_ffmpeg_prefix(out_dir: &str) -> Result<String> {
    let prefix = exec("brew --prefix ffmpeg@7", out_dir)?.replace('\n', "");
    Ok(join(&prefix, "./lib")?)
}

#[derive(Debug)]
struct Callbacks;

impl ParseCallbacks for Callbacks {
    fn int_macro(&self, _name: &str, value: i64) -> Option<IntKind> {
        let ch_layout_prefix = "AV_CH_";
        let codec_cap_prefix = "AV_CODEC_CAP_";
        let codec_flag_prefix = "AV_CODEC_FLAG_";
        let error_max_size = "AV_ERROR_MAX_STRING_SIZE";

        if _name.starts_with(ch_layout_prefix) {
            Some(IntKind::ULongLong)
        } else if value >= i32::MIN as i64
            && value <= i32::MAX as i64
            && (_name.starts_with(codec_cap_prefix) || _name.starts_with(codec_flag_prefix))
        {
            Some(IntKind::UInt)
        } else if _name == error_max_size {
            Some(IntKind::Custom {
                name: "usize",
                is_signed: false,
            })
        } else if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
            Some(IntKind::Int)
        } else {
            None
        }
    }

    fn enum_variant_behavior(
        &self,
        _enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<EnumVariantCustomBehavior> {
        let dummy_codec_id_prefix = "AV_CODEC_ID_FIRST_";
        if original_variant_name.starts_with(dummy_codec_id_prefix) {
            Some(EnumVariantCustomBehavior::Constify)
        } else {
            None
        }
    }

    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        use MacroParsingBehavior::*;

        match name {
            "FP_INFINITE" => Ignore,
            "FP_NAN" => Ignore,
            "FP_NORMAL" => Ignore,
            "FP_SUBNORMAL" => Ignore,
            "FP_ZERO" => Ignore,
            _ => Default,
        }
    }
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=./build.rs");

    let is_docs = std::env::var("DOCS_RS").is_ok();
    let out_dir = env::var("OUT_DIR")?;

    if !is_docs {
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
    }

    let mut builder = bindgen::Builder::default()
        .clang_args([format!("-I{}", "./include")])
        .blocklist_type("max_align_t")
        .opaque_type("__mingw_ldbl_type_t")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .prepend_enum_name(false)
        .derive_eq(true)
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(Callbacks))
        .blocklist_function("_.*")
        .blocklist_function("acoshl")
        .blocklist_function("acosl")
        .blocklist_function("asinhl")
        .blocklist_function("asinl")
        .blocklist_function("atan2l")
        .blocklist_function("atanhl")
        .blocklist_function("atanl")
        .blocklist_function("cbrtl")
        .blocklist_function("ceill")
        .blocklist_function("copysignl")
        .blocklist_function("coshl")
        .blocklist_function("cosl")
        .blocklist_function("dreml")
        .blocklist_function("ecvt_r")
        .blocklist_function("erfcl")
        .blocklist_function("erfl")
        .blocklist_function("exp2l")
        .blocklist_function("expl")
        .blocklist_function("expm1l")
        .blocklist_function("fabsl")
        .blocklist_function("fcvt_r")
        .blocklist_function("fdiml")
        .blocklist_function("finitel")
        .blocklist_function("floorl")
        .blocklist_function("fmal")
        .blocklist_function("fmaxl")
        .blocklist_function("fminl")
        .blocklist_function("fmodl")
        .blocklist_function("frexpl")
        .blocklist_function("gammal")
        .blocklist_function("hypotl")
        .blocklist_function("ilogbl")
        .blocklist_function("isinfl")
        .blocklist_function("isnanl")
        .blocklist_function("j0l")
        .blocklist_function("j1l")
        .blocklist_function("jnl")
        .blocklist_function("ldexpl")
        .blocklist_function("lgammal")
        .blocklist_function("lgammal_r")
        .blocklist_function("llrintl")
        .blocklist_function("llroundl")
        .blocklist_function("log10l")
        .blocklist_function("log1pl")
        .blocklist_function("log2l")
        .blocklist_function("logbl")
        .blocklist_function("logl")
        .blocklist_function("lrintl")
        .blocklist_function("lroundl")
        .blocklist_function("modfl")
        .blocklist_function("nanl")
        .blocklist_function("nearbyintl")
        .blocklist_function("nextafterl")
        .blocklist_function("nexttoward")
        .blocklist_function("nexttowardf")
        .blocklist_function("nexttowardl")
        .blocklist_function("powl")
        .blocklist_function("qecvt")
        .blocklist_function("qecvt_r")
        .blocklist_function("qfcvt")
        .blocklist_function("qfcvt_r")
        .blocklist_function("qgcvt")
        .blocklist_function("remainderl")
        .blocklist_function("remquol")
        .blocklist_function("rintl")
        .blocklist_function("roundl")
        .blocklist_function("scalbl")
        .blocklist_function("scalblnl")
        .blocklist_function("scalbnl")
        .blocklist_function("significandl")
        .blocklist_function("sinhl")
        .blocklist_function("sinl")
        .blocklist_function("sqrtl")
        .blocklist_function("strtold")
        .blocklist_function("tanhl")
        .blocklist_function("tanl")
        .blocklist_function("tgammal")
        .blocklist_function("truncl")
        .blocklist_function("y0l")
        .blocklist_function("y1l")
        .blocklist_function("ynl")
        .generate_comments(false)
        .header("src/defines.h");

    let mut headers = Vec::with_capacity(255);

    #[cfg(feature = "avcodec")]
    headers.append(&mut vec!["libavcodec/avcodec.h"]);

    #[cfg(feature = "avdevice")]
    {
        headers.append(&mut vec!["libavdevice/avdevice.h"]);

        #[cfg(target_os = "windows")]
        {
            // ignore d3d11
            {
                let header_path = join(&out_dir, "hwcontext_d3d11va.h")?;
                fs::write(
                    &header_path,
                    fs::read_to_string(search_include(
                        &["./include".to_string()],
                        "libavutil/hwcontext_d3d11va.h",
                    )?)?
                    .replace("#include <d3d11.h>", "")
                    .replace("ID3D11DeviceContext", "void")
                    .replace("ID3D11Device", "void")
                    .replace("ID3D11VideoDevice", "void")
                    .replace("ID3D11VideoContext", "void")
                    .replace("ID3D11Texture2D", "void")
                    .replace("UINT", "uint32_t"),
                )?;

                builder = builder.header(header_path);
            }

            #[cfg(feature = "qsv")]
            headers.append(&mut vec!["libavutil/hwcontext_qsv.h"]);
        }

        #[cfg(target_os = "linux")]
        {
            headers.append(&mut vec!["libavutil/hwcontext_drm.h"]);

            #[cfg(feature = "vaapi")]
            headers.append(&mut vec!["libavutil/hwcontext_vaapi.h"]);

            #[cfg(feature = "vaapi")]
            headers.append(&mut vec!["libavutil/hwcontext_qsv.h"]);
        }
    }

    #[cfg(feature = "avfilter")]
    headers.append(&mut vec![
        "libavfilter/avfilter.h",
        "libavfilter/buffersrc.h",
        "libavfilter/buffersink.h",
    ]);

    #[cfg(feature = "avformat")]
    headers.append(&mut vec!["libavformat/avformat.h"]);

    #[cfg(feature = "avutil")]
    headers.append(&mut vec![
        "libavutil/avutil.h",
        "libavutil/rational.h",
        "libavutil/imgutils.h",
        "libavutil/channel_layout.h",
    ]);

    #[cfg(feature = "swresample")]
    headers.append(&mut vec!["libswresample/swresample.h"]);

    #[cfg(feature = "swscale")]
    headers.append(&mut vec!["libswscale/swscale.h"]);

    for it in headers {
        builder = builder.header(search_include(&["./include".to_string()], it)?);
    }

    #[cfg(any(
        feature = "avcodec",
        feature = "avdevice",
        feature = "avfilter",
        feature = "avformat",
        feature = "avutil",
        feature = "swresample",
        feature = "swresample",
        feature = "swscale"
    ))]
    builder
        .generate()?
        .write_to_file(&join(&out_dir, "bindings.rs")?)?;
    Ok(())
}
