use std::sync::Arc;
use std::time::Duration;

use crate::client::RequestRetryNotice;
use crate::client::RequestRetryNotifier;
use crate::codex::Session;
use crate::codex::TurnContext;
use crate::protocol::EventMsg;
use crate::protocol::WarningEvent;

pub(crate) fn make_request_retry_warning_notifier(
    sess: Arc<Session>,
    turn_context: Arc<TurnContext>,
) -> RequestRetryNotifier {
    Arc::new(move |notice: RequestRetryNotice| {
        let sess = sess.clone();
        let turn_context = turn_context.clone();
        tokio::spawn(async move {
            let message = format_request_retry_warning(&notice);
            sess.send_event(&turn_context, EventMsg::Warning(WarningEvent { message }))
                .await;
        });
    })
}

fn format_request_retry_warning(notice: &RequestRetryNotice) -> String {
    let status = notice
        .status
        .map(|status| status.as_u16().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let delay = format_delay(notice.delay);
    format!(
        "HTTP {status} from {}. Retrying in {delay} (retry {}/{})",
        notice.endpoint, notice.attempt, notice.max_attempts
    )
}

fn format_delay(delay: Duration) -> String {
    let seconds = delay.as_secs_f64();
    if seconds < 1.0 {
        format!("{}ms", delay.as_millis())
    } else if (seconds - seconds.round()).abs() < 0.05 {
        format!("{:.0}s", seconds.round())
    } else {
        format!("{seconds:.1}s")
    }
}
