use super::RealtimeInputTask;
use super::RealtimeInputTaskExit;
use super::RealtimeSidebandSessionInitialization;
use super::RealtimeWebrtcSidebandInputTask;
use super::flush_realtime_transcript_tail;
use super::report_realtime_transport_loss;
use super::run_realtime_input_task;
use codex_api::ApiError;
use codex_api::RealtimeEvent;
use codex_api::RealtimeEventParser;
use codex_api::RealtimeTranscriptState;
use codex_api::map_api_error;
use codex_login::default_client::default_headers;
use http::StatusCode;
use std::sync::atomic::Ordering;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::info;
use tracing::warn;

pub(super) fn spawn_webrtc_sideband_input_task(
    input: RealtimeWebrtcSidebandInputTask,
) -> JoinHandle<()> {
    let RealtimeWebrtcSidebandInputTask {
        client,
        session_config,
        call_id,
        sideband_headers,
        mut session_initialization,
        input_channels,
        events_tx,
        handoff_state,
        session_kind,
        event_parser,
        realtime_active,
        transcript_tail_flush,
        stop_token,
    } = input;

    tokio::spawn(async move {
        let mut reconnecting = false;
        let mut pending_outbound = None;
        let transcript_state = match &session_initialization {
            RealtimeSidebandSessionInitialization::ExistingCall {
                initial_connection: Some(connection),
            } => connection.events().transcript_state(),
            RealtimeSidebandSessionInitialization::CoreCreated
            | RealtimeSidebandSessionInitialization::ExistingCall {
                initial_connection: None,
            } => RealtimeTranscriptState::default(),
        };
        while realtime_active.load(Ordering::Relaxed) {
            let connection = match tokio::select! {
                biased;
                _ = stop_token.cancelled() => break,
                connection = async {
                    match &mut session_initialization {
                        RealtimeSidebandSessionInitialization::CoreCreated => {
                            client.connect_webrtc_sideband(
                                session_config.clone(),
                                &call_id,
                                sideband_headers.clone(),
                                default_headers(),
                                transcript_state.clone(),
                            ).await
                        }
                        RealtimeSidebandSessionInitialization::ExistingCall { initial_connection } => {
                            match initial_connection.take() {
                                Some(connection) => Ok(connection),
                                None => client.connect_existing_call_sideband(
                                    session_config.clone(),
                                    &call_id,
                                    sideband_headers.clone(),
                                    default_headers(),
                                    transcript_state.clone(),
                                ).await,
                            }
                        }
                    }
                } => connection,
            } {
                Ok(connection) => connection,
                Err(err) => {
                    if !realtime_active.load(Ordering::Relaxed) || stop_token.is_cancelled() {
                        break;
                    }
                    if reconnecting && webrtc_sideband_session_ended(&err) {
                        info!(
                            call_id,
                            "realtime sideband session ended while reconnecting"
                        );
                        break;
                    }
                    let mapped_error = map_api_error(err);
                    warn!(
                        reconnecting,
                        "failed to connect realtime sideband: {mapped_error}"
                    );
                    let _ = events_tx
                        .send(RealtimeEvent::Error(mapped_error.to_string()))
                        .await;
                    break;
                }
            };

            if !realtime_active.load(Ordering::Relaxed) || stop_token.is_cancelled() {
                break;
            }

            let exit = run_realtime_input_task(
                RealtimeInputTask {
                    writer: connection.writer(),
                    events: connection.events(),
                    text_rx: input_channels.text_rx.clone(),
                    handoff_output_rx: input_channels.handoff_output_rx.clone(),
                    audio_rx: input_channels.audio_rx.clone(),
                    events_tx: events_tx.clone(),
                    handoff_state: handoff_state.clone(),
                    session_kind,
                    event_parser,
                    stop_token: stop_token.clone(),
                },
                pending_outbound.take(),
            )
            .await;

            let RealtimeInputTaskExit::TransportLost {
                err,
                pending_outbound: failed_outbound,
            } = exit
            else {
                break;
            };
            pending_outbound = failed_outbound;
            if !realtime_active.load(Ordering::Relaxed) || stop_token.is_cancelled() {
                break;
            }
            if event_parser != RealtimeEventParser::FramelessBidi
                || matches!(
                    &err,
                    ApiError::Transport(codex_client::TransportError::Policy(_))
                )
            {
                report_realtime_transport_loss(&events_tx, err).await;
                break;
            }
            let mapped_error = map_api_error(err);
            let delay = mapped_error.server_retry_delay().unwrap_or_else(codex_client::fixed_retry_delay);
            tracing::debug!(
                call_id,
                delay_ms = delay.as_millis(),
                "live sideband transport lost; reconnecting: {mapped_error}"
            );
            reconnecting = true;
            tokio::select! {
                biased;
                _ = stop_token.cancelled() => break,
                _ = sleep(delay) => {}
            }
        }

        let transcript_tail = transcript_state.take_transcript_tail().await;
        flush_realtime_transcript_tail(&transcript_tail_flush, &transcript_tail).await;
    })
}

fn webrtc_sideband_session_ended(err: &ApiError) -> bool {
    matches!(
        err,
        ApiError::Api { status, .. } | ApiError::Transport(codex_client::TransportError::Http { status, .. })
            if matches!(*status, StatusCode::NOT_FOUND | StatusCode::GONE)
    )
}


#[cfg(test)]
#[path = "sideband_tests.rs"]
mod tests;
