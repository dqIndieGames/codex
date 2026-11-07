//! Six-stage, per-image recovery. The original content is immutable; only request copies change.

use std::collections::HashSet;
use std::io::{self, Cursor, Write};
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use codex_history::{ImageLadderImage, ImageLadderState, ImagesShrunkItem};
use codex_protocol::models::{ImageDetail, ImageReference, ResponseItem};
use image::{ImageDecoder, ImageEncoder};
use sha1::{Digest, Sha1};

use crate::context_overflow_image_ladder::{collect_slots, image_parts_mut, is_placeholder_image, tiny_png_data_url};

pub(crate) struct RecoveryReport {
    pub record: ImagesShrunkItem,
    pub message: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

/// Old rollouts did not always store item IDs. Derive identities before replaying any change.
pub(crate) fn assign_legacy_ids(items: &mut [ResponseItem]) -> io::Result<()> {
    let mut used: HashSet<String> = items.iter().filter_map(|item| item.id().map(|id| id.as_str().to_owned())).collect();
    for slot in collect_slots(items) {
        let item = &mut items[slot.item_idx];
        if item.id().is_some_and(|id| !id.is_empty()) { continue }
        let key = digest(&serde_json::to_vec(item).map_err(io::Error::other)?);
        let prefix = item.id_prefix().unwrap_or("msg");
        let mut occurrence = 0;
        loop {
            let id = codex_protocol::ResponseItemId::with_suffix(prefix, &format!("local3_{key}_{occurrence}"));
            if used.insert(id.as_str().to_owned()) { item.set_id(Some(id)); break }
            occurrence += 1;
        }
    }
    Ok(())
}

fn write_object(directory: &Path, key: &str, bytes: &[u8]) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    match std::fs::OpenOptions::new().write(true).create_new(true).open(directory.join(key)) {
        Ok(mut file) => file.write_all(bytes),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

fn content(entry: &ImageLadderImage, original: bool) -> io::Result<Arc<[u8]>> {
    let (key, memory) = if original {
        (&entry.source_digest, &entry.source_content)
    } else {
        (&entry.result_digest, &entry.result_content)
    };
    if let Some(bytes) = memory {
        return Ok(bytes.clone());
    }
    let directory = entry.cache_dir.as_ref().ok_or_else(|| io::Error::other("image content reference is unavailable"))?;
    let bytes = std::fs::read(directory.join(key))?;
    if digest(&bytes) != *key {
        return Err(io::Error::other("image content reference failed integrity check"));
    }
    Ok(bytes.into())
}

/// Register entire batches as protected. Sending or receiving another image never releases one.
pub(crate) fn discover(
    items: &mut [ResponseItem],
    state: &mut ImageLadderState,
    cache_dir: Option<&Path>,
) -> io::Result<()> {
    assign_legacy_ids(items)?;
    for slot in collect_slots(items) {
        let Some(item_id) = items[slot.item_idx].id().map(|id| id.as_str().to_owned()) else {
            continue;
        };
        let id = format!("{item_id}:{}", slot.content_idx);
        if state.images.contains_key(&id) {
            continue;
        }
        let Some((reference, _)) = image_parts_mut(items, slot) else { continue };
        if is_placeholder_image(reference) { continue }
        let ImageReference::Inline { image_url } = reference else { continue };
        let Some((prefix, encoded)) = image_url.split_once(";base64,") else { continue };
        let Some(mime) = prefix.strip_prefix("data:") else { continue };
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else { continue };
        let Ok(decoded) = image::load_from_memory(&bytes) else { continue };
        let key = digest(&bytes);
        if let Some(directory) = cache_dir { write_object(directory, &key, &bytes)?; }
        let memory = cache_dir.is_none().then(|| Arc::<[u8]>::from(bytes.as_slice()));
        state.images.insert(id.clone(), ImageLadderImage {
            id, item_id, content_index: slot.content_idx,
            source_digest: key.clone(), source_mime: mime.to_owned(),
            result_digest: key, result_mime: mime.to_owned(),
            cache_dir: cache_dir.map(Path::to_path_buf), attempted: 0, tier: 0,
            protected: true, minimum_long_edge: 1280,
            source_width: decoded.width(), source_height: decoded.height(),
            result_width: decoded.width(), result_height: decoded.height(), result_bytes: bytes.len(),
            source_content: memory.clone(), result_content: memory,
        });
    }
    Ok(())
}

/// Record the official preparation result while retaining the immutable, pre-preparation source.
pub(crate) fn capture_prepared(items: &mut [ResponseItem], state: &mut ImageLadderState) -> io::Result<()> {
    for slot in collect_slots(items) {
        let Some(id) = items[slot.item_idx].id() else { continue };
        let key = format!("{}:{}", id.as_str(), slot.content_idx);
        let Some(entry) = state.images.get_mut(&key) else { continue };
        if entry.tier != 0 { continue }
        let Some((ImageReference::Inline { image_url }, _)) = image_parts_mut(items, slot) else { continue };
        let Some((prefix, data)) = image_url.split_once(";base64,") else { continue };
        let Some(mime) = prefix.strip_prefix("data:") else { continue };
        let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(io::Error::other)?;
        let decoded = image::load_from_memory(&bytes).map_err(io::Error::other)?;
        adopt(entry, bytes, mime, decoded.width(), decoded.height(), 0)?;
    }
    Ok(())
}

fn encode(entry: &ImageLadderImage, tier: u8) -> io::Result<(Vec<u8>, &'static str, u32, u32)> {
    let cache_key = format!("{}-v2-{tier}.image", entry.source_digest);
    if let Some(directory) = &entry.cache_dir
        && directory.join(&cache_key).is_file() {
        let bytes = std::fs::read(directory.join(&cache_key))?;
        let decoded = image::load_from_memory(&bytes).map_err(io::Error::other)?;
        let mime = if image::guess_format(&bytes).map_err(io::Error::other)? == image::ImageFormat::Jpeg { "image/jpeg" } else { "image/png" };
        return Ok((bytes, mime, decoded.width(), decoded.height()));
    }
    let bytes = content(entry, true)?;
    let mut decoder = image::ImageReader::new(Cursor::new(&*bytes))
        .with_guessed_format()?.into_decoder().map_err(io::Error::other)?;
    let icc = decoder.icc_profile().map_err(io::Error::other)?;
    let exif = decoder.exif_metadata().map_err(io::Error::other)?;
    let mut decoded = image::DynamicImage::from_decoder(decoder).map_err(io::Error::other)?;
    let limit = match tier { 3 => 1280, 4 => 768, _ => u32::MAX };
    if decoded.width().max(decoded.height()) > limit {
        decoded = decoded.resize(limit, limit, image::imageops::FilterType::Lanczos3);
    }
    let (width, height) = (decoded.width(), decoded.height());
    let mut output = Vec::new();
    // Alpha is preserved. Never flatten a transparent image onto a guessed background.
    let transparent = decoded.color().has_alpha() && decoded.to_rgba16().pixels().any(|pixel| pixel[3] != u16::MAX);
    let mime = if tier == 1 || transparent {
        let mut encoder = image::codecs::png::PngEncoder::new_with_quality(
            &mut output, image::codecs::png::CompressionType::Best, image::codecs::png::FilterType::Adaptive,
        );
        if let Some(profile) = icc { encoder.set_icc_profile(profile).map_err(io::Error::other)?; }
        if let Some(metadata) = exif { encoder.set_exif_metadata(metadata).map_err(io::Error::other)?; }
        encoder.write_image(decoded.as_bytes(), width, height, decoded.color().into()).map_err(io::Error::other)?;
        "image/png"
    } else {
        let quality = match tier { 2 => 95, 3 => 85, _ => 75 };
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
        if let Some(profile) = icc { encoder.set_icc_profile(profile).map_err(io::Error::other)?; }
        if let Some(metadata) = exif { encoder.set_exif_metadata(metadata).map_err(io::Error::other)?; }
        encoder.encode(&decoded.to_rgb8(), width, height, image::ExtendedColorType::Rgb8).map_err(io::Error::other)?;
        "image/jpeg"
    };
    if let Some(directory) = &entry.cache_dir { write_object(directory, &cache_key, &output)?; }
    Ok((output, mime, width, height))
}

fn adopt(entry: &mut ImageLadderImage, bytes: Vec<u8>, mime: &str, width: u32, height: u32, tier: u8) -> io::Result<()> {
    let key = digest(&bytes);
    if let Some(directory) = &entry.cache_dir { write_object(directory, &key, &bytes)?; }
    entry.result_digest = key;
    entry.result_mime = mime.to_owned();
    entry.result_width = width;
    entry.result_height = height;
    entry.result_bytes = bytes.len();
    entry.result_content = entry.cache_dir.is_none().then(|| Arc::from(bytes));
    entry.tier = tier;
    Ok(())
}

/// A round takes one effective step per eligible image, skipping remembered no-op stages.
pub(crate) fn advance(
    items: &mut [ResponseItem], state: &mut ImageLadderState, cache_dir: Option<&Path>, dimensions_only: bool,
) -> io::Result<Option<RecoveryReport>> {
    discover(items, state, cache_dir)?;
    let mut ordered = Vec::new();
    for slot in collect_slots(items) {
        if !matches!(image_parts_mut(items, slot), Some((ImageReference::Inline { .. }, _))) { continue }
        if let Some(id) = items[slot.item_idx].id() {
            let key = format!("{}:{}", id.as_str(), slot.content_idx);
            if state.images.contains_key(&key) { ordered.push(key); }
        }
    }
    let mut changed = Vec::new();
    // Old images exhaust their available recovery first. Protected batches are considered only then.
    for protected in [false, true] {
        for key in &ordered {
            let entry = state.images.get_mut(key).expect("discovered image");
            if entry.protected != protected || entry.tier >= 5 { continue }
            for tier in 1..=4u8 {
                let bit = 1 << (tier - 1);
                if entry.attempted & bit != 0 { continue }
                if dimensions_only && tier <= 2 { entry.attempted |= bit; continue }
                let limit = match tier { 3 => 1280, 4 => 768, _ => u32::MAX };
                if entry.protected && limit < entry.minimum_long_edge.min(entry.source_width.max(entry.source_height)) {
                    continue;
                }
                // No-op sentinels survive restarts without appending fake image-change records.
                let no_op_key = format!("{}-{}-v2-{tier}.noop", entry.source_digest, entry.result_digest);
                if entry.cache_dir.as_ref().is_some_and(|dir| dir.join(&no_op_key).is_file()) {
                    entry.attempted |= bit;
                    continue;
                }
                let (bytes, mime, width, height) = encode(entry, tier)?;
                entry.attempted |= bit;
                if bytes.len() >= entry.result_bytes {
                    if let Some(directory) = &entry.cache_dir { write_object(directory, &no_op_key, &[])?; }
                    continue;
                }
                adopt(entry, bytes, mime, width, height, tier)?;
                changed.push(entry.clone());
                break;
            }
        }
        if !changed.is_empty() { break }
        if !protected {
            let recent: HashSet<_> = ordered.iter().rev().filter(|key| state.images[*key].tier < 5).take(5).cloned().collect();
            let placeholder_url = tiny_png_data_url();
            let encoded = placeholder_url.split_once(',').expect("fixed PNG data URL").1;
            let placeholder = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(io::Error::other)?;
            for retention in state.retention.max(5)..=6 {
                for key in &ordered {
                    let entry = state.images.get_mut(key).expect("discovered image");
                    if entry.protected || entry.tier >= 5 || entry.attempted != 15
                        || (retention == 5 && recent.contains(key)) || entry.result_bytes <= placeholder.len() { continue }
                    adopt(entry, placeholder.clone(), "image/png", 1, 1, retention)?;
                    changed.push(entry.clone());
                    // With no authoritative request budget, stage six evicts one image per round.
                    if retention == 6 { break }
                }
                if !changed.is_empty() { state.retention = retention; break }
            }
            if !changed.is_empty() { break }
        }
    }
    if changed.is_empty() { return Ok(None) }
    let tier = changed.iter().map(|entry| entry.tier).max().unwrap_or(0);
    let record = ImagesShrunkItem {
        version: Some(2), tier, changed: u32::try_from(changed.len()).ok(), images: changed,
        retention: Some(state.retention),
    };
    apply_record(items, &record)?;
    let (image_bytes, real, placeholders) = request_image_stats(items);
    Ok(Some(RecoveryReport {
        message: format!("图片处理第 {tier} 档：图片 {image_bytes} 字节，真图 {real} 张，占位 {placeholders} 张；保留受保护图片。"),
        record,
    }))
}

pub(crate) fn apply_record(items: &mut [ResponseItem], record: &ImagesShrunkItem) -> io::Result<()> {
    assign_legacy_ids(items)?;
    if record.version != Some(2) {
        crate::context_overflow_image_ladder::apply_image_ladder_tier(items, record.tier);
        return Ok(());
    }
    let mut replacements = Vec::new();
    for entry in &record.images {
        if entry.tier == 0 { continue }
        let Some(item_idx) = items.iter().position(|item| item.id().is_some_and(|id| id.as_str() == entry.item_id)) else { continue };
        let Some(slot) = collect_slots(items).into_iter().find(|slot| slot.item_idx == item_idx && slot.content_idx == entry.content_index) else { continue };
        let bytes = match content(entry, false) {
            Ok(bytes) => bytes,
            Err(_) if entry.tier >= 5 => {
                let url = tiny_png_data_url();
                Arc::from(base64::engine::general_purpose::STANDARD.decode(url.split_once(',').expect("PNG URL").1).map_err(io::Error::other)?)
            }
            Err(_) => {
                let (bytes, _, _, _) = encode(entry, entry.tier)?;
                if digest(&bytes) != entry.result_digest { return Err(io::Error::other("image recovery cache does not match recorded result")); }
                if let Some(directory) = &entry.cache_dir { write_object(directory, &entry.result_digest, &bytes)?; }
                Arc::from(bytes)
            }
        };
        // Full decoding rejects corrupt placeholders and corrupt cached results before installation.
        image::load_from_memory(&bytes).map_err(io::Error::other)?;
        replacements.push((slot, codex_utils_image::data_url_from_bytes(&entry.result_mime, &bytes)));
    }
    for (slot, image_url) in replacements {
        if let Some((image, detail)) = image_parts_mut(items, slot) {
            *image = ImageReference::Inline { image_url };
            *detail = Some(ImageDetail::Original);
        }
    }
    Ok(())
}

pub(crate) fn request_image_stats(items: &mut [ResponseItem]) -> (usize, usize, usize) {
    let (mut bytes, mut real, mut placeholders) = (0, 0, 0);
    for slot in collect_slots(items) {
        let Some((image, _)) = image_parts_mut(items, slot) else { continue };
        if is_placeholder_image(image) { placeholders += 1; } else { real += 1; }
        if let ImageReference::Inline { image_url } = image { bytes += image_url.len(); }
    }
    (bytes, real, placeholders)
}

/// Install already decoded results by identity without replacing concurrent additions to history.
pub(crate) fn copy_processed_images(current: &mut [ResponseItem], processed: &[ResponseItem], record: &ImagesShrunkItem) {
    for entry in &record.images {
        let Some(source) = processed.iter().find(|item| item.id().is_some_and(|id| id.as_str() == entry.item_id)) else { continue };
        let Some(target) = current.iter_mut().find(|item| item.id().is_some_and(|id| id.as_str() == entry.item_id)) else { continue };
        // Preserve non-image fields and the original envelope attribution.
        match (target, source) {
            (ResponseItem::Message { content: target, .. }, ResponseItem::Message { content: source, .. }) => {
                if let (Some(target), Some(source)) = (target.get_mut(entry.content_index), source.get(entry.content_index)) { *target = source.clone(); }
            }
            (ResponseItem::FunctionCallOutput { output: target, .. }, ResponseItem::FunctionCallOutput { output: source, .. })
            | (ResponseItem::CustomToolCallOutput { output: target, .. }, ResponseItem::CustomToolCallOutput { output: source, .. }) => {
                if let (Some(target), Some(source)) = (target.content_items_mut(), source.content_items()) {
                    if let (Some(target), Some(source)) = (target.get_mut(entry.content_index), source.get(entry.content_index)) { *target = source.clone(); }
                }
            }
            _ => {}
        }
    }
}
