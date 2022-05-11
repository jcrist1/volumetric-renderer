use anyhow::Result;
pub trait LogErrWasm {
    fn log_err(self);
}

impl LogErrWasm for Result<()> {
    fn log_err(self) {
        match self {
            Ok(()) => (),
            Err(err) => unsafe { web_sys::console::log_1(&format!("{err:?}").into()) },
        }
    }
}
