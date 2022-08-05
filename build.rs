// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use fslock::LockFile;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::process::Stdio;
use which::which;

fn main() {
  println!("cargo:rerun-if-changed=.gn");
  println!("cargo:rerun-if-changed=BUILD.gn");
  println!("cargo:rerun-if-changed=src/binding.cc");

  // These are all the environment variables that we check. This is
  // probably more than what is needed, but missing an important
  // variable can lead to broken links when switching rusty_v8
  // versions.
  let envs = vec![
    "CCACHE",
    "CLANG_BASE_PATH",
    "DENO_TRYBUILD",
    "DOCS_RS",
    "GN",
    "GN_ARGS",
    "HOST",
    "NINJA",
    "OUT_DIR",
    "RUSTY_V8_ARCHIVE",
    "RUSTY_V8_MIRROR",
    "SCCACHE",
    "V8_FORCE_DEBUG",
    "V8_FROM_SOURCE",
  ];
  for env in envs {
    println!("cargo:rerun-if-env-changed={}", env);
  }

  // Detect if trybuild tests are being compiled.
  let is_trybuild = env::var_os("DENO_TRYBUILD").is_some();

  // Don't build V8 if "cargo doc" is being run. This is to support docs.rs.
  let is_cargo_doc = env::var_os("DOCS_RS").is_some();

  // Don't build V8 if the rust language server (RLS) is running.
  let is_rls = env::var_os("CARGO")
    .map(PathBuf::from)
    .as_ref()
    .and_then(|p| p.file_stem())
    .and_then(|f| f.to_str())
    .map(|s| s.starts_with("rls"))
    .unwrap_or(false);

  // Early exit
  if is_cargo_doc || is_rls {
    return;
  }

  print_link_flags();

  // Don't attempt rebuild but link
  if is_trybuild {
    return;
  }

  // Build from source
  if env::var_os("V8_FROM_SOURCE").is_some() {
    return build_v8();
  }

  // utilize a lockfile to prevent linking of
  // only partially downloaded static library.
  let root = env::current_dir().unwrap();
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let lockfilepath = root
    .join(out_dir)
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("lib_download.fslock");
  println!("download lockfile: {:?}", &lockfilepath);
  let mut lockfile = LockFile::open(&lockfilepath)
    .expect("Couldn't open lib download lockfile.");
  lockfile.lock().expect("Couldn't get lock");
  download_static_lib_binaries();
  lockfile.unlock().expect("Couldn't unlock lockfile");
}

