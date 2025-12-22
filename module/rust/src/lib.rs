mod api;
mod binding;
#[doc(hidden)]
pub mod macros;
mod module;

#[macro_use]
extern crate log;
#[cfg(target_os = "android")]
extern crate android_logger;

#[cfg(target_os = "android")]
use android_logger::Config;
#[cfg(target_os = "android")]
use log::Level;

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::ptr;

pub use api::ZygiskApi;
pub use binding::{AppSpecializeArgs, ServerSpecializeArgs, StateFlags, ZygiskOption, API_VERSION};
use jni::JNIEnv;
pub use module::ZygiskModule;

// Path Config & Loader
const CONFIG_PATH: &str = "/data/adb/modules/zygisk-loader/active_config.txt";
const PAYLOAD_PATH: &str = "/data/adb/modules/zygisk-loader/payload.so";

static MODULE: ZygiskLoaderModule = ZygiskLoaderModule {};
crate::zygisk_module!(&MODULE);

struct ZygiskLoaderModule {}

impl ZygiskModule for ZygiskLoaderModule {
    fn on_load(&self, api: ZygiskApi, _env: JNIEnv) {
        #[cfg(target_os = "android")]
        android_logger::init_once(
            Config::default()
                .with_min_level(Level::Info)
                .with_tag("Zygisk_Loader"),
        );

        info!("Zygisk-Loader Loaded. Waiting for target app...");
    }

    fn post_app_specialize(&self, _api: ZygiskApi, _args: &AppSpecializeArgs) {
        let current_process = match get_process_name() {
            Ok(name) => name,
            Err(_) => return,
        };

        let target_package = match read_target_config() {
            Ok(target) => target,
            Err(e) => {
                return;
            }
        };

        if current_process.trim() == target_package.trim() {
            info!("TARGET DETECTED: {}", current_process);
            info!("Injecting Payload from: {}", PAYLOAD_PATH);

            unsafe {
                inject_payload(PAYLOAD_PATH);
            }
        }
    }
}

// Helper Function

fn get_process_name() -> std::io::Result<String> {
    let mut f = File::open("/proc/self/cmdline")?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;

    let name = buffer.split(|&c| c == 0)
        .next()
        .and_then(|slice| String::from_utf8(slice.to_vec()).ok())
        .unwrap_or_default();

    Ok(name)
}

fn read_target_config() -> std::io::Result<String> {
    let f = File::open(CONFIG_PATH)?;
    let mut reader = BufReader::new(f);
    let mut line = String::new();

    reader.read_line(&mut line)?;

    Ok(line.trim().to_string())
}

unsafe fn inject_payload(path: &str) {
    let c_path = CString::new(path).unwrap();

    let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW);

    if handle.is_null() {
        let err_ptr = libc::dlerror();
        if !err_ptr.is_null() {
            let err_msg = CStr::from_ptr(err_ptr).to_string_lossy();
            error!("❌ GAGAL LOAD PAYLOAD: {}", err_msg);
        } else {
            error!("❌ GAGAL LOAD PAYLOAD: Unknown error");
        }
    } else {
        info!("✅ PAYLOAD BERHASIL DIMUAT! Handle: {:p}", handle);
    }
}

#[cfg(test)]
mod test {
    use std::os::unix::io::RawFd;
    fn companion(_socket: RawFd) {}
    crate::zygisk_companion!(companion);
}
