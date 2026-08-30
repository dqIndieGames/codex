//! Context-window overflow image ladder for local3.
//!
//! Triggered only from sampling retries. Does not touch `previous_response_id`
//! or sticky-break; those stay on their existing every-3-retries path.
//!
//! Tiers, applied on overflow retry 3 / 6 / 9 / 12:
//! 1. Keep last 5 images unchanged; downgrade earlier `original` → `high`.
//! 2. Keep last 1 image unchanged; downgrade earlier `original` → `high`.
//! 3. Keep last 5 images; replace earlier images with a 1x1 PNG.
//! 4. Keep last 1 image; replace earlier images with a 1x1 PNG.
//!
//! A no-op tier is skipped in the same trigger so a thread with no originals
//! does not burn another 3 retries before eviction.
//!
//! Each applied tier is persisted as a `RolloutItem::ImagesShrunk` carrying only
//! the tier number. Rollout replay re-runs [`apply_image_ladder_tier`] against
//! the history it has rebuilt so far, which keeps "keep the last N images"
//! correct even when a rollback has since shortened that history. Every tier is
//! idempotent: already-downgraded and already-placeholdered images are skipped.

use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::ResponseItem;

const TINY_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==";
const MAX_TIER: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageLadderReport {
    pub tier: u8,
    pub changed: usize,
    pub message: String,
}

#[derive(Clone, Copy)]
struct ImageSlot {
    item_idx: usize,
    content_idx: usize,
    in_tool_output: bool,
}

pub(crate) fn apply_next_image_ladder_tier(
    items: &mut [ResponseItem],
    applied_tier: &mut u8,
) -> Option<ImageLadderReport> {
    let start = (*applied_tier).saturating_add(1).max(1);
    for tier in start..=MAX_TIER {
        let changed = apply_image_ladder_tier(items, tier);
        if changed > 0 {
            *applied_tier = tier;
            return Some(ImageLadderReport {
                tier,
                changed,
                message: ladder_message(tier, changed),
            });
        }
        *applied_tier = tier;
    }
    None
}

fn ladder_message(tier: u8, changed: usize) -> String {
    match tier {
        1 => format!(
            "Context overflow image ladder step 1: kept last 5 originals, downgraded {changed}."
        ),
        2 => format!(
            "Context overflow image ladder step 2: kept last 1 original, downgraded {changed}."
        ),
        3 => format!(
            "Context overflow image ladder step 3: kept last 5 images, replaced {changed} with placeholders."
        ),
        4 => format!(
            "Context overflow image ladder step 4: kept last 1 image, replaced {changed} with placeholders."
        ),
        _ => format!("Context overflow image ladder step {tier}: changed {changed}."),
    }
}

/// Applies a single ladder tier and reports how many images it changed.
///
/// Also used by rollout replay to rebuild the shrunken history from a persisted
/// tier number.
pub(crate) fn apply_image_ladder_tier(items: &mut [ResponseItem], tier: u8) -> usize {
    match tier {
        1 => downgrade_except_last(items, 5),
        2 => downgrade_except_last(items, 1),
        3 => placeholder_except_last(items, 5),
        4 => placeholder_except_last(items, 1),
        _ => 0,
    }
}

fn tiny_png_data_url() -> String {
    format!("data:image/png;base64,{TINY_PNG_BASE64}")
}

fn is_placeholder_url(image_url: &str) -> bool {
    image_url == tiny_png_data_url()
}

