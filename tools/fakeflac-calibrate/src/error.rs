//! Errors that end a run, with the exit code of calibration.md section 2.

/// Which exit code an error maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// Exit 1: internal error (a bug).
    Internal,
    /// Exit 2: input or configuration error.
    Input,
    /// Exit 3: external tool error (ffmpeg not found).
    ExternalTool,
}

impl ExitKind {
    pub fn code(self) -> i32 {
        match self {
            ExitKind::Internal => 1,
            ExitKind::Input => 2,
            ExitKind::ExternalTool => 3,
        }
    }
}

/// An error that stops the run. `lines` are written to the error output, one per line
/// (for example every `trusted.txt:<line>: <message>`, in line order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    pub kind: ExitKind,
    pub lines: Vec<String>,
}
