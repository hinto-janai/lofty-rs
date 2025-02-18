mod attached_picture_frame;
mod audio_text_frame;
mod encapsulated_object;
mod extended_text_frame;
mod extended_url_frame;
mod identifier;
pub(in crate::id3::v2) mod language_frame;
mod popularimeter;
mod sync_text;
mod text_information_frame;
mod url_link_frame;

pub use attached_picture_frame::AttachedPictureFrame;
pub use audio_text_frame::{scramble, AudioTextFrame, AudioTextFrameFlags};
pub use encapsulated_object::GeneralEncapsulatedObject;
pub use extended_text_frame::ExtendedTextFrame;
pub use extended_url_frame::ExtendedUrlFrame;
pub use identifier::UniqueFileIdentifierFrame;
pub use language_frame::{CommentFrame, UnsynchronizedTextFrame};
pub use popularimeter::Popularimeter;
pub use sync_text::{SyncTextContentType, SynchronizedText, TimestampFormat};
pub use text_information_frame::TextInformationFrame;
pub use url_link_frame::UrlLinkFrame;