fn collect_slots(items: &[ResponseItem]) -> Vec<ImageSlot> {
    let mut slots = Vec::new();
    for (item_idx, item) in items.iter().enumerate() {
        match item {
            ResponseItem::Message { content, .. } => {
                for (content_idx, content_item) in content.iter().enumerate() {
                    if matches!(content_item, ContentItem::InputImage { .. }) {
                        slots.push(ImageSlot {
                            item_idx,
                            content_idx,
                            in_tool_output: false,
                        });
                    }
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                let Some(content) = output.content_items() else {
                    continue;
                };
                for (content_idx, content_item) in content.iter().enumerate() {
                    if matches!(
                        content_item,
                        FunctionCallOutputContentItem::InputImage { .. }
                    ) {
                        slots.push(ImageSlot {
                            item_idx,
                            content_idx,
                            in_tool_output: true,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    slots
}

fn image_parts_mut<'a>(
    items: &'a mut [ResponseItem],
    slot: ImageSlot,
) -> Option<(&'a mut String, &'a mut Option<ImageDetail>)> {
    let item = items.get_mut(slot.item_idx)?;
    if slot.in_tool_output {
        let content = match item {
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => output.content_items_mut()?,
            _ => return None,
        };
        match content.get_mut(slot.content_idx)? {
            FunctionCallOutputContentItem::InputImage { image_url, detail } => {
                Some((image_url, detail))
            }
            _ => None,
        }
    } else {
        let content = match item {
            ResponseItem::Message { content, .. } => content,
            _ => return None,
        };
        match content.get_mut(slot.content_idx)? {
            ContentItem::InputImage { image_url, detail } => Some((image_url, detail)),
            _ => None,
        }
    }
}

fn is_original_detail(detail: Option<ImageDetail>) -> bool {
    matches!(detail, Some(ImageDetail::Original))
}

fn downgrade_except_last(items: &mut [ResponseItem], keep_last: usize) -> usize {
    let slots = collect_slots(items);
    if slots.len() <= keep_last {
        return 0;
    }
    let cut = slots.len() - keep_last;
    let mut changed = 0;
    for slot in slots.into_iter().take(cut) {
        let Some((image_url, detail)) = image_parts_mut(items, slot) else {
            continue;
        };
        if is_placeholder_url(image_url) {
            continue;
        }
        if !is_original_detail(*detail) {
            continue;
        }
        *detail = Some(ImageDetail::High);
        changed += 1;
    }
    changed
}

fn placeholder_except_last(items: &mut [ResponseItem], keep_last: usize) -> usize {
    let slots = collect_slots(items);
    if slots.len() <= keep_last {
        return 0;
    }
    let cut = slots.len() - keep_last;
    let placeholder = tiny_png_data_url();
    let mut changed = 0;
    for slot in slots.into_iter().take(cut) {
        let Some((image_url, detail)) = image_parts_mut(items, slot) else {
            continue;
        };
        if is_placeholder_url(image_url) {
            continue;
        }
        *image_url = placeholder.clone();
        *detail = Some(ImageDetail::High);
        changed += 1;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_protocol::ResponseItemId;
    use codex_protocol::models::FunctionCallOutputBody;
    use codex_protocol::models::FunctionCallOutputPayload;

    fn original_image(url: &str) -> ContentItem {
        ContentItem::InputImage {
            image_url: url.to_string(),
            detail: Some(ImageDetail::Original),
        }
    }

    fn high_image(url: &str) -> ContentItem {
        ContentItem::InputImage {
            image_url: url.to_string(),
            detail: Some(ImageDetail::High),
        }
    }

    fn user_images(urls: &[&str]) -> Vec<ResponseItem> {
        urls.iter()
            .enumerate()
            .map(|(idx, url)| ResponseItem::Message {
                id: Some(ResponseItemId::from_server(format!("msg-{idx}"))),
                role: "user".to_string(),
                content: vec![original_image(url)],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            })
            .collect()
    }

    fn details(items: &[ResponseItem]) -> Vec<Option<ImageDetail>> {
        collect_slots(items)
            .into_iter()
            .map(|slot| {
                let item = &items[slot.item_idx];
                match item {
                    ResponseItem::Message { content, .. } => match &content[slot.content_idx] {
                        ContentItem::InputImage { detail, .. } => *detail,
                        _ => None,
                    },
                    _ => None,
                }
            })
            .collect()
    }

    fn urls(items: &[ResponseItem]) -> Vec<String> {
        collect_slots(items)
            .into_iter()
            .map(|slot| {
                let item = &items[slot.item_idx];
                match item {
                    ResponseItem::Message { content, .. } => match &content[slot.content_idx] {
                        ContentItem::InputImage { image_url, .. } => image_url.clone(),
                        _ => String::new(),
                    },
                    ResponseItem::FunctionCallOutput { output, .. }
                    | ResponseItem::CustomToolCallOutput { output, .. } => output
                        .content_items()
                        .and_then(|content| match &content[slot.content_idx] {
                            FunctionCallOutputContentItem::InputImage { image_url, .. } => {
                                Some(image_url.clone())
                            }
                            _ => None,
                        })
                        .unwrap_or_default(),
                    _ => String::new(),
                }
            })
            .collect()
    }

    #[test]
    fn tier1_keeps_last_five_originals_and_downgrades_earlier() {
        let mut items = user_images(&["a", "b", "c", "d", "e", "f", "g"]);
        let changed = apply_image_ladder_tier(&mut items, 1);
        assert_eq!(changed, 2);
        assert_eq!(
            details(&items),
            vec![
                Some(ImageDetail::High),
                Some(ImageDetail::High),
                Some(ImageDetail::Original),
                Some(ImageDetail::Original),
                Some(ImageDetail::Original),
                Some(ImageDetail::Original),
                Some(ImageDetail::Original),
            ]
        );
    }

    #[test]
    fn tier2_keeps_last_original_only() {
        let mut items = user_images(&["a", "b", "c"]);
        let changed = apply_image_ladder_tier(&mut items, 2);
        assert_eq!(changed, 2);
        assert_eq!(
            details(&items),
            vec![
                Some(ImageDetail::High),
                Some(ImageDetail::High),
                Some(ImageDetail::Original),
            ]
        );
    }

    #[test]
    fn tier3_replaces_older_images_with_placeholder() {
        let mut items = user_images(&["a", "b", "c", "d", "e", "f"]);
        let changed = apply_image_ladder_tier(&mut items, 3);
        assert_eq!(changed, 1);
        let got = urls(&items);
        assert_eq!(got[0], tiny_png_data_url());
        assert_eq!(got[1], "b");
        assert_eq!(got[5], "f");
    }

    #[test]
    fn tier4_keeps_only_last_image() {
        let mut items = user_images(&["a", "b", "c"]);
        let changed = apply_image_ladder_tier(&mut items, 4);
        assert_eq!(changed, 2);
        let got = urls(&items);
        assert_eq!(got[0], tiny_png_data_url());
        assert_eq!(got[1], tiny_png_data_url());
        assert_eq!(got[2], "c");
    }

    #[test]
    fn no_op_when_too_few_images_then_cascade_reaches_eviction() {
        let mut items = vec![ResponseItem::Message {
            id: Some(ResponseItemId::from_server("msg-0".to_string())),
            role: "user".to_string(),
            content: vec![high_image("only-high")],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }];
        let mut applied = 0;
        let report = apply_next_image_ladder_tier(&mut items, &mut applied);
        assert!(report.is_none());
        assert_eq!(applied, 4);
        assert_eq!(urls(&items), vec!["only-high".to_string()]);
    }

    #[test]
    fn cascade_skips_quality_tiers_when_nothing_is_original() {
        let mut items = (0..6)
            .map(|idx| ResponseItem::Message {
                id: Some(ResponseItemId::from_server(format!("msg-{idx}"))),
                role: "user".to_string(),
                content: vec![high_image(&format!("img-{idx}"))],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            })
            .collect::<Vec<_>>();
        let mut applied = 0;
        let report = apply_next_image_ladder_tier(&mut items, &mut applied)
            .expect("eviction should run after quality no-ops");
        assert_eq!(report.tier, 3);
        assert_eq!(report.changed, 1);
        assert_eq!(applied, 3);
        assert_eq!(urls(&items)[0], tiny_png_data_url());
        assert_eq!(urls(&items)[5], "img-5");
    }

    #[test]
    fn tool_output_images_are_included() {
        let mut items = vec![ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-1".to_string()),
            name: None,
            namespace: None,
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::ContentItems(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "tool-a".to_string(),
                        detail: Some(ImageDetail::Original),
                    },
                ]),
                success: Some(true),
            },
            internal_chat_message_metadata_passthrough: None,
        }];
        let changed = apply_image_ladder_tier(&mut items, 2);
        assert_eq!(changed, 0);
        let changed = apply_image_ladder_tier(&mut items, 4);
        assert_eq!(changed, 0);
        items.push(ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-2".to_string()),
            name: None,
            namespace: None,
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::ContentItems(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image_url: "tool-b".to_string(),
                        detail: Some(ImageDetail::Original),
                    },
                ]),
                success: Some(true),
            },
            internal_chat_message_metadata_passthrough: None,
        });
        let changed = apply_image_ladder_tier(&mut items, 2);
        assert_eq!(changed, 1);
        match &items[0] {
            ResponseItem::FunctionCallOutput { output, .. } => {
                let content = output.content_items().expect("content items");
                match &content[0] {
                    FunctionCallOutputContentItem::InputImage { detail, .. } => {
                        assert_eq!(*detail, Some(ImageDetail::High));
                    }
                    other => panic!("expected image, got {other:?}"),
                }
            }
            other => panic!("expected tool output, got {other:?}"),
        }
    }
}
