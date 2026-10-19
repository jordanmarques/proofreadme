//! `cargo xtask package`: builds ProofReadMe in release and produces the package of the host OS in
//! `dist/`, either the `ProofReadMe.app` bundle (macOS) or `ProofReadMe.exe` (Windows).
//!
//! The macOS bundle is signed with a self-signed certificate of the login keychain (ADR 0004): the
//! Accessibility permission is tied to the signature, and a stable certificate keeps it granted
//! across updates. No installer, no Developer ID, no notarisation.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The name of the application: that of the bundle, of its executable, of its icon and of the
/// `.exe`.
const NAME: &str = "ProofReadMe";

/// The name of the executable cargo builds.
const BINARY: &str = "proofreadme";

/// The identifier of the macOS bundle. It is also the `Label` of the LaunchAgent the application
/// registers (`proofreadme::IDENTIFIER`): a test checks that the two stay equal.
const IDENTIFIER: &str = "io.github.jordanmarques.proofreadme";

/// The oldest macOS version targeted: the first one on Apple silicon. It is given to the build
/// (`MACOSX_DEPLOYMENT_TARGET`) and declared in the Info.plist.
const MACOS_MINIMUM: &str = "11.0";

/// The environment variable that names the certificate the macOS bundle is signed with.
const IDENTITY_VARIABLE: &str = "PROOFREADME_SIGNING_IDENTITY";

/// The name of the certificate when [`IDENTITY_VARIABLE`] is not set.
const DEFAULT_IDENTITY: &str = "ProofReadMe Signing";

/// The option that asks for an ad hoc signature instead of the certificate.
const AD_HOC: &str = "--ad-hoc";

/// The share of the side left transparent around the tile in the icon of the bundle, as macOS
/// expects of its application icons.
const ICON_MARGIN: f32 = 0.1;

/// How the macOS bundle is signed.
#[derive(Debug, PartialEq, Eq)]
enum Signature {
    /// With the self-signed certificate of this name, from the login keychain (ADR 0004).
    Certificate(String),
    /// Ad hoc, with no identity: the Accessibility permission is lost at every update. Only on
    /// request (`--ad-hoc`), for a machine without the certificate such as the CI.
    AdHoc,
}

/// Builds ProofReadMe in release, then produces the package of the host OS in `dist/`.
///
/// On macOS the bundle is signed with the certificate named by `PROOFREADME_SIGNING_IDENTITY`
/// (`ProofReadMe Signing` by default), or ad hoc with `--ad-hoc`.
pub fn package(arguments: &[String]) -> Result<(), String> {
    let signature = signature(arguments, std::env::var(IDENTITY_VARIABLE).ok())?;
    let root = crate::root();
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let dist = root.join("dist");
    let product = if cfg!(target_os = "macos") {
        // Checked before the build: a missing certificate is known at once.
        if let Signature::Certificate(name) = &signature {
            certificate_in_the_keychain(name)?;
        }
        build()?;
        let bundle = assemble_the_bundle(&dist, &target.join("release").join(BINARY), env!("CARGO_PKG_VERSION"))?;
        sign(&bundle, &signature)?;
        bundle
    } else if cfg!(windows) {
        if signature == Signature::AdHoc {
            return Err(format!("package: {AD_HOC} only exists on macOS"));
        }
        build()?;
        copy_the_exe(&dist, &target.join("release").join(format!("{BINARY}.exe")))?
    } else {
        return Err("package: only macOS and Windows have a package".to_owned());
    };
    eprintln!("xtask package: {}", product.display());
    Ok(())
}

/// How the bundle is to be signed, after the arguments of `package` (only `--ad-hoc`, at most
/// once) and the value of [`IDENTITY_VARIABLE`]. An empty variable counts as unset.
fn signature(arguments: &[String], variable: Option<String>) -> Result<Signature, String> {
    let mut ad_hoc = false;
    for argument in arguments {
        if argument != AD_HOC {
            return Err(format!("package: unknown argument `{argument}`; the only option is {AD_HOC}"));
        }
        if std::mem::replace(&mut ad_hoc, true) {
            return Err(format!("package: `{argument}` is given twice"));
        }
    }
    if ad_hoc {
        return Ok(Signature::AdHoc);
    }
    let name = variable.map(|name| name.trim().to_owned()).filter(|name| !name.is_empty());
    Ok(Signature::Certificate(name.unwrap_or_else(|| DEFAULT_IDENTITY.to_owned())))
}

