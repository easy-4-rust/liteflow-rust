mod multimodal_config;
mod multimodal_error;
mod prepared_messages;
mod processor;

pub use multimodal_config::MultimodalConfig;
pub use multimodal_error::MultimodalError;
pub use prepared_messages::PreparedMessages;
pub use processor::{
    contains_image_markers, count_image_markers, extract_ollama_image_payload, parse_image_markers,
    prepare_messages_for_provider,
};
