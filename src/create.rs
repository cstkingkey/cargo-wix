// Copyright (C) 2017 Christopher R. Field.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use BINARY_FOLDER_NAME;
use CARGO;
use Cultures;
use Error;
use EXE_FILE_EXTENSION;
use Platform;
use Result;
use semver::Version;
use std::env;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use toml::Value;
use WIX;
use WIX_COMPILER;
use WIX_LINKER;
use WIX_PATH_KEY;
use WIX_SOURCE_FILE_EXTENSION;
use WIX_SOURCE_FILE_NAME;

/// A builder for running the subcommand.
#[derive(Debug, Clone)]
pub struct Builder<'a> {
    bin_path: Option<&'a str>,
    capture_output: bool,
    culture: Cultures,
    input: Option<&'a str>,
    locale: Option<&'a str>,
    name: Option<&'a str>,
    no_build: bool,
    output: Option<&'a str>,
    version: Option<&'a str>,
}

impl<'a> Builder<'a> {
    /// Creates a new `Wix` instance.
    pub fn new() -> Self {
        Builder {
            bin_path: None,
            capture_output: true,
            culture: Cultures::EnUs,
            input: None,
            locale: None,
            name: None,
            no_build: false,
            output: None,
            version: None,
        }
    }

    /// Sets the path to the WiX Toolset's `bin` folder.
    ///
    /// The WiX Toolset's `bin` folder should contain the needed `candle.exe` and `light.exe`
    /// applications. The default is to use the PATH system environment variable. This will
    /// override any value obtained from the environment.
    pub fn bin_path(&mut self, b: Option<&'a str>) -> &mut Self {
        self.bin_path = b;
        self
    }

    /// Enables or disables capturing of the output from the builder (`cargo`), compiler
    /// (`candle`), linker (`light`), and signer (`signtool`).
    ///
    /// The default is to capture all output, i.e. display nothing in the console but the log
    /// statements.
    pub fn capture_output(&mut self, c: bool) -> &mut Self {
        self.capture_output = c;
        self
    }

    /// Sets the culture to use with the linker (light.exe) for building a localized installer.
    pub fn culture(&mut self, c: Cultures) -> &mut Self {
        self.culture = c;
        self
    }

    /// Sets the path to a file to be used as the WiX Source (wxs) file instead of `wix\main.rs`.
    pub fn input(&mut self, i: Option<&'a str>) -> &mut Self {
        self.input = i;
        self
    }

    /// Sets the path to a WiX localization file, `.wxl`, for the linker (light.exe).
    ///
    /// The [WiX localization
    /// file](http://wixtoolset.org/documentation/manual/v3/howtos/ui_and_localization/make_installer_localizable.html)
    /// is an XML file that contains localization strings.
    pub fn locale(mut self, l: Option<&'a str>) -> Self {
        self.locale = l;
        self
    }

    /// Sets the name.
    ///
    /// The default is to use the `name` field under the `package` section of the package's
    /// manifest (Cargo.toml). This overrides that value. An error occurs if the `name` field is
    /// not found in the manifest. 
    ///
    /// The installer (msi) that is created will be named as "name-major.minor.patch-platform.msi" format,
    /// where name is the value specified with this method or the value from the `name` field under
    /// the `package` section, the major.minor.patch is the version number from the package's
    /// manifest `version` field or the value specified at the command line, and the _platform_ is
    /// either "i686" or "x86_64" depending on the build environment.
    ///
    /// This does __not__ change the name of the executable that is installed. The name of the
    /// executable can be changed by modifying the WiX Source (wxs) file with a text editor.
    pub fn name(&mut self, p: Option<&'a str>) -> &mut Self {
        self.name = p;
        self
    }

    /// Skips the building of the project with the release profile.
    ///
    /// If `true`, the project will _not_ be built using the release profile, i.e. the `cargo build
    /// --release` command will not be executed. The default is to build the project before each
    /// creation. This is useful if building the project is more involved or is handled in
    /// a separate process.
    pub fn no_build(&mut self, n: bool) -> &mut Self {
        self.no_build = n;
        self
    }

    /// Sets the output file.
    ///
    /// The default is to create a MSI file with the `<product-name>-<version>-<arch>.msi` file
    /// name and extension in the `target\wix` folder. Use this method to override the destination
    /// and file name of the Windows installer.
    pub fn output(&mut self, o: Option<&'a str>) -> &mut Self {
        self.output = o;
        self
    }