/// True if the output of `security find-identity -p codesigning` lists a certificate of exactly
/// this name, trusted or not (a self-signed certificate signs without being trusted).
fn identity_is_listed(find_identity: &str, name: &str) -> bool {
    let quoted = format!("\"{name}\"");
    find_identity
        .lines()
        .any(|line| line.split_once(") ").is_some_and(|(_, rest)| rest.split_once(' ').is_some_and(|(_, label)| label.starts_with(&quoted))))
}

/// Checks that the keychains hold a code signing certificate of this name, and otherwise says how
/// to create it.
fn certificate_in_the_keychain(name: &str) -> Result<(), String> {
    let output = Command::new("security")
        .args(["find-identity", "-p", "codesigning"])
        .output()
        .map_err(|e| format!("security find-identity: cannot run: {e}"))?;
    if identity_is_listed(&String::from_utf8_lossy(&output.stdout), name) {
        return Ok(());
    }
    Err(format!(
        "package: no code signing certificate named \"{name}\" in the keychain.\n\
         Create it once in the login keychain (README, \"Signing certificate\"), or name another one with \
         {IDENTITY_VARIABLE}=<name>.\n\
         To package anyway, pass {AD_HOC}: the Accessibility permission will then have to be granted again after every update."
    ))
}

/// Signs the bundle. The signature covers the Info.plist and the resources, which the linker's own
/// signature of the executable does not.
fn sign(bundle: &Path, signature: &Signature) -> Result<(), String> {
    let mut codesign = Command::new("codesign");
    match signature {
        Signature::Certificate(name) => {
            codesign.args(["--force", "--sign", name]).arg(bundle);
            crate::run(&mut codesign, &format!("codesign --force --sign \"{name}\""))
        }
        Signature::AdHoc => {
            eprintln!(
                "\nxtask package: WARNING: {NAME}.app is signed AD HOC, not with a certificate.\n\
                 macOS will NOT keep the Accessibility permission across updates of this build.\n"
            );
            codesign.args(["--force", "--sign", "-"]).arg(bundle);
            crate::run(&mut codesign, "codesign --force --sign - (ad hoc)")
        }
    }
}

/// `cargo build --release` of the application, for the architecture of the machine.
fn build() -> Result<(), String> {
    let mut command = crate::cargo();
    command.args(["build", "--release", "--locked", "-p", BINARY]);
    if cfg!(target_os = "macos") {
        command.env("MACOSX_DEPLOYMENT_TARGET", MACOS_MINIMUM);
    }
    crate::run(&mut command, &format!("cargo build --release --locked -p {BINARY}"))
}

/// Assembles `dist/ProofReadMe.app` around the given executable and returns the path of the bundle.
/// A previous bundle is replaced as a whole; the rest of `dist/` is not touched.
fn assemble_the_bundle(dist: &Path, executable: &Path, version: &str) -> Result<PathBuf, String> {
    let bundle = dist.join(format!("{NAME}.app"));
    if bundle.exists() {
        fs::remove_dir_all(&bundle).map_err(|e| format!("removing {}: {e}", bundle.display()))?;
    }
    let contents = bundle.join("Contents");
    let (binaries, resources) = (contents.join("MacOS"), contents.join("Resources"));
    for folder in [&binaries, &resources] {
        fs::create_dir_all(folder).map_err(|e| format!("creating {}: {e}", folder.display()))?;
    }
    let binary = binaries.join(NAME);
    copy(executable, &binary)?;
    make_executable(&binary)?;
    write(&contents.join("Info.plist"), info_plist(version).as_bytes())?;
    // Type and creator of the bundle, the old way: `APPL` and an unknown creator.
    write(&contents.join("PkgInfo"), b"APPL????")?;
    write(&resources.join(format!("{NAME}.icns")), &icns()?)?;
    Ok(bundle)
}

