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

impl ToolError {
    /// Exit 2 with one line.
    pub fn input(message: impl Into<String>) -> ToolError {
        ToolError {
            kind: ExitKind::Input,
            lines: vec![message.into()],
        }
    }

    /// Exit 2 with several lines.
    pub fn input_lines(lines: Vec<String>) -> ToolError {
        ToolError {
            kind: ExitKind::Input,
            lines,
        }
    }

    /// Exit 1 with one line.
    pub fn internal(message: impl Into<String>) -> ToolError {
        ToolError {
            kind: ExitKind::Internal,
            lines: vec![message.into()],
        }
    }

    /// Exit 3 with one line.
    pub fn external(message: impl Into<String>) -> ToolError {
        ToolError {
            kind: ExitKind::ExternalTool,
            lines: vec![message.into()],
        }
    }
}

impl From<std::io::Error> for ToolError {
    fn from(e: std::io::Error) -> ToolError {
        ToolError::internal(format!("error: {e}"))
    }
}
