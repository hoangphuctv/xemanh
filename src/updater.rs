//! Background update checking. The implementation is intentionally Windows-only:
//! the installer asset is a Windows executable and other platforms keep a no-op.

#[cfg(target_os = "windows")]
mod platform {
    use std::fs::{self, File};
    use std::io::copy;
    use std::path::PathBuf;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::{Duration, Instant};

    const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
    const RELEASE_API: &str = "https://api.github.com/repos/hoangphuctv/xemanh/releases/latest";
    const UPDATE_DELAY: Duration = Duration::from_secs(12);

    pub enum UpdatePoll {
        None,
        Downloading,
        LaunchInstaller,
        Error(String),
    }

    enum WorkerResult {
        NoUpdate,
        Available(Release),
        Downloaded(PathBuf),
        Failed(String),
    }

    struct Release {
        version: String,
        installer_url: String,
    }

    pub struct Updater {
        started_at: Instant,
        receiver: Option<Receiver<WorkerResult>>,
        checking_started: bool,
        downloading: bool,
    }

    impl Updater {
        pub fn new() -> Self {
            Self {
                started_at: Instant::now(),
                receiver: None,
                checking_started: false,
                downloading: false,
            }
        }

        pub fn poll(&mut self) -> UpdatePoll {
            if !self.checking_started && self.started_at.elapsed() >= UPDATE_DELAY {
                self.checking_started = true;
                self.receiver = Some(start_check_worker());
            }

            let Some(receiver) = &self.receiver else {
                return UpdatePoll::None;
            };
            let Ok(result) = receiver.try_recv() else {
                return if self.downloading {
                    UpdatePoll::Downloading
                } else {
                    UpdatePoll::None
                };
            };
            self.receiver = None;

            match result {
                WorkerResult::NoUpdate => UpdatePoll::None,
                WorkerResult::Available(release) => {
                    if ask_user_to_update(&release.version) {
                        self.downloading = true;
                        self.receiver = Some(start_worker(move || download_installer(release)));
                        UpdatePoll::Downloading
                    } else {
                        UpdatePoll::None
                    }
                }
                WorkerResult::Downloaded(installer) => {
                    self.downloading = false;
                    match std::process::Command::new(&installer).spawn() {
                        Ok(_) => UpdatePoll::LaunchInstaller,
                        Err(error) => UpdatePoll::Error(format!(
                            "Không thể mở trình cài đặt cập nhật: {error}"
                        )),
                    }
                }
                WorkerResult::Failed(error) => {
                    self.downloading = false;
                    UpdatePoll::Error(error)
                }
            }
        }
    }

    fn start_worker(
        work: impl FnOnce() -> Result<WorkerResult, String> + Send + 'static,
    ) -> Receiver<WorkerResult> {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = work().unwrap_or_else(WorkerResult::Failed);
            let _ = sender.send(result);
        });
        receiver
    }

    fn start_check_worker() -> Receiver<WorkerResult> {
        start_worker(|| match fetch_latest_release() {
            Ok(result) => Ok(result),
            // Checking is intentionally silent: a missing network connection or
            // GitHub outage must not interrupt image viewing.
            Err(_) => Ok(WorkerResult::NoUpdate),
        })
    }

    fn fetch_latest_release() -> Result<WorkerResult, String> {
        let response = ureq::get(RELEASE_API)
            .set("User-Agent", "XemAnh-updater")
            .call()
            .map_err(|error| format!("Không thể kiểm tra bản cập nhật: {error}"))?;
        let body = response
            .into_string()
            .map_err(|error| format!("Không thể đọc thông tin cập nhật: {error}"))?;
        let release: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| format!("Thông tin cập nhật không hợp lệ: {error}"))?;

        let version = release["tag_name"]
            .as_str()
            .unwrap_or_default()
            .trim_start_matches('v')
            .to_owned();
        if version.is_empty() || !is_newer_version(&version, CURRENT_VERSION) {
            return Ok(WorkerResult::NoUpdate);
        }

        let expected_name = format!("xemanh-{version}-setup.exe");
        let installer_url = release["assets"]
            .as_array()
            .and_then(|assets| {
                assets.iter().find_map(|asset| {
                    let name = asset["name"].as_str()?;
                    let url = asset["browser_download_url"].as_str()?;
                    (name.eq_ignore_ascii_case(&expected_name)
                        && url.starts_with(
                            "https://github.com/hoangphuctv/xemanh/releases/download/",
                        ))
                    .then(|| url.to_owned())
                })
            })
            .ok_or_else(|| "Không tìm thấy bộ cài Windows cho bản cập nhật mới".to_owned())?;

        Ok(WorkerResult::Available(Release {
            version,
            installer_url,
        }))
    }

    fn download_installer(release: Release) -> Result<WorkerResult, String> {
        let update_dir = std::env::temp_dir().join("XemAnh").join("updates");
        fs::create_dir_all(&update_dir)
            .map_err(|error| format!("Không thể tạo thư mục cập nhật: {error}"))?;
        let destination = update_dir.join(format!("xemanh-{}-setup.exe", release.version));
        let mut response = ureq::get(&release.installer_url)
            .set("User-Agent", "XemAnh-updater")
            .call()
            .map_err(|error| format!("Không thể tải bản cập nhật: {error}"))?
            .into_reader();
        let mut file = File::create(&destination)
            .map_err(|error| format!("Không thể lưu bản cập nhật: {error}"))?;
        copy(&mut response, &mut file)
            .map_err(|error| format!("Tải bản cập nhật không hoàn tất: {error}"))?;

        Ok(WorkerResult::Downloaded(destination))
    }

    fn is_newer_version(latest: &str, current: &str) -> bool {
        let parse = |version: &str| -> Option<Vec<u64>> {
            version
                .split(['.', '-'])
                .map(|part| part.parse::<u64>().ok())
                .collect()
        };
        let (Some(mut latest), Some(mut current)) = (parse(latest), parse(current)) else {
            return false;
        };
        let length = latest.len().max(current.len());
        latest.resize(length, 0);
        current.resize(length, 0);
        latest > current
    }

    fn ask_user_to_update(version: &str) -> bool {
        use std::ffi::c_void;

        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(
                hwnd: *mut c_void,
                text: *const u16,
                caption: *const u16,
                kind: u32,
            ) -> i32;
        }

        const MB_YESNO: u32 = 0x0000_0004;
        const MB_ICONINFORMATION: u32 = 0x0000_0040;
        const IDYES: i32 = 6;
        let text =
            format!("Đã có XemAnh {version}. Bạn có muốn tải và cài đặt bản cập nhật này không?");
        let to_wide =
            |value: &str| -> Vec<u16> { value.encode_utf16().chain(std::iter::once(0)).collect() };
        let text = to_wide(&text);
        let caption = to_wide("Cập nhật XemAnh");
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                MB_YESNO | MB_ICONINFORMATION,
            ) == IDYES
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    #[allow(dead_code)]
    pub enum UpdatePoll {
        None,
        Downloading,
        LaunchInstaller,
        Error(String),
    }

    pub struct Updater;

    impl Updater {
        pub fn new() -> Self {
            Self
        }

        pub fn poll(&mut self) -> UpdatePoll {
            UpdatePoll::None
        }
    }
}

pub use platform::{UpdatePoll, Updater};
