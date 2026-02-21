use anyhow::{Result, bail};
use nom::Finish;
use nom_derive::{NomBE, NomLE, Parse};

/// Update file format: contains all the operations needed to update a system to
/// a specific version. It can be a full payload which can update from any
/// version, or a delta payload which can only update from a specific version.
#[allow(dead_code)]
#[derive(Debug, NomBE)]
pub struct Payload<'a> {
    /// Should be "CrAU".
    #[nom(Tag = r#"b"CrAU""#)]
    pub magic_bytes: &'a [u8],

    /// Payload major version.
    pub file_format_version: u64,

    /// Size of [`DeltaArchiveManifest`].
    pub manifest_size: u64,

    /// Only present if format_version >= 2.
    #[nom(If = "file_format_version > 1")]
    pub metadata_signature_size: Option<u32>,

    /// This is a serialized [`DeltaArchiveManifest`] message.
    #[nom(Take = "manifest_size")]
    pub manifest: &'a [u8],

    /// The signature of the metadata (from the beginning of the payload up to
    /// this location, not including the signature itself). This is a serialized
    /// [`Signatures`] message.
    #[nom(If = "metadata_signature_size.is_some()", Take = "metadata_signature_size.unwrap()")]
    pub metadata_signature: Option<&'a [u8]>,

    /// Data blobs for files, no specific format. The specific offset and length
    /// of each data blob is recorded in the [`DeltaArchiveManifest`].
    #[nom(Parse = "::nom::combinator::rest")]
    pub data: &'a [u8],
}

impl<'a> Payload<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        match <Self as Parse<&'a [u8]>>::parse(bytes).finish() {
            Ok((_, payload)) if payload.looks_sane() => Ok(payload),
            Ok(_) | Err(_) => match <PayloadLe as Parse<&'a [u8]>>::parse(bytes).finish() {
                Ok((_, payload)) => {
                    let payload = Payload::from(payload);
                    if payload.looks_sane() {
                        Ok(payload)
                    } else {
                        bail!("Unable to parse payload file")
                    }
                }
                Err(e) if e.code == nom::error::ErrorKind::Tag => bail!("Invalid payload file"),
                Err(_) => bail!("Unable to parse payload file"),
            },
        }
    }

    fn looks_sane(&self) -> bool {
        matches!(self.file_format_version, 1 | 2 | 3 | 4)
    }
}

#[derive(Debug, NomLE)]
struct PayloadLe<'a> {
    #[nom(Tag = r#"b"CrAU""#)]
    pub magic_bytes: &'a [u8],
    pub file_format_version: u64,
    pub manifest_size: u64,
    #[nom(If = "file_format_version > 1")]
    pub metadata_signature_size: Option<u32>,
    #[nom(Take = "manifest_size")]
    pub manifest: &'a [u8],
    #[nom(If = "metadata_signature_size.is_some()", Take = "metadata_signature_size.unwrap()")]
    pub metadata_signature: Option<&'a [u8]>,
    #[nom(Parse = "::nom::combinator::rest")]
    pub data: &'a [u8],
}

impl<'a> From<PayloadLe<'a>> for Payload<'a> {
    fn from(payload: PayloadLe<'a>) -> Self {
        Self {
            magic_bytes: payload.magic_bytes,
            file_format_version: payload.file_format_version,
            manifest_size: payload.manifest_size,
            metadata_signature_size: payload.metadata_signature_size,
            manifest: payload.manifest,
            metadata_signature: payload.metadata_signature,
            data: payload.data,
        }
    }
}