/// Copies the Windows executable to `dist/ProofReadMe.exe` and returns this path.
fn copy_the_exe(dist: &Path, executable: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dist).map_err(|e| format!("creating {}: {e}", dist.display()))?;
    let exe = dist.join(format!("{NAME}.exe"));
    copy(executable, &exe)?;
    Ok(exe)
}

fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination).map(|_| ()).map_err(|e| format!("copying {} to {}: {e}", source.display(), destination.display()))
}

fn write(file: &Path, content: &[u8]) -> Result<(), String> {
    fs::write(file, content).map_err(|e| format!("writing {}: {e}", file.display()))
}

#[cfg(unix)]
fn make_executable(file: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).map_err(|e| format!("permissions of {}: {e}", file.display()))
}

/// Outside Unix, a file has no execute permission to set.
#[cfg(not(unix))]
fn make_executable(_file: &Path) -> Result<(), String> {
    Ok(())
}

/// The Info.plist of the bundle: an agent with no Dock icon (`LSUIElement`), its identifier, its
/// executable and its icon.
fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>{NAME}</string>
  <key>CFBundleName</key>
  <string>{NAME}</string>
  <key>CFBundleExecutable</key>
  <string>{NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>{IDENTIFIER}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleSignature</key>
  <string>????</string>
  <key>CFBundleShortVersionString</key>
  <string>{version}</string>
  <key>CFBundleVersion</key>
  <string>{version}</string>
  <key>CFBundleIconFile</key>
  <string>{NAME}</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.productivity</string>
  <key>LSMinimumSystemVersion</key>
  <string>{MACOS_MINIMUM}</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSHumanReadableCopyright</key>
  <string>Copyright © Jordan Marques. MIT OR Apache-2.0.</string>
</dict>
</plist>
"#
    )
}

/// The icon of the bundle in the `.icns` format: the application icon in PNG, at the sizes macOS
/// asks for.
fn icns() -> Result<Vec<u8>, String> {
    /// The type of each entry and the side of its image, in pixels. `ic11` and `ic12` are the 16
    /// and 32 points at double density, `ic13` and `ic14` the 128 and 256 points at double density.
    const ENTRIES: [(&[u8; 4], u32); 8] =
        [(b"ic11", 32), (b"ic12", 64), (b"ic07", 128), (b"ic13", 256), (b"ic08", 256), (b"ic14", 512), (b"ic09", 512), (b"ic10", 1024)];

    let mut entries = Vec::new();
    // Two entries with the same side share their image.
    let mut last: Option<(u32, Vec<u8>)> = None;
    for (kind, side) in ENTRIES {
        let image = match last.take() {
            Some((previous_side, image)) if previous_side == side => image,
            _ => png(side, &proofreadme_icon::application(side, ICON_MARGIN))?,
        };
        entries.extend_from_slice(kind);
        entries.extend_from_slice(&length(8 + image.len())?);
        entries.extend_from_slice(&image);
        last = Some((side, image));
    }
    let mut file = Vec::with_capacity(8 + entries.len());
    file.extend_from_slice(b"icns");
    file.extend_from_slice(&length(8 + entries.len())?);
    file.extend_from_slice(&entries);
    Ok(file)
}

/// A length in an `.icns` file: 32 bits, big-endian.
fn length(bytes: usize) -> Result<[u8; 4], String> {
    u32::try_from(bytes).map(u32::to_be_bytes).map_err(|_| "icon too large for an .icns file".to_owned())
}

/// A square RGBA image encoded as PNG.
fn png(side: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let as_text = |error: png::EncodingError| format!("PNG encoding of the icon: {error}");
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, side, side);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(as_text)?;
    writer.write_image_data(rgba).map_err(as_text)?;
    writer.finish().map_err(as_text)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(list: &[&str]) -> Vec<String> {
        list.iter().map(|argument| (*argument).to_owned()).collect()
    }

    #[test]
    fn the_bundle_is_signed_with_the_default_certificate_unless_the_variable_names_another() {
        assert_eq!(signature(&[], None), Ok(Signature::Certificate("ProofReadMe Signing".to_owned())));
        assert_eq!(signature(&[], Some("Jordan Code".to_owned())), Ok(Signature::Certificate("Jordan Code".to_owned())));
        assert_eq!(signature(&[], Some("  ".to_owned())), Ok(Signature::Certificate("ProofReadMe Signing".to_owned())));
    }

    #[test]
    fn the_ad_hoc_signature_is_only_given_on_request() {
        assert_eq!(signature(&arguments(&["--ad-hoc"]), Some("Jordan Code".to_owned())), Ok(Signature::AdHoc));
    }

    #[test]
    fn package_refuses_an_unknown_or_repeated_argument() {
        assert!(signature(&arguments(&["--adhoc"]), None).unwrap_err().contains("--adhoc"));
        assert!(signature(&arguments(&["--ad-hoc", "--ad-hoc"]), None).is_err());
    }

    /// The output of `security find-identity -p codesigning` on a machine holding the certificate,
    /// not trusted (as a self-signed certificate is unless trusted by hand).
    const FIND_IDENTITY: &str = r#"
