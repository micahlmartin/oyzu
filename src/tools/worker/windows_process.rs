//! Native same-image creation with explicit environment and handle inheritance.
//! The caller owns release admission and worker bootstrap arguments; this owner
//! pins the expected image, assigns the suspended process to its job, then resumes.
use super::WindowsToolWorkerJob;
use anyhow::{ensure, Context, Result};
mod image;
use image::ImagePin;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs::OpenOptions,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::{null, null_mut},
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::{SetHandleInformation, HANDLE, HANDLE_FLAG_INHERIT, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::*,
};

const WIDE_LIMIT: usize = 32_767;

/// Owns a same-binary child and its job. This is an experimental native primitive,
/// not a public command or backend authorization. Arguments, environment and
/// capability handles must already be admitted by the trusted supervisor.
/// Explicit terminate() confirms job-wide exit; drop only requests termination
/// through kill-on-close. Control-I/O joining remains a separate obligation.
pub struct WindowsToolWorkerProcess {
    job: WindowsToolWorkerJob,
    process: OwnedHandle,
    _image: ImagePin,
}

impl WindowsToolWorkerProcess {
    /// Pin the current executable against an independently trusted release hash,
    /// spawn with ONLY the supplied handles plus null standard streams, assign
    /// suspended to a private job, and resume. No PATH or project-selected image.
    /// Consumes supplied handles; their parent copies close before child resume.
    /// Their raw values remain unchanged and may be used in bootstrap arguments.
    /// The empty environment is truly explicit; no parent variables are inherited.
    pub fn spawn_current(
        expected_image_digest: &str,
        arguments: &[OsString],
        environment: &BTreeMap<String, OsString>,
        directory: &Path,
        mut inherited: Vec<OwnedHandle>,
    ) -> Result<Self> {
        ensure!(inherited.len() <= 16, "TOOL_WORKER_HANDLE_LIMIT");
        ensure!(directory.is_absolute(), "TOOL_WORKER_DIRECTORY_INVALID");
        let directory = wide(directory.as_os_str())?;
        let mut command = command_line(arguments)?;
        let environment = environment_block(environment)?;
        let image = ImagePin::current(expected_image_digest)?;
        let image_path = wide(image.path.as_os_str())?;
        let mut job = WindowsToolWorkerJob::new()?;

        // Standard streams are explicit null devices, never the control channel
        // or the parent's console/diagnostic handles.
        let standard = OpenOptions::new().read(true).write(true).open("NUL")?;
        let standard: OwnedHandle = standard.into();
        let standard_handle = standard.as_raw_handle();
        inherited.push(standard);
        let handles: Vec<HANDLE> = inherited.iter().map(AsRawHandle::as_raw_handle).collect();
        for handle in &handles {
            // These handles are exclusively owned by this launch and consumed
            // on every path. Unrelated parent handles are never modified.
            check(
                unsafe { SetHandleInformation(*handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) },
                "TOOL_WORKER_HANDLE_INHERIT_FAILED",
            )?;
        }
        let mut attributes = Attributes::new(&handles)?;
        let startup = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: size_of::<STARTUPINFOEXW>() as u32,
                dwFlags: STARTF_USESTDHANDLES,
                hStdInput: standard_handle,
                hStdOutput: standard_handle,
                hStdError: standard_handle,
                ..Default::default()
            },
            lpAttributeList: attributes.pointer(),
        };
        let mut output = PROCESS_INFORMATION::default();
        // Every pointer refers to a live owned buffer/handle. The image path is
        // absolute and its file plus ancestors stay pinned across creation.
        check(
            unsafe {
                CreateProcessW(
                    image_path.as_ptr(),
                    command.as_mut_ptr(),
                    null(),
                    null(),
                    1,
                    CREATE_SUSPENDED
                        | CREATE_NO_WINDOW
                        | CREATE_UNICODE_ENVIRONMENT
                        | EXTENDED_STARTUPINFO_PRESENT,
                    environment.as_ptr().cast(),
                    directory.as_ptr(),
                    &startup.StartupInfo,
                    &mut output,
                )
            },
            "TOOL_WORKER_SPAWN_FAILED",
        )?;
        let pending = PendingProcess {
            process: Some(unsafe { OwnedHandle::from_raw_handle(output.hProcess) }),
            thread: unsafe { OwnedHandle::from_raw_handle(output.hThread) },
        };
        drop(attributes);
        drop(inherited);
        job.assign_suspended(pending.process.as_ref().unwrap().as_handle())?;
        let previous = unsafe { ResumeThread(pending.thread.as_raw_handle()) };
        ensure!(previous == 1, "TOOL_WORKER_RESUME_FAILED");
        Ok(Self {
            job,
            process: pending.commit(),
            _image: image,
        })
    }

    /// Preserve the full native DWORD exit code. A terminated initial process
    /// does not establish that descendants or I/O threads have finished.
    pub fn try_wait(&self) -> Result<Option<u32>> {
        match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
            WAIT_TIMEOUT => Ok(None),
            WAIT_OBJECT_0 => {
                let mut code = 0;
                check(
                    unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) },
                    "TOOL_WORKER_EXIT_QUERY_FAILED",
                )?;
                Ok(Some(code))
            }
            _ => Err(std::io::Error::last_os_error()).context("TOOL_WORKER_WAIT_FAILED"),
        }
    }

    pub fn terminate(&mut self, deadline: Instant) -> Result<u32> {
        self.job.terminate(deadline)?;
        loop {
            if let Some(code) = self.try_wait()? {
                return Ok(code);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            ensure!(!remaining.is_zero(), "TOOL_WORKER_EXIT_UNCONFIRMED");
            std::thread::sleep(remaining.min(std::time::Duration::from_millis(5)));
        }
    }
}

