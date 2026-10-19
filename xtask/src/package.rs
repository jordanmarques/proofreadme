//! `cargo xtask package`: builds ProofReadMe in release and produces the package of the host OS in
//! `dist/`, either the `ProofReadMe.app` bundle (macOS) or `ProofReadMe.exe` (Windows).
//!
//! The macOS bundle is signed with a self-signed certificate of the keychain (ADR 0004): the
//! Accessibility permission is tied to the signature, and a stable certificate keeps it granted
//! across updates. No Developer ID, no notarisation. On macOS `--dmg` also wraps the bundle in a
//! disk image, `dist/ProofReadMe.dmg`; on Windows `--msi` also wraps the executable in an
//! installer, `dist/ProofReadMe.msi`, built with WiX.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The name of the application: that of the bundle, of its executable, of its icon and of the
/// `.exe`.
const NAME: &str = "ProofReadMe";

/// The name of the executable cargo builds.
const BINARY: &str = "proofreadme";

/// The identifier of the macOS bundle. It is also the `Label` of the LaunchAgent the application
/// registers (`proofreadme::IDENTIFIER`), and the AppUserModelID its Windows toasts are sent under:
/// a test checks that they stay equal.
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

/// The option that asks for a universal macOS bundle (Apple silicon and Intel).
const UNIVERSAL: &str = "--universal";

/// The option that asks for a macOS disk image holding the bundle.
const DMG: &str = "--dmg";

/// The option that asks for a Windows installer holding the executable.
const MSI: &str = "--msi";

/// The two architectures of a universal macOS executable.
const MACOS_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// The version of the WiX toolset that builds the installer (`dotnet tool install --global wix`),
/// and of its extension that closes the running agent.
const WIX_VERSION: &str = "5.0.2";

/// The upgrade code of the installer: the identity shared by all its versions, through which a
/// new version replaces the installed one. It must never change.
const UPGRADE_CODE: &str = "bbaac658-803d-4940-a7e2-9a4c3f298bb8";

/// The identity of the installed executable. A per-user file is tracked through a registry value,
/// which keeps WiX from deriving this identity by itself; like the upgrade code, it never changes.
const EXECUTABLE_COMPONENT: &str = "61649b70-3b13-4f29-ae6c-d289cd00531e";

/// The manufacturer the installer declares, shown in "Installed apps".
const MANUFACTURER: &str = "Jordan Marques";

/// How many times the disk image is attempted: `hdiutil` sometimes fails with "Resource busy" on
/// a machine that is indexing the staging folder.
const DMG_ATTEMPTS: u32 = 3;

/// The share of the side left transparent around the tile in the icon of the bundle, as macOS
/// expects of its application icons.
const ICON_MARGIN: f32 = 0.1;

/// What `cargo xtask package` was asked for.
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    ad_hoc: bool,
    universal: bool,
    dmg: bool,
    msi: bool,
}

/// How the macOS bundle is signed.
#[derive(Debug, PartialEq, Eq)]
enum Signature {
    /// With the self-signed certificate of this name, from the keychain (ADR 0004).
    Certificate(String),
    /// Ad hoc, with no identity: the Accessibility permission is lost at every update. Only on
    /// request (`--ad-hoc`), for a machine without the certificate.
    AdHoc,
}

/// Builds ProofReadMe in release, then produces the package of the host OS in `dist/`.
///
/// On macOS the bundle is signed with the certificate named by `PROOFREADME_SIGNING_IDENTITY`
/// (`ProofReadMe Signing` by default), or ad hoc with `--ad-hoc`; `--universal` gives it one
/// executable for both architectures (both Rust targets must be installed), and `--dmg` also
/// writes `dist/ProofReadMe.dmg`, a disk image holding the bundle next to a link to
/// `/Applications`. On Windows, `--msi` also writes `dist/ProofReadMe.msi`, the installer of the
/// executable (the `wix` tool must be installed).
pub fn package(arguments: &[String]) -> Result<(), String> {
    let options = options(arguments)?;
    let signature = signature(&options, std::env::var(IDENTITY_VARIABLE).ok());
    let root = crate::root();
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let dist = root.join("dist");
    let product = if cfg!(target_os = "macos") {
        if options.msi {
            return Err(format!("package: {MSI} only exists on Windows"));
        }
        // Checked before the build: a missing certificate is known at once.
        if let Signature::Certificate(name) = &signature {
            certificate_in_the_keychain(name)?;
        }
        let executable = if options.universal {
            universal_executable(&target)?
        } else {
            build(None)?;
            target.join("release").join(BINARY)
        };
        let bundle = assemble_the_bundle(&dist, &executable, env!("CARGO_PKG_VERSION"))?;
        sign(&bundle, &signature)?;
        if options.dmg { disk_image(&dist, &bundle, &target.join("dmg"))? } else { bundle }
    } else if cfg!(windows) {
        if options.ad_hoc || options.universal || options.dmg {
            return Err(format!("package: {AD_HOC}, {UNIVERSAL} and {DMG} only exist on macOS"));
        }
        build(None)?;
        let exe = copy_the_exe(&dist, &target.join("release").join(format!("{BINARY}.exe")))?;
        if options.msi { installer(&dist, &exe, &target.join("msi"), env!("CARGO_PKG_VERSION"))? } else { exe }
    } else {
        return Err("package: only macOS and Windows have a package".to_owned());
    };
    eprintln!("xtask package: {}", product.display());
    Ok(())
}

