use codex_protocol::models::ResponseItem;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Compression {
    #[default]
    None,
    Zstd,
}

pub(crate) fn attach_item_ids(payload_json: &mut Value, original_items: &[ResponseItem]) {
    let Some(input_value) = payload_json.get_mut("input") else {
        return;
    };
    let Value::Array(items) = input_value else {
        return;
    };

    for (value, item) in items.iter_mut().zip(original_items.iter()) {
        let Some(id) = item.id() else {
            continue;
        };

        if let Some(obj) = value.as_object_mut() {
            obj.insert("id".to_string(), Value::String(id.to_string()));
        }
    }
}

/// Safe counts for the final serialized request; byte count excludes transport compression.
pub(crate) fn record_image_request_size(items: &[ResponseItem], json_bytes: usize, transport: &str) {
    use codex_protocol::models::{ContentItem, FunctionCallOutputContentItem, ImageReference};
    let (mut image_url_bytes, mut real_images, mut placeholder_images, mut file_references) = (0usize, 0usize, 0usize, 0usize);
    let mut count = |image: &ImageReference| match image {
        ImageReference::Inline { image_url } => {
            image_url_bytes += image_url.len();
            if image_url == "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGNgYGBgAAAABQABpfZFQAAAAABJRU5ErkJggg==" { placeholder_images += 1; }
            else { real_images += 1; }
        }
        ImageReference::File { .. } => { real_images += 1; file_references += 1; }
    };
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                for part in content { if let ContentItem::InputImage { image, .. } = part { count(image); } }
            }
            ResponseItem::FunctionCallOutput { output, .. } | ResponseItem::CustomToolCallOutput { output, .. } => {
                if let Some(parts) = output.content_items() {
                    for part in parts { if let FunctionCallOutputContentItem::InputImage { image, .. } = part { count(image); } }
                }
            }
            _ => {}
        }
    }
    tracing::debug!(transport, json_bytes, image_url_bytes, real_images, placeholder_images, file_references, "serialized request image size");
}