struct PendingProcess {
    process: Option<OwnedHandle>,
    thread: OwnedHandle,
}
impl PendingProcess {
    fn commit(mut self) -> OwnedHandle {
        self.process.take().unwrap()
    }
}
impl Drop for PendingProcess {
    fn drop(&mut self) {
        if let Some(process) = &self.process {
            // A failed assignment/resume must never strand a suspended process.
            // Keep its handle until termination is observed, not just requested.
            unsafe {
                TerminateProcess(process.as_raw_handle(), 130);
                WaitForSingleObject(process.as_raw_handle(), INFINITE);
            }
        }
    }
}

struct Attributes {
    storage: Vec<usize>,
}
impl Attributes {
    fn new(handles: &[HANDLE]) -> Result<Self> {
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes);
        }
        ensure!(
            bytes > 0 && bytes <= 64 * 1024,
            "TOOL_WORKER_ATTRIBUTES_INVALID"
        );
        let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        check(
            unsafe {
                InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), 1, 0, &mut bytes)
            },
            "TOOL_WORKER_ATTRIBUTES_FAILED",
        )?;
        let mut attributes = Self { storage };
        check(
            unsafe {
                UpdateProcThreadAttribute(
                    attributes.pointer(),
                    0,
                    PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                    handles.as_ptr().cast(),
                    std::mem::size_of_val(handles),
                    null_mut(),
                    null(),
                )
            },
            "TOOL_WORKER_HANDLE_LIST_FAILED",
        )?;
        Ok(attributes)
    }
    fn pointer(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        unsafe {
            DeleteProcThreadAttributeList(self.pointer());
        }
    }
}

fn check(success: i32, code: &'static str) -> Result<()> {
    if success == 0 {
        Err(std::io::Error::last_os_error()).context(code)
    } else {
        Ok(())
    }
}

fn append(output: &mut Vec<u16>, unit: u16) -> Result<()> {
    ensure!(
        unit != 0 && output.len() < WIDE_LIMIT - 1,
        "TOOL_WORKER_STARTUP_LIMIT"
    );
    output.push(unit);
    Ok(())
}
fn wide(value: &OsStr) -> Result<Vec<u16>> {
    let mut output = Vec::new();
    for unit in value.encode_wide() {
        append(&mut output, unit)?;
    }
    output.push(0);
    Ok(output)
}

fn command_line(arguments: &[OsString]) -> Result<Vec<u16>> {
    ensure!(arguments.len() <= 256, "TOOL_WORKER_STARTUP_LIMIT");
    let mut output: Vec<_> = "oyzu".encode_utf16().collect();
    for argument in arguments {
        append(&mut output, b' ' as u16)?;
        append(&mut output, b'"' as u16)?;
        let mut slashes = 0;
        for unit in argument.encode_wide() {
            if unit == b'\\' as u16 {
                slashes += 1;
                ensure!(slashes < WIDE_LIMIT, "TOOL_WORKER_STARTUP_LIMIT");
                continue;
            }
            for _ in 0..slashes * if unit == b'"' as u16 { 2 } else { 1 } {
                append(&mut output, b'\\' as u16)?;
            }
            slashes = 0;
            if unit == b'"' as u16 {
                append(&mut output, b'\\' as u16)?;
            }
            append(&mut output, unit)?;
        }
        for _ in 0..slashes * 2 {
            append(&mut output, b'\\' as u16)?;
        }
        append(&mut output, b'"' as u16)?;
    }
    output.push(0);
    Ok(output)
}

fn environment_block(environment: &BTreeMap<String, OsString>) -> Result<Vec<u16>> {
    ensure!(environment.len() <= 256, "TOOL_WORKER_ENVIRONMENT_INVALID");
    let mut sorted = BTreeMap::new();
    for (key, value) in environment {
        ensure!(
            !key.is_empty()
                && key.len() <= 256
                && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "TOOL_WORKER_ENVIRONMENT_INVALID"
        );
        ensure!(
            sorted
                .insert(key.to_ascii_uppercase(), (key, value))
                .is_none(),
            "TOOL_WORKER_ENVIRONMENT_INVALID"
        );
    }
    let mut output = Vec::new();
    for (_, (key, value)) in sorted {
        for unit in key
            .encode_utf16()
            .chain(Some(b'=' as u16))
            .chain(value.encode_wide())
        {
            append(&mut output, unit)?;
        }
        output.push(0);
    }
    if output.is_empty() {
        output.push(0);
    }
    output.push(0);
    ensure!(output.len() <= WIDE_LIMIT, "TOOL_WORKER_STARTUP_LIMIT");
    Ok(output)
}

#[cfg(test)]
#[path = "windows_process_tests.rs"]
mod tests;