Policy: Code Signing
  Matching identities
  1) 3F2A6C1D9E8B7A6F5E4D3C2B1A0F9E8D7C6B5A49 "ProofReadMe Signing" (CSSMERR_TP_NOT_TRUSTED)
  2) 0A1B2C3D4E5F60718293A4B5C6D7E8F901234567 "Apple Development: Jordan Marques (ABCDE12345)"
     2 identities found

  Valid identities only
  1) 0A1B2C3D4E5F60718293A4B5C6D7E8F901234567 "Apple Development: Jordan Marques (ABCDE12345)"
     1 valid identities found
"#;

    #[test]
    fn a_certificate_of_the_keychain_is_found_by_its_exact_name_even_untrusted() {
        assert!(identity_is_listed(FIND_IDENTITY, "ProofReadMe Signing"));
        assert!(!identity_is_listed(FIND_IDENTITY, "ProofReadMe"));
        assert!(!identity_is_listed(FIND_IDENTITY, "Other"));
        assert!(!identity_is_listed("\nPolicy: Code Signing\n  Matching identities\n     0 identities found\n", "ProofReadMe Signing"));
    }

    /// A fake release executable, placed in a temporary folder.
    fn fake_executable(folder: &Path, name: &str) -> PathBuf {
        let path = folder.join(name);
        fs::write(&path, b"test binary").unwrap();
        path
    }

    /// The value that follows the given key in an Info.plist, tags included.
    fn value<'a>(plist: &'a str, key: &str) -> &'a str {
        let after = plist.split(&format!("<key>{key}</key>")).nth(1).unwrap_or_else(|| panic!("key {key} missing"));
        after.trim_start().lines().next().unwrap().trim()
    }

    #[test]
    fn the_info_plist_describes_an_agent_with_no_dock_icon_with_its_identifier_and_its_version() {
        let plist = info_plist("1.2.3");

        assert_eq!(value(&plist, "CFBundleIdentifier"), "<string>io.github.jordanmarques.proofreadme</string>");
        assert_eq!(value(&plist, "CFBundleExecutable"), "<string>ProofReadMe</string>");
        assert_eq!(value(&plist, "CFBundleName"), "<string>ProofReadMe</string>");
        assert_eq!(value(&plist, "CFBundlePackageType"), "<string>APPL</string>");
        assert_eq!(value(&plist, "CFBundleIconFile"), "<string>ProofReadMe</string>");
        assert_eq!(value(&plist, "CFBundleShortVersionString"), "<string>1.2.3</string>");
        assert_eq!(value(&plist, "CFBundleVersion"), "<string>1.2.3</string>");
        assert_eq!(value(&plist, "LSMinimumSystemVersion"), "<string>11.0</string>");
        assert_eq!(value(&plist, "LSUIElement"), "<true/>", "no icon in the Dock");
        assert_eq!(value(&plist, "NSHighResolutionCapable"), "<true/>");
    }

    #[test]
    fn the_identifier_of_the_bundle_is_the_one_the_application_registers_at_login() {
        let source = fs::read_to_string(crate::root().join("apps/proofreadme/src/startup.rs")).unwrap();

        assert!(source.contains(&format!("pub const IDENTIFIER: &str = \"{IDENTIFIER}\";")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_info_plist_is_accepted_by_plutil() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("Info.plist");
        fs::write(&file, info_plist("0.1.0")).unwrap();

        let output = Command::new("plutil").arg("-lint").arg(&file).output().unwrap();

        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    }

    #[test]
    fn the_icns_file_lays_out_one_png_image_per_size_macos_expects() {
        let file = icns().unwrap();

        assert_eq!(&file[..4], b"icns");
        assert_eq!(u32::from_be_bytes(file[4..8].try_into().unwrap()) as usize, file.len());
        // (type of the entry, side in pixels), after the format: ic11 and ic12 are the 16 and 32
        // points at double density, ic13 and ic14 the 128 and 256 points at double density.
        let expected: [(&[u8; 4], u32); 8] =
            [(b"ic11", 32), (b"ic12", 64), (b"ic07", 128), (b"ic13", 256), (b"ic08", 256), (b"ic14", 512), (b"ic09", 512), (b"ic10", 1024)];
        let mut position = 8;
        for (kind, side) in expected {
            assert_eq!(&file[position..position + 4], kind);
            let size = u32::from_be_bytes(file[position + 4..position + 8].try_into().unwrap()) as usize;
            let png = &file[position + 8..position + size];
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "the entry {} is a PNG", String::from_utf8_lossy(kind));
            assert_eq!(&png[12..16], b"IHDR");
            assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), side, "width");
            assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), side, "height");
            position += size;
        }
        assert_eq!(position, file.len());
    }

    #[test]
    fn the_bundle_puts_the_info_plist_the_executable_and_the_icon_in_their_place() {
        let folder = tempfile::tempdir().unwrap();
        let executable = fake_executable(folder.path(), "proofreadme");
        let dist = folder.path().join("dist");

        let bundle = assemble_the_bundle(&dist, &executable, "1.2.3").unwrap();

        assert_eq!(bundle, dist.join("ProofReadMe.app"));
        let contents = bundle.join("Contents");
        assert_eq!(fs::read_to_string(contents.join("Info.plist")).unwrap(), info_plist("1.2.3"));
        assert_eq!(fs::read(contents.join("MacOS/ProofReadMe")).unwrap(), b"test binary");
        assert_eq!(fs::read(contents.join("Resources/ProofReadMe.icns")).unwrap(), icns().unwrap());
        assert_eq!(fs::read(contents.join("PkgInfo")).unwrap(), b"APPL????");
    }

    #[cfg(unix)]
    #[test]
    fn the_executable_of_the_bundle_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().unwrap();
        let executable = fake_executable(folder.path(), "proofreadme");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o644)).unwrap();

        let bundle = assemble_the_bundle(&folder.path().join("dist"), &executable, "1.2.3").unwrap();

        let mode = fs::metadata(bundle.join("Contents/MacOS/ProofReadMe")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn a_previous_bundle_is_replaced_as_a_whole() {
        let folder = tempfile::tempdir().unwrap();
        let executable = fake_executable(folder.path(), "proofreadme");
        let dist = folder.path().join("dist");
        let leftover = dist.join("ProofReadMe.app/Contents/MacOS/old");
        fs::create_dir_all(leftover.parent().unwrap()).unwrap();
        fs::write(&leftover, b"leftover of a previous build").unwrap();
        fs::write(dist.join("other.txt"), b"does not belong to the bundle").unwrap();

        assemble_the_bundle(&dist, &executable, "1.2.3").unwrap();

        assert!(!leftover.exists(), "the previous bundle is removed");
        assert!(dist.join("other.txt").exists(), "the rest of dist/ is not touched");
    }

    #[test]
    fn a_missing_executable_is_an_error_that_names_it() {
        let folder = tempfile::tempdir().unwrap();
        let missing = folder.path().join("proofreadme");

        let error = assemble_the_bundle(&folder.path().join("dist"), &missing, "1.2.3").unwrap_err();

        assert!(error.contains(&missing.display().to_string()), "{error}");
    }

    #[test]
    fn the_windows_exe_is_copied_under_the_name_of_the_application() {
        let folder = tempfile::tempdir().unwrap();
        let executable = fake_executable(folder.path(), "proofreadme.exe");
        let dist = folder.path().join("dist");

        let exe = copy_the_exe(&dist, &executable).unwrap();

        assert_eq!(exe, dist.join("ProofReadMe.exe"));
        assert_eq!(fs::read(exe).unwrap(), b"test binary");
    }
}