    /// Sets the version.
    ///
    /// This overrides the default where the version is obtained from the `version` field of the
    /// package's manifest (Cargo.toml). The version should be in the "Major.Minor.Patch" notation.
    pub fn version(&mut self, v: Option<&'a str>) -> &mut Self {
        self.version = v;
        self
    }

    /// Builds the project using the release profile, creates the installer (msi), and optionally
    /// signs the output. 
    pub fn build(&mut self) -> Execution {
        Execution {
            bin_path: self.bin_path.map(PathBuf::from),
            capture_output: self.capture_output,
            culture: self.culture.clone(),
            input: self.input.map(PathBuf::from),
            locale: self.locale.map(PathBuf::from),
            name: self.name.map(String::from),
            no_build: self.no_build,
            output: self.output.map(PathBuf::from),
            version: self.version.map(String::from),
        }
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Builder::new()
    }
}

#[derive(Debug)]
pub struct Execution {
    bin_path: Option<PathBuf>,
    capture_output: bool,
    culture: Cultures,
    input: Option<PathBuf>,
    locale: Option<PathBuf>,
    name: Option<String>,
    no_build: bool,
    output: Option<PathBuf>,
    version: Option<String>,
}

impl Execution {
    pub fn run(self) -> Result<()> {
        debug!("bin_path = {:?}", self.bin_path);
        debug!("capture_output = {:?}", self.capture_output);
        debug!("culture = {:?}", self.culture);
        debug!("input = {:?}", self.input);
        debug!("locale = {:?}", self.locale);
        debug!("name = {:?}", self.name);
        debug!("no_build = {:?}", self.no_build);
        debug!("output = {:?}", self.output);
        debug!("version = {:?}", self.version);
        let manifest = super::manifest(self.input.as_ref())?;
        let name = self.name(&manifest)?;
        debug!("name = {:?}", name);
        let version = self.version(&manifest)?;
        debug!("version = {:?}", version);
        let locale = self.locale()?;
        debug!("locale = {:?}", locale);
        let platform = self.platform();
        debug!("platform = {:?}", platform);
        let source_wxs = self.wxs_source()?;
        debug!("source_wxs = {:?}", source_wxs);
        let source_wixobj = self.source_wixobj(); 
        debug!("source_wixobj = {:?}", source_wixobj);
        let destination_msi = self.destination_msi(&name, &version, &platform);
        debug!("destination_msi = {:?}", destination_msi);
        if self.no_build {
            warn!("Skipped building the release binary");
        } else {
            // Build the binary with the release profile. If a release binary has already been built, then
            // this will essentially do nothing.
            info!("Building the release binary");
            let mut builder = Command::new(CARGO);
            debug!("builder = {:?}", builder);
            if self.capture_output {
                trace!("Capturing the '{}' output", CARGO);
                builder.stdout(Stdio::null());
                builder.stderr(Stdio::null());
            }
            let status = builder.arg("build").arg("--release").status()?;
            if !status.success() {
                return Err(Error::Command(CARGO, status.code().unwrap_or(100)));
            }
        }
        // Compile the installer
        info!("Compiling the installer");
        let mut compiler = self.compiler()?;
        debug!("compiler = {:?}", compiler);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_COMPILER);
            compiler.stdout(Stdio::null());
            compiler.stderr(Stdio::null());
        } 
        compiler.arg(format!("-dVersion={}", version))
            .arg(format!("-dPlatform={}", platform))
            .arg("-o")
            .arg(&source_wixobj)
            .arg(&source_wxs);
        debug!("command = {:?}", compiler);
        let status = compiler.status().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                Error::Generic(format!(
                    "The compiler application ({}) could not be found in the PATH environment \
                    variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                    installed and check the WiX Toolset's '{}' folder has been added to the PATH \
                    system environment variable, the {} system environment variable exists, or use \
                    the '-B,--bin-path' command line argument.", 
                    WIX_COMPILER,
                    BINARY_FOLDER_NAME,
                    WIX_PATH_KEY
                ))
            } else {
                err.into()
            }
        })?;
        if !status.success() {
            return Err(Error::Command(WIX_COMPILER, status.code().unwrap_or(100)));
        }
        // Link the installer
        info!("Linking the installer");
        let mut linker = self.linker()?; 
        debug!("linker = {:?}", linker);
        if self.capture_output {
            trace!("Capturing the '{}' output", WIX_LINKER);
            linker.stdout(Stdio::null());
            linker.stderr(Stdio::null());
        }
        if let Some(l) = locale {
            trace!("Using the a WiX localization file");
            linker.arg("-loc").arg(l);
        }
        linker.arg("-ext")
            .arg("WixUIExtension")
            .arg(format!("-cultures:{}", self.culture)) 
            .arg(&source_wixobj)
            .arg("-out")
            .arg(&destination_msi);
        debug!("command = {:?}", linker);
        let status = linker.status().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                Error::Generic(format!(
                    "The linker application ({}) could not be found in the PATH environment \
                    variable. Please check the WiX Toolset (http://wixtoolset.org/) is \
                    installed and check the WiX Toolset's '{}' folder has been added to the PATH \
                    environment variable, the {} system environment variable exists, or use the \
                    '-B,--bin-path' command line argument.", 
                    WIX_LINKER,
                    BINARY_FOLDER_NAME,
                    WIX_PATH_KEY
                ))
            } else {
                err.into()
            }
        })?;
        if !status.success() {
            return Err(Error::Command(WIX_LINKER, status.code().unwrap_or(100)));
        }
        Ok(())
    }

    /// Gets the command for the compiler application (`candle.exe`).
    fn compiler(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.as_ref().map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset's '{}' folder for the compiler", 
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(WIX_COMPILER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the `candle` application from the path
                Err(Error::Generic(format!(
                    "The compiler application ('{}') does not exist at the '{}' path specified via \
                    the '-B, --bin-path' command line argument. Please check the path is correct and \
                    the compiler application exists at the path.",
                    WIX_COMPILER, 
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
                let mut p = PathBuf::from(s);
                trace!(
                    "Using the '{}' path to the WiX Toolset's '{}' folder for the compiler", 
                    p.display(), 
                    BINARY_FOLDER_NAME
                );
                p.push(BINARY_FOLDER_NAME);
                p.push(WIX_COMPILER);
                p.set_extension(EXE_FILE_EXTENSION);
                p
            }) {
                if !path.exists() {
                    path.pop(); // Remove the `candle` application from the path
                    Err(Error::Generic(format!(
                        "The compiler application ('{}') does not exist at the '{}' path specified \
                        via the {} environment variable. Please check the path is correct and the \
                        compiler application exists at the path.",
                        WIX_COMPILER,
                        path.display(),
                        WIX_PATH_KEY
                    )))
                } else {
                    Ok(Command::new(path))
                }
            } else {
                Ok(Command::new(WIX_COMPILER))
            }
        }
    }

    /// Gets the WiX localization file and checks if it exists.
    ///
    /// Returns `None` if no localization file is specified.
    fn locale(&self) -> Result<Option<PathBuf>> {
        if let Some(locale) = self.locale.as_ref().map(PathBuf::from) {
            if locale.exists() {
                Ok(Some(locale))
            } else {
                Err(Error::Generic(format!(
                    "The '{}' WiX localization file could not be found, or it does not exist. \
                    Please check the path is correct and the file exists.",
                    locale.display()
                )))
            }
        } else {
            Ok(None)
        }
    }

    /// Gets the command for the linker application (`light.exe`).
    fn linker(&self) -> Result<Command> {
        if let Some(mut path) = self.bin_path.as_ref().map(|s| {
            let mut p = PathBuf::from(s);
            trace!(
                "Using the '{}' path to the WiX Toolset '{}' folder for the linker", 
                p.display(),
                BINARY_FOLDER_NAME
            );
            p.push(WIX_LINKER);
            p.set_extension(EXE_FILE_EXTENSION);
            p
        }) {
            if !path.exists() {
                path.pop(); // Remove the 'light' application from the path
                Err(Error::Generic(format!(
                    "The linker application ('{}') does not exist at the '{}' path specified via \
                    the '-B,--bin-path' command line argument. Please check the path is correct \
                    and the linker application exists at the path.",
                    WIX_LINKER,
                    path.display()
                )))
            } else {
                Ok(Command::new(path))
            }
        } else {
            if let Some(mut path) = env::var_os(WIX_PATH_KEY).map(|s| {
                let mut p = PathBuf::from(s);
                trace!(
                    "Using the '{}' path to the WiX Toolset's '{}' folder for the linker", 
                    p.display(),
                    BINARY_FOLDER_NAME
                );
                p.push(BINARY_FOLDER_NAME);
                p.push(WIX_LINKER);
                p.set_extension(EXE_FILE_EXTENSION);
                p
            }) {
                if !path.exists() {
                    path.pop(); // Remove the `candle` application from the path
                    Err(Error::Generic(format!(
                        "The linker application ('{}') does not exist at the '{}' path specified \
                        via the {} environment variable. Please check the path is correct and the \
                        linker application exists at the path.",
                        WIX_LINKER,
                        path.display(),
                        WIX_PATH_KEY
                    )))
                } else {
                    Ok(Command::new(path))
                }
            } else {
                Ok(Command::new(WIX_LINKER))
            }
        }
    }

    /// Gets the platform.
    fn platform(&self) -> Platform {
        if cfg!(target_arch = "x86_64") {
            Platform::X64
        } else {
            Platform::X86
        }
    }

    fn name(&self, manifest: &Value) -> Result<String> {
        if let Some(ref p) = self.name {
            Ok(p.to_owned())
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
                .ok_or(Error::Manifest("name"))
        }
    }

    /// Gets the destination for the linker.
    fn destination_msi(&self, name: &str, version: &Version, platform: &Platform) -> PathBuf {
        if let Some(ref o) = self.output {
            PathBuf::from(o)
        } else {
            let mut destination_msi = PathBuf::from("target");
            destination_msi.push(WIX);
            // Do NOT use the `set_extension` method for the MSI path. Since the pkg_version is in X.X.X
            // format, the `set_extension` method will replace the Patch version number and
            // architecture/platform with `msi`.  Instead, just include the extension in the formatted
            // name.
            destination_msi.push(&format!("{}-{}-{}.msi", name, version, platform.arch()));
            destination_msi
        }
    }

    /// Gets the destination for the compiler output/linker input.
    fn source_wixobj(&self) -> PathBuf {
        let mut source_wixobj = PathBuf::from("target");
        source_wixobj.push(WIX);
        source_wixobj.push(WIX_SOURCE_FILE_NAME);
        source_wixobj.set_extension("wixobj");
        source_wixobj
    }

    /// Gets the WiX Source (wxs) file.
    fn wxs_source(&self) -> Result<PathBuf> {
        if let Some(p) = self.input.as_ref().map(|s| PathBuf::from(s)) {
            if p.exists() {
                if p.is_dir() {
                    Err(Error::Generic(format!(
                        "The '{}' path is not a file. Please check the path and ensure it is to \
                        a WiX Source (wxs) file.", 
                        p.display()
                    )))
                } else {
                    trace!("Using the '{}' WiX source file", p.display());
                    Ok(p)
                }
            } else {
                Err(Error::Generic(format!(
                    "The '{0}' file does not exist. Consider using the 'cargo wix --print-template \
                    WXS > {0}' command to create it.", 
                    p.display()
                )))
            }
        } else {
            trace!("Using the default WiX source file");
            let mut main_wxs = PathBuf::from(WIX);
            main_wxs.push(WIX_SOURCE_FILE_NAME);
            main_wxs.set_extension(WIX_SOURCE_FILE_EXTENSION);
            if main_wxs.exists() {
                Ok(main_wxs)
            } else {
               Err(Error::Generic(format!(
                   "The '{0}' file does not exist. Consider using the 'cargo wix --init' command to \
                   create it.", 
                   main_wxs.display()
               )))
            }
        }
    }

    /// Gets the package version.
    fn version(&self, manifest: &Value) -> Result<Version> {
        if let Some(ref v) = self.version {
            Version::parse(v).map_err(Error::from)
        } else {
            manifest.get("package")
                .and_then(|p| p.as_table())
                .and_then(|t| t.get("version"))
                .and_then(|v| v.as_str())
                .ok_or(Error::Manifest("version"))
                .and_then(|s| Version::parse(s).map_err(Error::from))
        }
    }
}

impl Default for Execution {
    fn default() -> Self {
        Builder::new().build()
    }
}