fn build_v8() {
  env::set_var("DEPOT_TOOLS_WIN_TOOLCHAIN", "0");

  // cargo publish doesn't like pyc files.
  env::set_var("PYTHONDONTWRITEBYTECODE", "1");

  // git submodule update --init --recursive
  let libcxx_src = PathBuf::from("buildtools/third_party/libc++/trunk/src");
  if !libcxx_src.is_dir() {
    eprintln!(
      "missing source code. Run 'git submodule update --init --recursive'"
    );
    exit(1);
  }

  if need_gn_ninja_download() {
    download_ninja_gn_binaries();
  }

  // On windows, rustc cannot link with a V8 debug build.
  let mut gn_args = if is_debug() && !cfg!(target_os = "windows") {
    // Note: When building for Android aarch64-qemu, use release instead of debug.
    vec!["is_debug=true".to_string()]
  } else {
    vec!["is_debug=false".to_string()]
  };

  if cfg!(not(feature = "use_custom_libcxx")) {
    gn_args.push("use_custom_libcxx=false".to_string());
  }

  // Fix GN's host_cpu detection when using x86_64 bins on Apple Silicon
  if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
    gn_args.push("host_cpu=\"arm64\"".to_string())
  }

  if let Some(clang_base_path) = find_compatible_system_clang() {
    println!("clang_base_path {}", clang_base_path.display());
    gn_args.push(format!("clang_base_path={:?}", clang_base_path));
    gn_args.push("treat_warnings_as_errors=false".to_string());
  } else {
    println!("using Chromiums clang");
    let clang_base_path = clang_download();
    gn_args.push(format!("clang_base_path={:?}", clang_base_path));

    if cfg!(target_os = "android") && cfg!(target_arch = "aarch64") {
      gn_args.push("treat_warnings_as_errors=false".to_string());
    }
  }

  if let Some(p) = env::var_os("SCCACHE") {
    cc_wrapper(&mut gn_args, Path::new(&p));
  } else if let Ok(p) = which("sccache") {
    cc_wrapper(&mut gn_args, &p);
  } else if let Some(p) = env::var_os("CCACHE") {
    cc_wrapper(&mut gn_args, Path::new(&p));
  } else if let Ok(p) = which("ccache") {
    cc_wrapper(&mut gn_args, &p);
  } else {
    println!("cargo:warning=Not using sccache or ccache");
  }

  if let Ok(args) = env::var("GN_ARGS") {
    for arg in args.split_whitespace() {
      gn_args.push(arg.to_string());
    }
  }

  let target_triple = env::var("TARGET").unwrap();
  // check if the target triple describes a non-native environment
  if target_triple != env::var("HOST").unwrap() {
    // cross-compilation setup
    if target_triple == "aarch64-unknown-linux-gnu"
      || target_triple == "aarch64-linux-android"
    {
      gn_args.push(r#"target_cpu="arm64""#.to_string());
      gn_args.push("use_sysroot=true".to_string());
      maybe_install_sysroot("arm64");
      maybe_install_sysroot("amd64");
    };

    if target_triple == "aarch64-linux-android" {
      gn_args.push(r#"v8_target_cpu="arm64""#.to_string());
      gn_args.push(r#"target_os="android""#.to_string());

      gn_args.push("treat_warnings_as_errors=false".to_string());

      // NDK 23 and above removes libgcc entirely.
      // https://github.com/rust-lang/rust/pull/85806
      maybe_clone_repo(
        "./third_party/android_ndk",
        "https://github.com/denoland/android_ndk.git",
      );

      static CHROMIUM_URI: &str = "https://chromium.googlesource.com";

      maybe_clone_repo(
        "./third_party/android_platform",
        &format!(
          "{}/chromium/src/third_party/android_platform.git",
          CHROMIUM_URI
        ),
      );
      maybe_clone_repo(
        "./third_party/catapult",
        &format!("{}/catapult.git", CHROMIUM_URI),
      );
    };
  }

  if target_triple.starts_with("i686-") {
    gn_args.push(r#"target_cpu="x86""#.to_string());
  }

  let gn_root = env::var("CARGO_MANIFEST_DIR").unwrap();

  let gn_out = maybe_gen(&gn_root, gn_args);
  assert!(gn_out.exists());
  assert!(gn_out.join("args.gn").exists());
  print_gn_args(&gn_out);
  build("rusty_v8", None);
}

fn print_gn_args(gn_out_dir: &Path) {
  let cmd = Command::new(gn())
    .arg("args")
    .arg(&gn_out_dir)
    .arg("--list")
    .output()
    .unwrap();
  println!("STDOUT: {}", String::from_utf8_lossy(&cmd.stdout));
  println!("STDERR: {}", String::from_utf8_lossy(&cmd.stderr));
}

fn maybe_clone_repo(dest: &str, repo: &str) {
  if !Path::new(&dest).exists() {
    assert!(Command::new("git")
      .arg("clone")
      .arg("--depth=1")
      .arg(repo)
      .arg(dest)
      .status()
      .unwrap()
      .success());
  }
}

fn maybe_install_sysroot(arch: &str) {
  let sysroot_path = format!("build/linux/debian_sid_{}-sysroot", arch);
  if !PathBuf::from(sysroot_path).is_dir() {
    assert!(Command::new("python")
      .arg("./build/linux/sysroot_scripts/install-sysroot.py")
      .arg(format!("--arch={}", arch))
      .status()
      .unwrap()
      .success());
  }
}

fn platform() -> &'static str {
  #[cfg(target_os = "windows")]
  {
    "win"
  }
  #[cfg(target_os = "linux")]
  {
    "linux64"
  }
  #[cfg(target_os = "macos")]
  {
    "mac"
  }
}

fn download_ninja_gn_binaries() {
  let target_dir = build_dir();
  let bin_dir = target_dir
    .join("ninja_gn_binaries-20220517")
    .join(platform());
  let gn = bin_dir.join("gn");
  let ninja = bin_dir.join("ninja");
  #[cfg(windows)]
  let gn = gn.with_extension("exe");
  #[cfg(windows)]
  let ninja = ninja.with_extension("exe");

  if !gn.exists() || !ninja.exists() {
    assert!(Command::new("python")
      .arg("./tools/ninja_gn_binaries.py")
      .arg("--dir")
      .arg(&target_dir)
      .status()
      .unwrap()
      .success());
  }
  assert!(gn.exists());
  assert!(ninja.exists());
  env::set_var("GN", gn);
  env::set_var("NINJA", ninja);
}

fn static_lib_url() -> String {
  if let Ok(custom_archive) = env::var("RUSTY_V8_ARCHIVE") {
    return custom_archive;
  }
  let default_base = "https://github.com/denoland/rusty_v8/releases/download";
  let base =
    env::var("RUSTY_V8_MIRROR").unwrap_or_else(|_| default_base.into());
  let version = env::var("CARGO_PKG_VERSION").unwrap();
  let target = env::var("TARGET").unwrap();

  // Note: we always use the release build on windows.
  if cfg!(target_os = "windows") {
    return format!("{}/v{}/rusty_v8_release_{}.lib", base, version, target);
  }
  // Use v8 in release mode unless $V8_FORCE_DEBUG=true
  let profile = match env_bool("V8_FORCE_DEBUG") {
    true => "debug",
    _ => "release",
  };
  format!("{}/v{}/librusty_v8_{}_{}.a", base, version, profile, target)
}

fn env_bool(key: &str) -> bool {
  matches!(
    env::var(key).unwrap_or_default().as_str(),
    "true" | "1" | "yes"
  )
}

fn static_lib_name() -> &'static str {
  match cfg!(target_os = "windows") {
    true => "rusty_v8.lib",
    false => "librusty_v8.a",
  }
}

fn static_lib_path() -> PathBuf {
  static_lib_dir().join(static_lib_name())
}

fn static_checksum_path() -> PathBuf {
  let mut t = static_lib_path();
  t.set_extension("sum");
  t
}

fn static_lib_dir() -> PathBuf {
  build_dir().join("gn_out").join("obj")
}

fn build_dir() -> PathBuf {
  let root = env::current_dir().unwrap();

  // target/debug//build/rusty_v8-d9e5a424d4f96994/out/
  let out_dir = env::var_os("OUT_DIR").expect(
    "The 'OUT_DIR' environment is not set (it should be something like \
     'target/debug/rusty_v8-{hash}').",
  );
  let out_dir_abs = root.join(out_dir);

  // This would be target/debug or target/release
  out_dir_abs
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf()
}

fn download_file(url: String, filename: PathBuf) {
  if !url.starts_with("http:") && !url.starts_with("https:") {
    fs::copy(&url, filename).unwrap();
    return;
  }

  // tmp file to download to so we don't clobber the existing one
  let tmpfile = {
    let mut t = filename.clone();
    t.set_extension("tmp");
    t
  };
  if tmpfile.exists() {
    println!("Deleting old tmpfile {}", tmpfile.display());
    std::fs::remove_file(&tmpfile).unwrap();
  }

  // Try downloading with python first. Python is a V8 build dependency,
  // so this saves us from adding a Rust HTTP client dependency.
  println!("Downloading {}", url);
  let status = Command::new("python")
    .arg("./tools/download_file.py")
    .arg("--url")
    .arg(&url)
    .arg("--filename")
    .arg(&tmpfile)
    .status();

  // Python is only a required dependency for `V8_FROM_SOURCE` builds.
  // If python is not available, try falling back to curl.
  let status = match status {
    Ok(status) if status.success() => status,
    _ => {
      println!("Python downloader failed, trying with curl.");
      Command::new("curl")
        .arg("-L")
        .arg("-s")
        .arg("-o")
        .arg(&tmpfile)
        .arg(&url)
        .status()
        .unwrap()
    }
  };

  // Assert DL was successful
  assert!(status.success());
  assert!(tmpfile.exists());

  // Write checksum (i.e url) & move file
  std::fs::write(static_checksum_path(), url).unwrap();
  std::fs::rename(&tmpfile, &filename).unwrap();
  assert!(filename.exists());
  assert!(static_checksum_path().exists());
  assert!(!tmpfile.exists());
}

fn download_static_lib_binaries() {
  let url = static_lib_url();
  println!("static lib URL: {}", url);

  let dir = static_lib_dir();
  std::fs::create_dir_all(&dir).unwrap();
  println!("cargo:rustc-link-search={}", dir.display());

  // Checksum (i.e: url) to avoid redownloads
  match std::fs::read_to_string(static_checksum_path()) {
    Ok(c) if c == static_lib_url() => return,
    _ => {}
  };
  download_file(url, static_lib_path());
}

fn print_link_flags() {
  println!("cargo:rustc-link-lib=static=rusty_v8");

  let should_dyn_link_libcxx = cfg!(not(feature = "use_custom_libcxx"))
    || env::var("GN_ARGS").map_or(false, |gn_args| {
      gn_args
        .split_whitespace()
        .any(|ba| ba == "use_custom_libcxx=false")
    });

  if should_dyn_link_libcxx {
    // Based on https://github.com/alexcrichton/cc-rs/blob/fba7feded71ee4f63cfe885673ead6d7b4f2f454/src/lib.rs#L2462
    let target = env::var("TARGET").unwrap();
    if target.contains("apple")
      || target.contains("freebsd")
      || target.contains("openbsd")
    {
      println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") {
      println!("cargo:rustc-link-lib=dylib=stdc++");
    } else if target.contains("android") {
      println!("cargo:rustc-link-lib=dylib=c++_shared");
    }
  }

  if cfg!(target_os = "windows") {
    println!("cargo:rustc-link-lib=dylib=winmm");
    println!("cargo:rustc-link-lib=dylib=dbghelp");
  }

  if cfg!(target_env = "msvc") {
    // On Windows, including libcpmt[d]/msvcprt[d] explicitly links the C++
    // standard library, which libc++ needs for exception_ptr internals.
    if cfg!(target_feature = "crt-static") {
      println!("cargo:rustc-link-lib=libcpmt");
    } else {
      println!("cargo:rustc-link-lib=dylib=msvcprt");
    }
  }
}

// Chromium depot_tools contains helpers
// which delegate to the "relevant" `buildtools`
// directory when invoked, so they don't count.
fn not_in_depot_tools(p: PathBuf) -> bool {
  !p.as_path().to_str().unwrap().contains("depot_tools")
}

fn need_gn_ninja_download() -> bool {
  let has_ninja = which("ninja").map_or(false, not_in_depot_tools)
    || env::var_os("NINJA").is_some();
  let has_gn = which("gn").map_or(false, not_in_depot_tools)
    || env::var_os("GN").is_some();

  !has_ninja || !has_gn
}

// Chromiums gn arg clang_base_path is currently compatible with:
// * Apples clang and clang from homebrew's llvm@x packages
// * the official binaries from releases.llvm.org
// * unversioned (Linux) packages of clang (if recent enough)
// but unfortunately it doesn't work with version-suffixed packages commonly
// found in Linux packet managers
fn is_compatible_clang_version(clang_path: &Path) -> bool {
  if let Ok(o) = Command::new(clang_path).arg("--version").output() {
    let _output = String::from_utf8(o.stdout).unwrap();
    // TODO check version output to make sure it's supported.
    const _MIN_APPLE_CLANG_VER: f32 = 11.0;
    const _MIN_LLVM_CLANG_VER: f32 = 8.0;
    return true;
  }
  false
}

fn find_compatible_system_clang() -> Option<PathBuf> {
  if cfg!(target_os = "android") {
    return None;
  }

  if let Ok(p) = env::var("CLANG_BASE_PATH") {
    let base_path = Path::new(&p);
    let clang_path = base_path.join("bin").join("clang");
    if is_compatible_clang_version(&clang_path) {
      return Some(base_path.to_path_buf());
    }
  }

  None
}

// Download chromium's clang into OUT_DIR because Cargo will not allow us to
// modify the source directory.
fn clang_download() -> PathBuf {
  let clang_base_path = build_dir().join("clang");
  println!("clang_base_path {}", clang_base_path.display());
  assert!(Command::new("python")
    .arg("./tools/clang/scripts/update.py")
    .arg("--output-dir")
    .arg(&clang_base_path)
    .status()
    .unwrap()
    .success());
  assert!(clang_base_path.exists());
  clang_base_path
}

fn cc_wrapper(gn_args: &mut Vec<String>, sccache_path: &Path) {
  gn_args.push(format!("cc_wrapper={:?}", sccache_path));
}

struct Dirs {
  pub out: PathBuf,
  pub root: PathBuf,
}

fn get_dirs(manifest_dir: Option<&str>) -> Dirs {
  // The OUT_DIR is going to be a crate-specific directory like
  // "target/debug/build/cargo_gn_example-eee5160084460b2c"
  // But we want to share the GN build amongst all crates
  // and return the path "target/debug". So to find it, we walk up three
  // directories.
  // TODO(ry) This is quite brittle - if Cargo changes the directory structure
  // this could break.
  let out = env::var("OUT_DIR").map(PathBuf::from).unwrap();
  let out = out
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .to_owned();

  let root = match manifest_dir {
    Some(s) => env::current_dir().unwrap().join(s),
    None => env::var("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap(),
  };

  let mut dirs = Dirs { out, root };
  maybe_symlink_root_dir(&mut dirs);
  dirs
}

#[cfg(not(target_os = "windows"))]
fn maybe_symlink_root_dir(_: &mut Dirs) {}

#[cfg(target_os = "windows")]
fn maybe_symlink_root_dir(dirs: &mut Dirs) {
  // GN produces invalid paths if the source (a.k.a. root) directory is on a
  // different drive than the output. If this is the case we'll create a
  // symlink called "gn_root' in the out directory, next to 'gn_out', so it
  // appears as if they're both on the same drive.
  use std::fs::remove_dir;
  use std::os::windows::fs::symlink_dir;

  let get_prefix = |p: &Path| {
    p.components()
      .find_map(|c| match c {
        std::path::Component::Prefix(p) => Some(p),
        _ => None,
      })
      .map(|p| p.as_os_str().to_owned())
  };

  let Dirs { out, root } = dirs;
  if get_prefix(out) != get_prefix(root) {
    let symlink = &*out.join("gn_root");
    let target = &*root.canonicalize().unwrap();

    println!("Creating symlink {:?} to {:?}", &symlink, &root);

    loop {
      match symlink.canonicalize() {
        Ok(existing) if existing == target => break,
        Ok(_) => remove_dir(symlink).expect("remove_dir failed"),
        Err(_) => {
          break symlink_dir(target, symlink).expect("symlink_dir failed")
        }
      }
    }

    dirs.root = symlink.to_path_buf();
  }
}

pub fn is_debug() -> bool {
  // Cargo sets PROFILE to either "debug" or "release", which conveniently
  // matches the build modes we support.
  let m = env::var("PROFILE").unwrap();
  if m == "release" {
    false
  } else if m == "debug" {
    true
  } else {
    panic!("unhandled PROFILE value {}", m)
  }
}

fn gn() -> String {
  env::var("GN").unwrap_or_else(|_| "gn".to_owned())
}

type NinjaEnv = Vec<(String, String)>;

fn ninja(gn_out_dir: &Path, maybe_env: Option<NinjaEnv>) -> Command {
  let cmd_string = env::var("NINJA").unwrap_or_else(|_| "ninja".to_owned());
  let mut cmd = Command::new(cmd_string);
  cmd.arg("-C");
  cmd.arg(&gn_out_dir);
  if let Some(env) = maybe_env {
    for item in env {
      cmd.env(item.0, item.1);
    }
  }
  cmd
}

pub type GnArgs = Vec<String>;

pub fn maybe_gen(manifest_dir: &str, gn_args: GnArgs) -> PathBuf {
  let dirs = get_dirs(Some(manifest_dir));
  let gn_out_dir = dirs.out.join("gn_out");

  if !gn_out_dir.exists() || !gn_out_dir.join("build.ninja").exists() {
    let args = gn_args.join(" ");

    let path = env::current_dir().unwrap();
    println!("The current directory is {}", path.display());
    println!(
      "gn gen --root={} {}",
      dirs.root.display(),
      gn_out_dir.display()
    );
    assert!(Command::new(gn())
      .arg(format!("--root={}", dirs.root.display()))
      .arg("gen")
      .arg(&gn_out_dir)
      .arg("--args=".to_owned() + &args)
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .envs(env::vars())
      .status()
      .unwrap()
      .success());
  }
  gn_out_dir
}

pub fn build(target: &str, maybe_env: Option<NinjaEnv>) {
  let gn_out_dir = get_dirs(None).out.join("gn_out");

  rerun_if_changed(&gn_out_dir, maybe_env.clone(), target);

  // This helps Rust source files locate the snapshot, source map etc.
  println!("cargo:rustc-env=GN_OUT_DIR={}", gn_out_dir.display());

  assert!(ninja(&gn_out_dir, maybe_env)
    .arg(target)
    .status()
    .unwrap()
    .success());

  // TODO This is not sufficent. We need to use "gn desc" to query the target
  // and figure out what else we need to add to the link.
  println!(
    "cargo:rustc-link-search=native={}/obj/",
    gn_out_dir.display()
  );
}

/// build.rs does not get re-run unless we tell cargo about what files we
/// depend on. This outputs a bunch of rerun-if-changed lines to stdout.
fn rerun_if_changed(out_dir: &Path, maybe_env: Option<NinjaEnv>, target: &str) {
  let deps = ninja_get_deps(out_dir, maybe_env, target);
  for d in deps {
    let p = out_dir.join(d);
    assert!(p.exists(), "Path doesn't exist: {:?}", p);
    println!("cargo:rerun-if-changed={}", p.display());
  }
}

fn ninja_get_deps(
  out_dir: &Path,
  maybe_env: Option<NinjaEnv>,
  target: &str,
) -> HashSet<String> {
  let mut cmd = ninja(out_dir, maybe_env.clone());
  cmd.arg("-t");
  cmd.arg("graph");
  cmd.arg(target);
  let output = cmd.output().expect("ninja -t graph failed");
  let stdout = String::from_utf8(output.stdout).unwrap();
  let graph_files = parse_ninja_graph(&stdout);

  let mut cmd = ninja(out_dir, maybe_env);
  cmd.arg(target);
  cmd.arg("-t");
  cmd.arg("deps");
  let output = cmd.output().expect("ninja -t deps failed");
  let stdout = String::from_utf8(output.stdout).unwrap();
  let deps_files = parse_ninja_deps(&stdout);

  graph_files.union(&deps_files).map(String::from).collect()
}

pub fn parse_ninja_deps(s: &str) -> HashSet<String> {
  let mut out = HashSet::new();
  for line in s.lines() {
    if line.starts_with("  ") {
      let filename = line.trim().to_string();
      out.insert(filename);
    }
  }
  out
}

/// A parser for the output of "ninja -t graph". It returns all of the input
/// files.
pub fn parse_ninja_graph(s: &str) -> HashSet<String> {
  let mut out = HashSet::new();
  // This is extremely hacky and likely to break.
  for line in s.lines() {
    if line.starts_with('\"')
      && line.contains("label=")
      && !line.contains("shape=")
      && !line.contains(" -> ")
    {
      let filename = line.split('\"').nth(3).unwrap();
      if !filename.starts_with("..") {
        continue;
      }
      out.insert(filename.to_string());
    }
  }
  out
}

#[cfg(test)]
mod test {
  use super::*;

  const MOCK_GRAPH: &str = r#"
digraph ninja {
rankdir="LR"
node [fontsize=10, shape=box, height=0.25]
edge [fontsize=10]
"0x7fc3c040c210" [label="default"]
"0x7fc3c040a7f0" -> "0x7fc3c040c210" [label=" phony"]
"0x7fc3c040a7f0" [label="obj/default.stamp"]
"0x7fc3c040a790" [label="stamp", shape=ellipse]
"0x7fc3c040a790" -> "0x7fc3c040a7f0"
"0x7fc3c040a6c0" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a8a0" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a920" -> "0x7fc3c040a790" [arrowhead=none]
"0x7fc3c040a6c0" [label="obj/count_bytes.stamp"]
"0x7fc3c040a4d0" -> "0x7fc3c040a6c0" [label=" stamp"]
"0x7fc3c040a4d0" [label="gen/output.txt"]
"0x7fc3c040a400" [label="___count_bytes___build_toolchain_mac_clang_x64__rule", shape=ellipse]
"0x7fc3c040a400" -> "0x7fc3c040a4d0"
"0x7fc3c040a580" -> "0x7fc3c040a400" [arrowhead=none]
"0x7fc3c040a620" -> "0x7fc3c040a400" [arrowhead=none]
"0x7fc3c040a580" [label="../../../example/src/count_bytes.py"]
"0x7fc3c040a620" [label="../../../example/src/input.txt"]
"0x7fc3c040a8a0" [label="foo"]
"0x7fc3c040b5e0" [label="link", shape=ellipse]
"0x7fc3c040b5e0" -> "0x7fc3c040a8a0"
"0x7fc3c040b5e0" -> "0x7fc3c040b6d0"
"0x7fc3c040b5e0" -> "0x7fc3c040b780"
"0x7fc3c040b5e0" -> "0x7fc3c040b820"
"0x7fc3c040b020" -> "0x7fc3c040b5e0" [arrowhead=none]
"0x7fc3c040a920" -> "0x7fc3c040b5e0" [arrowhead=none]
"0x7fc3c040b020" [label="obj/foo/foo.o"]
"0x7fc3c040b0d0" -> "0x7fc3c040b020" [label=" cxx"]
"0x7fc3c040b0d0" [label="../../../example/src/foo.cc"]
"0x7fc3c040a920" [label="obj/libhello.a"]
"0x7fc3c040be00" -> "0x7fc3c040a920" [label=" alink"]
"0x7fc3c040be00" [label="obj/hello/hello.o"]
"0x7fc3c040beb0" -> "0x7fc3c040be00" [label=" cxx"]
"0x7fc3c040beb0" [label="../../../example/src/hello.cc"]
}
  "#;

  #[test]
  fn test_parse_ninja_graph() {
    let files = parse_ninja_graph(MOCK_GRAPH);
    assert!(files.contains("../../../example/src/input.txt"));
    assert!(files.contains("../../../example/src/count_bytes.py"));
    assert!(!files.contains("obj/hello/hello.o"));
  }

  #[test]
  fn test_static_lib_size() {
    let static_lib_size = std::fs::metadata(static_lib_path()).unwrap().len();
    assert!(static_lib_size <= 200u64 << 20); // No more than 200 MiB.
  }
}
