//! ffmpeg as an external process (`std::process::Command`, argument list, no shell). Generation.md section 2.

use crate::config::Setting;
use crate::error::ToolError;
use crate::transcode::{Fragment, TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder};
use std::path::{Path, PathBuf};

/// Arguments of the encode call, one `String` per argument, `source` passed unchanged:
/// `-nostdin -hide_banner -loglevel error [-ss <start> -t <fragment_s>] -i <source>
///  -map 0:a:0 -map_metadata -1 <setting args> -y <out>`. Times have 3 decimals; with no fragment
/// `-ss` and `-t` are omitted. The program name is not part of the list.
pub fn encode_args(
    _source: &Path,
    _fragment: Option<Fragment>,
    _setting: &Setting,
    _out: &Path,
) -> Vec<String> {
    todo!("RP-11: generation.md section 2")
}

/// Arguments of the decode call:
/// `-nostdin -hide_banner -loglevel error -i <input> -map 0:a:0 -map_metadata -1 <decode_args> -y <out>`.
pub fn decode_args(_input: &Path, _decode_args: &[String], _out: &Path) -> Vec<String> {
    todo!("RP-11: generation.md section 2")
}

/// `FAKEFLAC_FFMPEG`, or `ffmpeg` on `PATH` when `None`.
#[derive(Debug, Clone)]
pub struct FfmpegTranscoder {
    pub program: Option<PathBuf>,
}

impl FfmpegTranscoder {
    pub fn new(program: Option<PathBuf>) -> FfmpegTranscoder {
        FfmpegTranscoder { program }
    }
}

impl Transcoder for FfmpegTranscoder {
    fn supports(&self, _encoder: &str) -> bool {
        todo!("RP-11: sample-corpus.md section 1 (the binary rejects `fixture`)")
    }

    fn version(&self) -> Result<String, ToolError> {
        todo!("RP-11: generation.md section 2")
    }

    fn transcode(
        &self,
        _request: &TranscodeRequest<'_>,
    ) -> Result<TranscodeOutput, TranscodeError> {
        todo!("RP-11: generation.md section 2")
    }
}
