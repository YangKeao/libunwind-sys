extern crate autotools;
extern crate bindgen;
extern crate fs_extra;

use std::path::PathBuf;
use std::env;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut libunwind_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    libunwind_path.push("libunwind");
    let project_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _e = fs_extra::dir::create(libunwind_path.clone(), true);
    let options = fs_extra::dir::CopyOptions::new();
    fs_extra::dir::copy(project_dir.join("libunwind"), out_dir.clone(), &options).unwrap();

    let target = env::var("TARGET").unwrap();

    // Choose build.
    let link_lib_arch = match target.as_str() {
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => "x86_64",
        "i686-unknown-linux-gnu" | "i586-unknown-linux-gnu" => "x86",
        "aarch64-unknown-linux-gnu" => "aarch64",
        _ => "",
    };
    if link_lib_arch.is_empty() {
        println!("cargo:warning=target {} is unsupported", target);
        return;
    }

    // Build native C library only for static link.
    #[cfg(feature = "static")]
    {
        use autotools::Config;

        use std::process::Command;
        use std::fs;

        // Build libunwind.
        // Configure. Check if we compile for  x86 target on x86_64 host.
        let mut dst = Config::new(&libunwind_path);

        cfg_if::cfg_if! {
            if #[cfg(feature = "ptrace")] {
                dst.enable("ptrace", None);
            } else {
                dst.disable("ptrace", None);
            }
        }

        dst.disable("documentation", None)
            .disable("tests", None)
            .disable("minidebuginfo", None)
            .disable("coredump", None)
            .disable("setjmp", None)
            .enable("zlibdebuginfo", None)
            .disable_shared()
            .enable_static();

        let dst = dst.build();
        let library_path = format!("{}/lib", dst.display());
        println!("cargo:rustc-link-search={}", library_path);

        Command::new("ar")
            .args(["x", &format!("libunwind-{}.a", link_lib_arch)])
            .current_dir(&library_path)
            .output()
            .expect("failed to extract libunwind-ARCH.a");

        Command::new("ar")
            .args(["x", "libunwind.a"])
            .current_dir(&library_path)
            .output()
            .expect("failed to extract libunwind.a");

        let mut ar_args = vec!["cr".to_owned(), "libunwind-all.a".to_owned()];
        let static_link_blocklist = ["Gget_accessors.o"];
        fs::read_dir(&library_path)
            .expect("failed to read libunwind directory")
            .filter_map(|e| e.ok())
            .filter(|e| !static_link_blocklist.contains(&e.file_name().to_str().unwrap()))
            .filter(|e| e.file_type().unwrap().is_file())
            .filter(|e| e.file_name().to_str().unwrap().ends_with(".o"))
            .for_each(|e| ar_args.push(e.file_name().to_str().unwrap().to_owned()));

        Command::new("ar")
            .args(ar_args)
            .current_dir(&library_path)
            .output()
            .expect("failed to create libunwind-all.a");

        println!("cargo:rustc-link-lib=static=unwind-all");

        #[cfg(feature = "ptrace")]
        {
            panic!("static link with ptrace feature is not supported")
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(not(feature = "static"))] {
            println!("cargo:rustc-link-lib=unwind-{}", link_lib_arch);
            println!("cargo:rustc-link-lib=unwind");

            #[cfg(feature = "ptrace")]
            {
                println!("cargo:rustc-link-lib=unwind-ptrace");
            }
        }
    }

    let bindings = bindgen::Builder::default();
    let bindings = match link_lib_arch {
        "x86" => bindings.blocklist_function("_Ux86_.*"),
        "arm" => bindings.blocklist_function("_Uarm_.*"),
        _ => bindings.blocklist_function("_Ux86_64_.*"),
    };
    let bindings = bindings.header("libunwind/include/libunwind.h");

    #[cfg(feature = "ptrace")]
    let bindings = { bindings.header("libunwind/include/libunwind-ptrace.h") };

    let bindings = bindings.generate().expect("Unable to generate bindings");
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