/// Reads the arguments of `package`: any of `--ad-hoc`, `--universal`, `--dmg` and `--msi`, each
/// at most once.
fn options(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    for argument in arguments {
        let option = match argument.as_str() {
            AD_HOC => &mut options.ad_hoc,
            UNIVERSAL => &mut options.universal,
            DMG => &mut options.dmg,
            MSI => &mut options.msi,
            unknown => return Err(format!("package: unknown argument `{unknown}`; the options are {AD_HOC}, {UNIVERSAL}, {DMG} and {MSI}")),
        };
        if std::mem::replace(option, true) {
            return Err(format!("package: `{argument}` is given twice"));
        }
    }
    Ok(options)
}

/// How the bundle is to be signed, after the options of `package` and the value of
/// [`IDENTITY_VARIABLE`]. An empty variable counts as unset.
fn signature(options: &Options, variable: Option<String>) -> Signature {
    if options.ad_hoc {
        return Signature::AdHoc;
    }
    let name = variable.map(|name| name.trim().to_owned()).filter(|name| !name.is_empty());
    Signature::Certificate(name.unwrap_or_else(|| DEFAULT_IDENTITY.to_owned()))
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
/// signature of the executable does not, and it is required after `lipo` rewrote the executable.
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

/// `cargo build --release` of the application, for the given Rust target or, without one, for the
/// architecture of the machine.
fn build(rust_target: Option<&str>) -> Result<(), String> {
    let mut command = crate::cargo();
    command.args(["build", "--release", "--locked", "-p", BINARY]);
    let mut description = format!("cargo build --release --locked -p {BINARY}");
    if let Some(rust_target) = rust_target {
        command.args(["--target", rust_target]);
        description.push_str(&format!(" --target {rust_target}"));
    }
    if cfg!(target_os = "macos") {
        command.env("MACOSX_DEPLOYMENT_TARGET", MACOS_MINIMUM);
    }
    crate::run(&mut command, &description)
}

/// Builds the application for both macOS architectures and merges the two executables into one
/// with `lipo`. Returns the path of the universal executable.
fn universal_executable(target: &Path) -> Result<PathBuf, String> {
    let mut slices = Vec::new();
    for rust_target in MACOS_TARGETS {
        build(Some(rust_target))?;
        slices.push(target.join(rust_target).join("release").join(BINARY));
    }
    let folder = target.join("universal");
    fs::create_dir_all(&folder).map_err(|e| format!("creating {}: {e}", folder.display()))?;
    let universal = folder.join(BINARY);
    let mut lipo = Command::new("lipo");
    lipo.arg("-create").arg("-output").arg(&universal).args(&slices);
    crate::run(&mut lipo, "lipo -create (universal executable)")?;
    Ok(universal)
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

/// Lays out what the disk image shows when it is opened: the bundle and a link to `/Applications`
/// to drag it onto. A previous staging folder is replaced.
fn stage_the_disk_image(staging: &Path, bundle: &Path) -> Result<(), String> {
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|e| format!("removing {}: {e}", staging.display()))?;
    }
    fs::create_dir_all(staging).map_err(|e| format!("creating {}: {e}", staging.display()))?;
    // `ditto` copies the bundle with its signature, permissions and links intact.
    let mut ditto = Command::new("ditto");
    ditto.arg(bundle).arg(staging.join(format!("{NAME}.app")));
    crate::run(&mut ditto, "ditto (bundle into the disk image staging folder)")?;
    link_to_applications(&staging.join("Applications"))
}

