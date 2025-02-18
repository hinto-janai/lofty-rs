use super::header::{parse_header, parse_v2_header};
use super::Frame;
use crate::error::{Id3v2Error, Id3v2ErrorKind, Result};
use crate::id3::v2::frame::content::parse_content;
use crate::id3::v2::util::synchsafe::{SynchsafeInteger, UnsynchronizedStream};
use crate::id3::v2::{FrameFlags, FrameId, FrameValue, Id3v2Version};
use crate::macros::try_vec;

use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};

impl<'a> Frame<'a> {
	pub(crate) fn read<R>(reader: &mut R, version: Id3v2Version) -> Result<(Option<Self>, bool)>
	where
		R: Read,
	{
		// The header will be upgraded to ID3v2.4 past this point, so they can all be treated the same
		let (id, mut size, mut flags) = match match version {
			Id3v2Version::V2 => parse_v2_header(reader)?,
			Id3v2Version::V3 => parse_header(reader, false)?,
			Id3v2Version::V4 => parse_header(reader, true)?,
		} {
			None => return Ok((None, true)),
			Some(frame_header) => frame_header,
		};

		// Get the encryption method symbol
		if let Some(enc) = flags.encryption.as_mut() {
			if size < 1 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			*enc = reader.read_u8()?;
			size -= 1;
		}

		// Get the group identifier
		if let Some(group) = flags.grouping_identity.as_mut() {
			if size < 1 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			*group = reader.read_u8()?;
			size -= 1;
		}

		// Get the real data length
		if flags.data_length_indicator.is_some() || flags.compression {
			if size < 4 {
				return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
			}

			// For some reason, no one can follow the spec, so while a data length indicator is *written*
			// the flag **isn't always set**
			let len = reader.read_u32::<BigEndian>()?.unsynch();
			flags.data_length_indicator = Some(len);
			size -= 4;
		}

		// Frames must have at least 1 byte, *after* all of the additional data flags can provide
		if size == 0 {
			return Err(Id3v2Error::new(Id3v2ErrorKind::BadFrameLength).into());
		}

		// Restrict the reader to the frame content
		let mut reader = reader.take(u64::from(size));

		// It seems like the flags are applied in the order:
		//
		// unsynchronization -> compression -> encryption
		//
		// Which all have their own needs, so this gets a little messy...
		match flags {
			// Possible combinations:
			//
			// * unsynchronized + compressed + encrypted
			// * unsynchronized + compressed
			// * unsynchronized + encrypted
			// * unsynchronized
			FrameFlags {
				unsynchronisation: true,
				..
			} => {
				let mut unsynchronized_reader = UnsynchronizedStream::new(reader);

				if flags.compression {
					let mut compression_reader = handle_compression(unsynchronized_reader)?;

					if flags.encryption.is_some() {
						return handle_encryption(&mut compression_reader, size, id, flags);
					}

					return parse_frame(&mut compression_reader, id, flags, version);
				}

				if flags.encryption.is_some() {
					return handle_encryption(&mut unsynchronized_reader, size, id, flags);
				}

				return parse_frame(&mut unsynchronized_reader, id, flags, version);
			},
			// Possible combinations:
			//
			// * compressed + encrypted
			// * compressed
			FrameFlags {
				compression: true, ..
			} => {
				let mut compression_reader = handle_compression(reader)?;

				if flags.encryption.is_some() {
					return handle_encryption(&mut compression_reader, size, id, flags);
				}

				return parse_frame(&mut compression_reader, id, flags, version);
			},
			// Possible combinations:
			//
			// * encrypted
			FrameFlags {
				encryption: Some(_),
				..
			} => {
				return handle_encryption(&mut reader, size, id, flags);
			},
			// Everything else that doesn't have special flags
			_ => {
				return parse_frame(&mut reader, id, flags, version);
			},
		}
	}
}

#[cfg(feature = "id3v2_compression_support")]
#[allow(clippy::unnecessary_wraps)]
fn handle_compression<R: Read>(reader: R) -> Result<flate2::read::ZlibDecoder<R>> {
	Ok(flate2::read::ZlibDecoder::new(reader))
}

#[cfg(not(feature = "id3v2_compression_support"))]
#[allow(clippy::unnecessary_wraps)]
fn handle_compression<R>(_: R) -> Result<std::io::Empty> {
	Err(Id3v2Error::new(Id3v2ErrorKind::CompressedFrameEncountered).into())
}

fn handle_encryption<R: Read>(
	reader: &mut R,
	size: u32,
	id: FrameId<'static>,
	flags: FrameFlags,
) -> Result<(Option<Frame<'static>>, bool)> {
	if flags.data_length_indicator.is_none() {
		return Err(Id3v2Error::new(Id3v2ErrorKind::MissingDataLengthIndicator).into());
	}

	let mut content = try_vec![0; size as usize];
	reader.read_exact(&mut content)?;

	let encrypted_frame = Frame {
		id,
		value: FrameValue::Binary(content),
		flags,
	};

	// Nothing further we can do with encrypted frames
	Ok((Some(encrypted_frame), false))
}

fn parse_frame<R: Read>(
	reader: &mut R,
	id: FrameId<'static>,
	flags: FrameFlags,
	version: Id3v2Version,
) -> Result<(Option<Frame<'static>>, bool)> {
	match parse_content(reader, id.as_str(), version)? {
		Some(value) => Ok((Some(Frame { id, value, flags }), false)),
		None => Ok((None, false)),
	}
}
