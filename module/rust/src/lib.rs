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
                .with_min_level(Level::Debug) // Changed to Debug for more detailed logs
                .with_tag("Zygisk_Loader"),
        );
        info!("Zygisk-Loader Loaded (on_load).");
    }

    fn post_app_specialize(&self, _api: ZygiskApi, _args: &AppSpecializeArgs) {
        // 1. Get Process Name
        let current_process = match get_process_name() {
            Ok(name) => name,
            Err(e) => {
                error!("Failed to read /proc/self/cmdline: {:?}", e);
                return;
            }
        };

        // (This will spam logcat a bit, but important for diagnosis)
        debug!("Checking process: '{}'", current_process);

        // 2. Read Target Config
        let target_package = match read_target_config() {
            Ok(target) => target,
            Err(e) => {
                // if error here, it means permission/SELinux issue
                error!("Failed to read config in {}: {:?}", CONFIG_PATH, e);
                return;
            }
        };

        if current_process.contains(target_package.trim()) {
            info!("Target Match! Process: '{}' matches Target: '{}'", current_process, target_package);
            info!("Attempting Injection: {}", PAYLOAD_PATH);

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
            error!("Fail Load Payload: {}", err_msg);
        } else {
            error!("Fail Load Payload: Unknown error");
        }
    } else {
        info!("Payload successfully loaded! Handle: {:p}", handle);
    }
}

#[cfg(test)]
mod test {
    use std::os::unix::io::RawFd;
    fn companion(_socket: RawFd) {}
    crate::zygisk_companion!(companion);
}