#[cfg(unix)]
fn link_to_applications(link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink("/Applications", link).map_err(|e| format!("linking {}: {e}", link.display()))
}

/// Outside Unix there is no disk image to stage.
#[cfg(not(unix))]
fn link_to_applications(_link: &Path) -> Result<(), String> {
    Err("package: a disk image can only be staged on macOS".to_owned())
}

/// Writes `dist/ProofReadMe.dmg`, a compressed read-only disk image of the bundle, and returns its
/// path. A previous image is replaced.
fn disk_image(dist: &Path, bundle: &Path, staging: &Path) -> Result<PathBuf, String> {
    stage_the_disk_image(staging, bundle)?;
    let image = dist.join(format!("{NAME}.dmg"));
    let mut last_error = String::new();
    for attempt in 1..=DMG_ATTEMPTS {
        let mut hdiutil = Command::new("hdiutil");
        hdiutil.args(["create", "-volname", NAME, "-fs", "HFS+", "-format", "UDZO", "-ov", "-srcfolder"]).arg(staging).arg(&image);
        match crate::run(&mut hdiutil, &format!("hdiutil create (attempt {attempt} of {DMG_ATTEMPTS})")) {
            Ok(()) => return Ok(image),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

/// Copies the Windows executable to `dist/ProofReadMe.exe` and returns this path.
fn copy_the_exe(dist: &Path, executable: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dist).map_err(|e| format!("creating {}: {e}", dist.display()))?;
    let exe = dist.join(format!("{NAME}.exe"));
    copy(executable, &exe)?;
    Ok(exe)
}

/// Writes `dist/ProofReadMe.msi`, the installer of the given executable, and returns its path. The
/// source of the installer and the icon it shows are written to the staging folder first. A
/// previous installer is replaced.
fn installer(dist: &Path, exe: &Path, staging: &Path, version: &str) -> Result<PathBuf, String> {
    let msi_version = msi_version(version)?;
    fs::create_dir_all(staging).map_err(|e| format!("creating {}: {e}", staging.display()))?;
    let source = staging.join(format!("{NAME}.wxs"));
    let icon = staging.join(format!("{NAME}.ico"));
    write(&source, wxs(&msi_version).as_bytes())?;
    write(&icon, &proofreadme_icon::ico())?;
    // Adding the extension again is harmless: it is how a fresh machine gets it.
    let extension = format!("WixToolset.Util.wixext/{WIX_VERSION}");
    let mut add = Command::new("wix");
    add.args(["extension", "add", "--global", &extension]);
    crate::run(&mut add, &format!("wix extension add --global {extension}"))
        .map_err(|error| format!("{error}\nthe installer is built with WiX: dotnet tool install --global wix --version {WIX_VERSION}"))?;
    let msi = dist.join(format!("{NAME}.msi"));
    let mut wix = Command::new("wix");
    wix.args(["build", "-arch", "x64", "-pdbtype", "none", "-ext", &extension])
        .arg("-d")
        .arg(format!("Exe={}", exe.display()))
        .arg("-d")
        .arg(format!("Icon={}", icon.display()))
        .arg("-o")
        .arg(&msi)
        .arg(&source);
    crate::run(&mut wix, "wix build (installer)")?;
    Ok(msi)
}

/// The version of the installer. Windows Installer only knows `major.minor.build`, at most 255,
/// 255 and 65535: a pre-release (`1.2.0-rc.1`) takes the version it precedes.
fn msi_version(version: &str) -> Result<String, String> {
    let release = version.split(['-', '+']).next().unwrap_or(version);
    let fields: Vec<&str> = release.split('.').collect();
    let within = |field: &&str, limit: u32| field.parse::<u32>().is_ok_and(|number| number <= limit);
    if fields.len() == 3 && fields.iter().zip([255, 255, 65535]).all(|(field, limit)| within(field, limit)) {
        Ok(release.to_owned())
    } else {
        Err(format!("package: `{version}` cannot be the version of a Windows installer (at most 255.255.65535)"))
    }
}

/// The source of the installer, in the WiX format. It installs for the current user only, without
/// administrator rights, in `%LOCALAPPDATA%\Programs\ProofReadMe`: the stable path the registration
/// at login records. A shortcut goes in the Start menu, carrying the AppUserModelID the toasts are
/// sent under. A newer version replaces the installed one.
///
/// The running agent holds its executable open: it is stopped before the files are replaced, then
/// launched again once an interactive installation is done, in the background after an update. On
/// uninstall, its registration at login goes with it.
fn wxs(msi_version: &str) -> String {
    format!(
        r#"<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs" xmlns:util="http://wixtoolset.org/schemas/v4/wxs/util">
  <Package Name="{NAME}" Manufacturer="{MANUFACTURER}" Version="{msi_version}" UpgradeCode="{UPGRADE_CODE}" Scope="perUser">
    <MajorUpgrade AllowSameVersionUpgrades="yes" DowngradeErrorMessage="A newer version of {NAME} is already installed." />
    <MediaTemplate EmbedCab="yes" />

    <Icon Id="{NAME}.ico" SourceFile="$(Icon)" />
    <Property Id="ARPPRODUCTICON" Value="{NAME}.ico" />
    <Property Id="ARPNOMODIFY" Value="1" />

    <StandardDirectory Id="LocalAppDataFolder">
      <Directory Id="ProgramsFolder" Name="Programs">
        <Directory Id="INSTALLFOLDER" Name="{NAME}">
          <Component Id="Executable" Guid="{EXECUTABLE_COMPONENT}">
            <File Id="Executable" Source="$(Exe)" Name="{NAME}.exe" KeyPath="no" />
            <RegistryValue Root="HKCU" Key="Software\{NAME}\Installer" Name="Executable" Type="integer" Value="1" KeyPath="yes" />
            <RemoveFolder Id="RemoveInstallFolder" On="uninstall" />
            <RemoveFolder Id="RemoveProgramsFolder" Directory="ProgramsFolder" On="uninstall" />
          </Component>
        </Directory>
      </Directory>
    </StandardDirectory>
    <StandardDirectory Id="ProgramMenuFolder">
      <Component Id="Shortcut">
        <Shortcut Id="Shortcut" Name="{NAME}" Target="[INSTALLFOLDER]{NAME}.exe" WorkingDirectory="INSTALLFOLDER" Icon="{NAME}.ico">
          <ShortcutProperty Key="System.AppUserModel.ID" Value="{IDENTIFIER}" />
        </Shortcut>
        <RegistryValue Root="HKCU" Key="Software\{NAME}\Installer" Name="Shortcut" Type="integer" Value="1" KeyPath="yes" />
      </Component>
    </StandardDirectory>

    <util:CloseApplication Id="StopTheAgent" Target="{NAME}.exe" TerminateProcess="0" RebootPrompt="no" />

    <CustomAction Id="Launch" FileRef="Executable" ExeCommand=" " Return="asyncNoWait" />
    <CustomAction Id="LaunchInTheBackground" FileRef="Executable" ExeCommand="--background" Return="asyncNoWait" />
    <CustomAction Id="Unregister" Directory="INSTALLFOLDER" ExeCommand='"[System64Folder]reg.exe" delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v {NAME} /f' Execute="deferred" Impersonate="yes" Return="ignore" />
    <InstallExecuteSequence>
      <Custom Action="Unregister" Before="RemoveFiles" Condition='REMOVE="ALL" AND NOT UPGRADINGPRODUCTCODE' />
      <Custom Action="Launch" After="InstallFinalize" Condition="NOT REMOVE AND UILevel &gt;= 3 AND NOT WIX_UPGRADE_DETECTED" />
      <Custom Action="LaunchInTheBackground" After="InstallFinalize" Condition="NOT REMOVE AND UILevel &gt;= 3 AND WIX_UPGRADE_DETECTED" />
    </InstallExecuteSequence>

    <Feature Id="Main">
      <ComponentRef Id="Executable" />
      <ComponentRef Id="Shortcut" />
    </Feature>
  </Package>
</Wix>
"#
    )
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
        let options = Options::default();

        assert_eq!(signature(&options, None), Signature::Certificate("ProofReadMe Signing".to_owned()));
        assert_eq!(signature(&options, Some("Jordan Code".to_owned())), Signature::Certificate("Jordan Code".to_owned()));
        assert_eq!(signature(&options, Some("  ".to_owned())), Signature::Certificate("ProofReadMe Signing".to_owned()));
    }

    #[test]
    fn the_ad_hoc_signature_is_only_given_on_request() {
        let options = options(&arguments(&["--ad-hoc"])).unwrap();

        assert_eq!(signature(&options, Some("Jordan Code".to_owned())), Signature::AdHoc);
    }

    #[test]
    fn package_takes_its_options_in_any_order() {
        assert_eq!(options(&[]), Ok(Options::default()));
        assert_eq!(options(&arguments(&["--universal"])), Ok(Options { universal: true, ..Options::default() }));
        assert_eq!(options(&arguments(&["--dmg", "--universal"])), Ok(Options { universal: true, dmg: true, ..Options::default() }));
        assert_eq!(options(&arguments(&["--dmg", "--ad-hoc", "--universal"])), Ok(Options { ad_hoc: true, universal: true, dmg: true, msi: false }));
        assert_eq!(options(&arguments(&["--msi"])), Ok(Options { msi: true, ..Options::default() }));
    }

    #[test]
    fn package_refuses_an_unknown_or_repeated_argument() {
        assert!(options(&arguments(&["--adhoc"])).unwrap_err().contains("--adhoc"));
        assert!(options(&arguments(&["--universal", "extra"])).is_err());
        for repeated in ["--ad-hoc", "--universal", "--dmg", "--msi"] {
            assert!(options(&arguments(&[repeated, repeated])).is_err(), "{repeated}");
        }
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

    #[test]
    fn the_installer_version_is_the_release_the_version_designates() {
        assert_eq!(msi_version("1.2.3"), Ok("1.2.3".to_owned()));
        assert_eq!(msi_version("0.1.1"), Ok("0.1.1".to_owned()));
        assert_eq!(msi_version("1.2.0-rc.1"), Ok("1.2.0".to_owned()), "a pre-release takes the version it precedes");
        assert_eq!(msi_version("255.255.65535"), Ok("255.255.65535".to_owned()));
    }

    #[test]
    fn a_version_windows_installer_cannot_carry_is_refused() {
        for version in ["256.0.0", "1.256.0", "1.2.65536", "1.2", "1.2.3.4", "1.x.3"] {
            assert!(msi_version(version).is_err(), "{version}");
        }
    }

    #[test]
    fn the_installer_keeps_its_upgrade_code_and_installs_for_the_user_where_the_login_registration_points() {
        let source = wxs("1.2.3");

        assert!(source.contains(r#"UpgradeCode="bbaac658-803d-4940-a7e2-9a4c3f298bb8""#), "the identity across versions never changes");
        assert!(source.contains(r#"Version="1.2.3""#));
        assert!(source.contains(r#"Scope="perUser""#), "no administrator rights");
        assert!(source.contains("<MajorUpgrade "), "a new version replaces the installed one");
        // %LOCALAPPDATA%\Programs\ProofReadMe\ProofReadMe.exe, the path the README names.
        assert!(source.contains(r#"<StandardDirectory Id="LocalAppDataFolder">"#));
        assert!(source.contains(r#"<Directory Id="ProgramsFolder" Name="Programs">"#));
        assert!(source.contains(r#"<Directory Id="INSTALLFOLDER" Name="ProofReadMe">"#));
        assert!(source.contains(r#"Name="ProofReadMe.exe""#));
        // On uninstall, the value the agent writes to the `Run` key goes too.
        assert!(source.contains(r"CurrentVersion\Run /v ProofReadMe /f"));
    }

    #[test]
    fn the_start_menu_shortcut_carries_the_app_user_model_id_of_the_toasts() {
        let source = wxs("1.2.3");

        assert!(source.contains(r#"<ShortcutProperty Key="System.AppUserModel.ID" Value="io.github.jordanmarques.proofreadme" />"#));
        // The toasts are sent under `crate::IDENTIFIER`, the constant checked against IDENTIFIER above.
        let notifications = fs::read_to_string(crate::root().join("apps/proofreadme/src/notifications.rs")).unwrap();
        assert!(notifications.contains("Ok(()) => Some(crate::IDENTIFIER)"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_disk_image_shows_the_bundle_next_to_a_link_to_applications() {
        let folder = tempfile::tempdir().unwrap();
        let bundle = assemble_the_bundle(&folder.path().join("dist"), &fake_executable(folder.path(), "proofreadme"), "1.2.3").unwrap();
        let staging = folder.path().join("staging");
        fs::create_dir_all(staging.join("left over from a previous run")).unwrap();

        stage_the_disk_image(&staging, &bundle).unwrap();

        let mut shown: Vec<String> = fs::read_dir(&staging).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        shown.sort();
        assert_eq!(shown, ["Applications", "ProofReadMe.app"]);
        assert_eq!(fs::read_link(staging.join("Applications")).unwrap(), Path::new("/Applications"));
        assert_eq!(fs::read(staging.join("ProofReadMe.app/Contents/MacOS/ProofReadMe")).unwrap(), b"test binary");
    }
}
