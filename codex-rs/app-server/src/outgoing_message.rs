use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;

use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Result;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use codex_app_server_protocol::ServerRequestPayload;
use codex_otel::span_w3c_trace_context;
use codex_protocol::ThreadId;
use codex_protocol::protocol::W3cTraceContext;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::Duration;
use tracing::Instrument;
use tracing::Span;
use tracing::warn;

use crate::error_code::INTERNAL_ERROR_CODE;
use crate::server_request_error::TURN_TRANSITION_PENDING_REQUEST_ERROR_REASON;

#[cfg(test)]
use codex_protocol::account::PlanType;

const NOTIFICATION_COALESCING_WINDOW: Duration = Duration::from_millis(150);

pub(crate) type ClientRequestResult = std::result::Result<Result, JSONRPCErrorError>;

/// Stable identifier for a transport connection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ConnectionId(pub(crate) u64);

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Stable identifier for a client request scoped to a transport connection.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ConnectionRequestId {
    pub(crate) connection_id: ConnectionId,
    pub(crate) request_id: RequestId,
}

/// Trace data we keep for an incoming request until we send its final
/// response or error.
#[derive(Clone)]
pub(crate) struct RequestContext {
    request_id: ConnectionRequestId,
    span: Span,
    parent_trace: Option<W3cTraceContext>,
}

impl RequestContext {
    pub(crate) fn new(
        request_id: ConnectionRequestId,
        span: Span,
        parent_trace: Option<W3cTraceContext>,
    ) -> Self {
        Self {
            request_id,
            span,
            parent_trace,
        }
    }

    pub(crate) fn request_trace(&self) -> Option<W3cTraceContext> {
        span_w3c_trace_context(&self.span).or_else(|| self.parent_trace.clone())
    }

    pub(crate) fn span(&self) -> Span {
        self.span.clone()
    }

    fn record_turn_id(&self, turn_id: &str) {
        self.span.record("turn.id", turn_id);
    }
}

#[derive(Debug)]
pub(crate) enum OutgoingEnvelope {
    ToConnection {
        connection_id: ConnectionId,
        message: OutgoingMessage,
        write_complete_tx: Option<oneshot::Sender<()>>,
    },
    Broadcast {
        message: OutgoingMessage,
    },
}

#[derive(Debug)]
pub(crate) struct QueuedOutgoingMessage {
    pub(crate) message: OutgoingMessage,
    pub(crate) write_complete_tx: Option<oneshot::Sender<()>>,
}

impl QueuedOutgoingMessage {
    pub(crate) fn new(message: OutgoingMessage) -> Self {
        Self {
            message,
            write_complete_tx: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NotificationCoalescing {
    Disabled,
    Enabled,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum NotificationRecipients {
    Broadcast,
    Connections(Vec<ConnectionId>),
}

impl NotificationRecipients {
    fn from_connection_ids(connection_ids: &[ConnectionId]) -> Self {
        if connection_ids.is_empty() {
            Self::Broadcast
        } else {
            Self::Connections(connection_ids.to_vec())
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CoalescingNotificationKind {
    CommandExecutionOutputDelta {
        thread_id: String,
        turn_id: String,
        item_id: String,
    },
    FileChangeOutputDelta {
        thread_id: String,
        turn_id: String,
        item_id: String,
    },
    ThreadTokenUsageUpdated {
        thread_id: String,
        turn_id: String,
    },
    TurnDiffUpdated {
        thread_id: String,
        turn_id: String,
    },
    TurnPlanUpdated {
        thread_id: String,
        turn_id: String,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CoalescingKey {
    recipients: NotificationRecipients,
    kind: CoalescingNotificationKind,
}

impl CoalescingKey {
    fn from_notification(
        connection_ids: &[ConnectionId],
        notification: &ServerNotification,
    ) -> Option<Self> {
        let recipients = NotificationRecipients::from_connection_ids(connection_ids);
        let kind = match notification {
            ServerNotification::CommandExecutionOutputDelta(notification) => {
                CoalescingNotificationKind::CommandExecutionOutputDelta {
                    thread_id: notification.thread_id.clone(),
                    turn_id: notification.turn_id.clone(),
                    item_id: notification.item_id.clone(),
                }
            }
            ServerNotification::FileChangeOutputDelta(notification) => {
                CoalescingNotificationKind::FileChangeOutputDelta {
                    thread_id: notification.thread_id.clone(),
                    turn_id: notification.turn_id.clone(),
                    item_id: notification.item_id.clone(),
                }
            }
            ServerNotification::ThreadTokenUsageUpdated(notification) => {
                CoalescingNotificationKind::ThreadTokenUsageUpdated {
                    thread_id: notification.thread_id.clone(),
                    turn_id: notification.turn_id.clone(),
                }
            }
            ServerNotification::TurnDiffUpdated(notification) => {
                CoalescingNotificationKind::TurnDiffUpdated {
                    thread_id: notification.thread_id.clone(),
                    turn_id: notification.turn_id.clone(),
                }
            }
            ServerNotification::TurnPlanUpdated(notification) => {
                CoalescingNotificationKind::TurnPlanUpdated {
                    thread_id: notification.thread_id.clone(),
                    turn_id: notification.turn_id.clone(),
                }
            }
            _ => return None,
        };
        Some(Self { recipients, kind })
    }
}

#[derive(Debug)]
struct PendingCoalescedNotification {
    connection_ids: Vec<ConnectionId>,
    notification: ServerNotification,
}

impl PendingCoalescedNotification {
    fn merge(&mut self, notification: ServerNotification) {
        match (&mut self.notification, notification) {
            (
                ServerNotification::CommandExecutionOutputDelta(existing),
                ServerNotification::CommandExecutionOutputDelta(next),
            ) => existing.delta.push_str(&next.delta),
            (
                ServerNotification::FileChangeOutputDelta(existing),
                ServerNotification::FileChangeOutputDelta(next),
            ) => existing.delta.push_str(&next.delta),
            (_, latest) => self.notification = latest,
        }
    }
}

#[derive(Clone)]
struct NotificationCoalescer {
    pending: Arc<Mutex<HashMap<CoalescingKey, PendingCoalescedNotification>>>,
    send_lock: Arc<Mutex<()>>,
}

impl Default for NotificationCoalescer {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
            send_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl NotificationCoalescer {
    async fn coalesce_or_send_later(
        &self,
        sender: mpsc::Sender<OutgoingEnvelope>,
        connection_ids: &[ConnectionId],
        notification: ServerNotification,
    ) -> bool {
        let Some(key) = CoalescingKey::from_notification(connection_ids, &notification) else {
            return false;
        };

        let mut pending = self.pending.lock().await;
        match pending.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().merge(notification);
            }
            Entry::Vacant(entry) => {
                entry.insert(PendingCoalescedNotification {
                    connection_ids: connection_ids.to_vec(),
                    notification,
                });
                let pending_map = Arc::clone(&self.pending);
                let send_lock = Arc::clone(&self.send_lock);
                tokio::spawn(async move {
                    tokio::time::sleep(NOTIFICATION_COALESCING_WINDOW).await;
                    let _send_guard = send_lock.lock().await;
                    let pending_notification = {
                        let mut pending = pending_map.lock().await;
                        pending.remove(&key)
                    };
                    if let Some(pending_notification) = pending_notification {
                        send_outgoing_notification(
                            &sender,
                            pending_notification.connection_ids.as_slice(),
                            pending_notification.notification,
                        )
                        .await;
                    }
                });
            }
        }
        true
    }

    async fn flush_pending_for_recipients(
        &self,
        sender: &mpsc::Sender<OutgoingEnvelope>,
        connection_ids: &[ConnectionId],
    ) {
        let recipients = NotificationRecipients::from_connection_ids(connection_ids);
        let _send_guard = self.send_lock.lock().await;
        let pending_notifications = self.take_pending_for_recipients(recipients).await;

        for pending_notification in pending_notifications {
            send_outgoing_notification(
                sender,
                pending_notification.connection_ids.as_slice(),
                pending_notification.notification,
            )
            .await;
        }
    }

    async fn take_pending_for_recipients(
        &self,
        recipients: NotificationRecipients,
    ) -> Vec<PendingCoalescedNotification> {
        let mut pending = self.pending.lock().await;
        let keys = pending
            .keys()
            .filter(|key| key.recipients == recipients)
            .cloned()
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| pending.remove(&key))
            .collect::<Vec<_>>()
    }
}

/// Sends messages to the client and manages request callbacks.
pub(crate) struct OutgoingMessageSender {
    next_server_request_id: AtomicI64,
    sender: mpsc::Sender<OutgoingEnvelope>,
    request_id_to_callback: Mutex<HashMap<RequestId, PendingCallbackEntry>>,
    notification_coalescing_enabled: bool,
    notification_coalescer: NotificationCoalescer,
    /// Incoming requests that are still waiting on a final response or error.
    /// We keep them here because this is where responses, errors, and
    /// disconnect cleanup all get handled.
    request_contexts: Mutex<HashMap<ConnectionRequestId, RequestContext>>,
}

#[derive(Clone)]
pub(crate) struct ThreadScopedOutgoingMessageSender {
    outgoing: Arc<OutgoingMessageSender>,
    connection_ids: Arc<Vec<ConnectionId>>,
    thread_id: ThreadId,
}

struct PendingCallbackEntry {
    callback: oneshot::Sender<ClientRequestResult>,
    thread_id: Option<ThreadId>,
    request: ServerRequest,
}

impl ThreadScopedOutgoingMessageSender {
    pub(crate) fn new(
        outgoing: Arc<OutgoingMessageSender>,
        connection_ids: Vec<ConnectionId>,
        thread_id: ThreadId,
    ) -> Self {
        Self {
            outgoing,
            connection_ids: Arc::new(connection_ids),
            thread_id,
        }
    }

    pub(crate) async fn send_request(
        &self,
        payload: ServerRequestPayload,
    ) -> (RequestId, oneshot::Receiver<ClientRequestResult>) {
        self.outgoing
            .send_request_to_connections(
                Some(self.connection_ids.as_slice()),
                payload,
                Some(self.thread_id),
            )
            .await
    }

    pub(crate) async fn send_server_notification(&self, notification: ServerNotification) {
        if self.connection_ids.is_empty() {
            return;
        }
        self.outgoing
            .send_server_notification_to_connections(self.connection_ids.as_slice(), notification)
            .await;
    }

    pub(crate) async fn send_global_server_notification(&self, notification: ServerNotification) {
        self.outgoing.send_server_notification(notification).await;
    }

    pub(crate) async fn abort_pending_server_requests(&self) {
        self.outgoing
            .cancel_requests_for_thread(
                self.thread_id,
                Some(JSONRPCErrorError {
                    code: INTERNAL_ERROR_CODE,
                    message: "client request resolved because the turn state was changed"
                        .to_string(),
                    data: Some(serde_json::json!({ "reason": TURN_TRANSITION_PENDING_REQUEST_ERROR_REASON })),
                }),
            )
            .await
    }

    pub(crate) async fn send_response<T: Serialize>(
        &self,
        request_id: ConnectionRequestId,
        response: T,
    ) {
        self.outgoing.send_response(request_id, response).await;
    }

    pub(crate) async fn send_error(
        &self,
        request_id: ConnectionRequestId,
        error: JSONRPCErrorError,
    ) {
        self.outgoing.send_error(request_id, error).await;
    }
}

async fn send_outgoing_notification(
    sender: &mpsc::Sender<OutgoingEnvelope>,
    connection_ids: &[ConnectionId],
    notification: ServerNotification,
) {
    let outgoing_message = OutgoingMessage::AppServerNotification(notification);
    if connection_ids.is_empty() {
        if let Err(err) = sender
            .send(OutgoingEnvelope::Broadcast {
                message: outgoing_message,
            })
            .await
        {
            warn!("failed to send server notification to client: {err:?}");
        }
        return;
    }
    for connection_id in connection_ids {
        if let Err(err) = sender
            .send(OutgoingEnvelope::ToConnection {
                connection_id: *connection_id,
                message: outgoing_message.clone(),
                write_complete_tx: None,
            })
            .await
        {
            warn!("failed to send server notification to client: {err:?}");
        }
    }
}

impl OutgoingMessageSender {
    pub(crate) fn new(sender: mpsc::Sender<OutgoingEnvelope>) -> Self {
        Self::new_with_notification_coalescing(sender, NotificationCoalescing::Disabled)
    }

    pub(crate) fn new_with_notification_coalescing(
        sender: mpsc::Sender<OutgoingEnvelope>,
        notification_coalescing: NotificationCoalescing,
    ) -> Self {
        Self {
            next_server_request_id: AtomicI64::new(0),
            sender,
            request_id_to_callback: Mutex::new(HashMap::new()),
            notification_coalescing_enabled: matches!(
                notification_coalescing,
                NotificationCoalescing::Enabled
            ),
            notification_coalescer: NotificationCoalescer::default(),
            request_contexts: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn register_request_context(&self, request_context: RequestContext) {
        let mut request_contexts = self.request_contexts.lock().await;
        if request_contexts
            .insert(request_context.request_id.clone(), request_context)
            .is_some()
        {
            warn!("replaced unresolved request context");
        }
    }

    pub(crate) async fn connection_closed(&self, connection_id: ConnectionId) {
        let mut request_contexts = self.request_contexts.lock().await;
        request_contexts.retain(|request_id, _| request_id.connection_id != connection_id);
    }

    pub(crate) async fn request_trace_context(
        &self,
        request_id: &ConnectionRequestId,
    ) -> Option<W3cTraceContext> {
        let request_contexts = self.request_contexts.lock().await;
        request_contexts
            .get(request_id)
            .and_then(RequestContext::request_trace)
    }

    pub(crate) async fn record_request_turn_id(
        &self,
        request_id: &ConnectionRequestId,
        turn_id: &str,
    ) {
        let request_contexts = self.request_contexts.lock().await;
        if let Some(request_context) = request_contexts.get(request_id) {
            request_context.record_turn_id(turn_id);
        }
    }

    async fn take_request_context(
        &self,
        request_id: &ConnectionRequestId,
    ) -> Option<RequestContext> {
        let mut request_contexts = self.request_contexts.lock().await;
        request_contexts.remove(request_id)
    }

    #[cfg(test)]
    async fn request_context_count(&self) -> usize {
        self.request_contexts.lock().await.len()
    }

    pub(crate) async fn send_request(
        &self,
        request: ServerRequestPayload,
    ) -> (RequestId, oneshot::Receiver<ClientRequestResult>) {
        self.send_request_to_connections(
            /*connection_ids*/ None, request, /*thread_id*/ None,
        )
        .await
    }

    fn next_request_id(&self) -> RequestId {
        RequestId::Integer(self.next_server_request_id.fetch_add(1, Ordering::Relaxed))
    }

    async fn send_request_to_connections(
        &self,
        connection_ids: Option<&[ConnectionId]>,
        request: ServerRequestPayload,
        thread_id: Option<ThreadId>,
    ) -> (RequestId, oneshot::Receiver<ClientRequestResult>) {
        let id = self.next_request_id();
        let outgoing_message_id = id.clone();
        let request = request.request_with_id(outgoing_message_id.clone());

        let (tx_approve, rx_approve) = oneshot::channel();
        {
            let mut request_id_to_callback = self.request_id_to_callback.lock().await;
            request_id_to_callback.insert(
                id,
                PendingCallbackEntry {
                    callback: tx_approve,
                    thread_id,
                    request: request.clone(),
                },
            );
        }

        let outgoing_message = OutgoingMessage::Request(request);
        if self.notification_coalescing_enabled {
            self.notification_coalescer
                .flush_pending_for_recipients(&self.sender, connection_ids.unwrap_or(&[]))
                .await;
        }
        let send_result = match connection_ids {
            None => {
                self.sender
                    .send(OutgoingEnvelope::Broadcast {
                        message: outgoing_message,
                    })
                    .await
            }
            Some(connection_ids) => {
                let mut send_error = None;
                for connection_id in connection_ids {
                    if let Err(err) = self
                        .sender
                        .send(OutgoingEnvelope::ToConnection {
                            connection_id: *connection_id,
                            message: outgoing_message.clone(),
                            write_complete_tx: None,
                        })
                        .await
                    {
                        send_error = Some(err);
                        break;
                    }
                }
                match send_error {
                    Some(err) => Err(err),
                    None => Ok(()),
                }
            }
        };

        if let Err(err) = send_result {
            warn!("failed to send request {outgoing_message_id:?} to client: {err:?}");
            let mut request_id_to_callback = self.request_id_to_callback.lock().await;
            request_id_to_callback.remove(&outgoing_message_id);
        }
        (outgoing_message_id, rx_approve)
    }

    pub(crate) async fn replay_requests_to_connection_for_thread(
        &self,
        connection_id: ConnectionId,
        thread_id: ThreadId,
    ) {
        let requests = self.pending_requests_for_thread(thread_id).await;
        for request in requests {
            if let Err(err) = self
                .sender
                .send(OutgoingEnvelope::ToConnection {
                    connection_id,
                    message: OutgoingMessage::Request(request),
                    write_complete_tx: None,
                })
                .await
            {
                warn!("failed to resend request to client: {err:?}");
            }
        }
    }

    pub(crate) async fn notify_client_response(&self, id: RequestId, result: Result) {
        let entry = self.take_request_callback(&id).await;

        match entry {
            Some((id, entry)) => {
                if let Err(err) = entry.callback.send(Ok(result)) {
                    warn!("could not notify callback for {id:?} due to: {err:?}");
                }
            }
            None => {
                warn!("could not find callback for {id:?}");
            }
        }
    }

    pub(crate) async fn notify_client_error(&self, id: RequestId, error: JSONRPCErrorError) {
        let entry = self.take_request_callback(&id).await;

        match entry {
            Some((id, entry)) => {
                warn!("client responded with error for {id:?}: {error:?}");
                if let Err(err) = entry.callback.send(Err(error)) {
                    warn!("could not notify callback for {id:?} due to: {err:?}");
                }
            }
            None => {
                warn!("could not find callback for {id:?}");
            }
        }
    }

    pub(crate) async fn cancel_request(&self, id: &RequestId) -> bool {
        self.take_request_callback(id).await.is_some()
    }

    pub(crate) async fn cancel_all_requests(&self, error: Option<JSONRPCErrorError>) {
        let entries = {
            let mut request_id_to_callback = self.request_id_to_callback.lock().await;
            request_id_to_callback
                .drain()
                .map(|(_, entry)| entry)
                .collect::<Vec<_>>()
        };

        if let Some(error) = error {
            for entry in entries {
                if let Err(err) = entry.callback.send(Err(error.clone())) {
                    let request_id = entry.request.id();
                    warn!("could not notify callback for {request_id:?} due to: {err:?}");
                }
            }
        }
    }

    async fn take_request_callback(
        &self,
        id: &RequestId,
    ) -> Option<(RequestId, PendingCallbackEntry)> {
        let mut request_id_to_callback = self.request_id_to_callback.lock().await;
        request_id_to_callback.remove_entry(id)
    }

    pub(crate) async fn pending_requests_for_thread(
        &self,
        thread_id: ThreadId,
    ) -> Vec<ServerRequest> {
        let request_id_to_callback = self.request_id_to_callback.lock().await;
        let mut requests = request_id_to_callback
            .iter()
            .filter_map(|(_, entry)| {
                (entry.thread_id == Some(thread_id)).then_some(entry.request.clone())
            })
            .collect::<Vec<_>>();
        requests.sort_by(|left, right| left.id().cmp(right.id()));
        requests
    }

    pub(crate) async fn cancel_requests_for_thread(
        &self,
        thread_id: ThreadId,
        error: Option<JSONRPCErrorError>,
    ) {
        let entries = {
            let mut request_id_to_callback = self.request_id_to_callback.lock().await;
            let request_ids = request_id_to_callback
                .iter()
                .filter_map(|(request_id, entry)| {
                    (entry.thread_id == Some(thread_id)).then_some(request_id.clone())
                })
                .collect::<Vec<_>>();

            let mut entries = Vec::with_capacity(request_ids.len());
            for request_id in request_ids {
                if let Some(entry) = request_id_to_callback.remove(&request_id) {
                    entries.push(entry);
                }
            }
            entries
        };

        if let Some(error) = error {
            for entry in entries {
                if let Err(err) = entry.callback.send(Err(error.clone())) {
                    let request_id = entry.request.id();
                    warn!("could not notify callback for {request_id:?} due to: {err:?}",);
                }
            }
        }
    }

    pub(crate) async fn send_response<T: Serialize>(
        &self,
        request_id: ConnectionRequestId,
        response: T,
    ) {
        let request_context = self.take_request_context(&request_id).await;
        match serde_json::to_value(response) {
            Ok(result) => {
                let outgoing_message = OutgoingMessage::Response(OutgoingResponse {
                    id: request_id.request_id.clone(),
                    result,
                });
                self.send_outgoing_message_to_connection(
                    request_context,
                    request_id.connection_id,
                    outgoing_message,
                    "response",
                )
                .await;
            }
            Err(err) => {
                self.send_error_inner(
                    request_context,
                    request_id,
                    JSONRPCErrorError {
                        code: INTERNAL_ERROR_CODE,
                        message: format!("failed to serialize response: {err}"),
                        data: None,
                    },
                )
                .await;
            }
        }
    }

    pub(crate) async fn send_server_notification(&self, notification: ServerNotification) {
        self.send_server_notification_to_connections(&[], notification)
            .await;
    }

    pub(crate) async fn send_server_notification_to_connections(
        &self,
        connection_ids: &[ConnectionId],
        notification: ServerNotification,
    ) {
        tracing::trace!(
            targeted_connections = connection_ids.len(),
            "app-server event: {notification}"
        );
        if self.notification_coalescing_enabled
            && self
                .notification_coalescer
                .coalesce_or_send_later(self.sender.clone(), connection_ids, notification.clone())
                .await
        {
            return;
        }
        if self.notification_coalescing_enabled {
            let recipients = NotificationRecipients::from_connection_ids(connection_ids);
            let _send_guard = self.notification_coalescer.send_lock.lock().await;
            let pending_notifications = self
                .notification_coalescer
                .take_pending_for_recipients(recipients)
                .await;

            for pending_notification in pending_notifications {
                send_outgoing_notification(
                    &self.sender,
                    pending_notification.connection_ids.as_slice(),
                    pending_notification.notification,
                )
                .await;
            }
            send_outgoing_notification(&self.sender, connection_ids, notification).await;
            return;
        }
        send_outgoing_notification(&self.sender, connection_ids, notification).await;
    }

    pub(crate) async fn send_server_notification_to_connection_and_wait(
        &self,
        connection_id: ConnectionId,
        notification: ServerNotification,
    ) {
        tracing::trace!("app-server event: {notification}");
        if self.notification_coalescing_enabled {
            let connection_ids = [connection_id];
            self.notification_coalescer
                .flush_pending_for_recipients(&self.sender, &connection_ids)
                .await;
        }
        let outgoing_message = OutgoingMessage::AppServerNotification(notification);
        let (write_complete_tx, write_complete_rx) = oneshot::channel();
        if let Err(err) = self
            .sender
            .send(OutgoingEnvelope::ToConnection {
                connection_id,
                message: outgoing_message,
                write_complete_tx: Some(write_complete_tx),
            })
            .await
        {
            warn!("failed to send server notification to client: {err:?}");
        }
        let _ = write_complete_rx.await;
    }

    pub(crate) async fn send_error(
        &self,
        request_id: ConnectionRequestId,
        error: JSONRPCErrorError,
    ) {
        let request_context = self.take_request_context(&request_id).await;
        self.send_error_inner(request_context, request_id, error)
            .await;
    }

    async fn send_error_inner(
        &self,
        request_context: Option<RequestContext>,
        request_id: ConnectionRequestId,
        error: JSONRPCErrorError,
    ) {
        let outgoing_message = OutgoingMessage::Error(OutgoingError {
            id: request_id.request_id,
            error,
        });
        self.send_outgoing_message_to_connection(
            request_context,
            request_id.connection_id,
            outgoing_message,
            "error",
        )
        .await;
    }

    async fn send_outgoing_message_to_connection(
        &self,
        request_context: Option<RequestContext>,
        connection_id: ConnectionId,
        message: OutgoingMessage,
        message_kind: &'static str,
    ) {
        if self.notification_coalescing_enabled {
            let connection_ids = [connection_id];
            self.notification_coalescer
                .flush_pending_for_recipients(&self.sender, &connection_ids)
                .await;
        }
        let send_fut = self.sender.send(OutgoingEnvelope::ToConnection {
            connection_id,
            message,
            write_complete_tx: None,
        });
        let send_result = if let Some(request_context) = request_context {
            send_fut.instrument(request_context.span()).await
        } else {
            send_fut.await
        };

        if let Err(err) = send_result {
            warn!("failed to send {message_kind} to client: {err:?}");
        }
    }
}

/// Outgoing message from the server to the client.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub(crate) enum OutgoingMessage {
    Request(ServerRequest),
    /// AppServerNotification is specific to the case where this is run as an
    /// "app server" as opposed to an MCP server.
    AppServerNotification(ServerNotification),
    Response(OutgoingResponse),
    Error(OutgoingError),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct OutgoingResponse {
    pub id: RequestId,
    pub result: Result,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct OutgoingError {
    pub error: JSONRPCErrorError,
    pub id: RequestId,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use codex_app_server_protocol::AccountLoginCompletedNotification;
    use codex_app_server_protocol::AccountRateLimitsUpdatedNotification;
    use codex_app_server_protocol::AccountUpdatedNotification;
    use codex_app_server_protocol::ApplyPatchApprovalParams;
    use codex_app_server_protocol::AuthMode;
    use codex_app_server_protocol::CommandExecutionOutputDeltaNotification;
    use codex_app_server_protocol::ConfigWarningNotification;
    use codex_app_server_protocol::DynamicToolCallParams;
    use codex_app_server_protocol::FileChangeOutputDeltaNotification;
    use codex_app_server_protocol::FileChangeRequestApprovalParams;
    use codex_app_server_protocol::GuardianWarningNotification;
    use codex_app_server_protocol::ModelRerouteReason;
    use codex_app_server_protocol::ModelReroutedNotification;
    use codex_app_server_protocol::ModelVerification;
    use codex_app_server_protocol::ModelVerificationNotification;
    use codex_app_server_protocol::RateLimitSnapshot;
    use codex_app_server_protocol::RateLimitWindow;
    use codex_app_server_protocol::ThreadTokenUsage;
    use codex_app_server_protocol::ThreadTokenUsageUpdatedNotification;
    use codex_app_server_protocol::ToolRequestUserInputParams;
    use codex_app_server_protocol::TokenUsageBreakdown;
    use codex_app_server_protocol::Turn;
    use codex_app_server_protocol::TurnCompletedNotification;
    use codex_app_server_protocol::TurnDiffUpdatedNotification;
    use codex_app_server_protocol::TurnPlanStep;
    use codex_app_server_protocol::TurnPlanStepStatus;
    use codex_app_server_protocol::TurnPlanUpdatedNotification;
    use codex_app_server_protocol::TurnStatus;
    use codex_protocol::ThreadId;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::time::timeout;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn verify_server_notification_serialization() {
        let notification =
            ServerNotification::AccountLoginCompleted(AccountLoginCompletedNotification {
                login_id: Some(Uuid::nil().to_string()),
                success: true,
                error: None,
            });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "account/login/completed",
                "params": {
                    "loginId": Uuid::nil().to_string(),
                    "success": true,
                    "error": null,
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the strum macros serialize the method field correctly"),
            "ensure the strum macros serialize the method field correctly"
        );
    }

    #[test]
    fn verify_account_login_completed_notification_serialization() {
        let notification =
            ServerNotification::AccountLoginCompleted(AccountLoginCompletedNotification {
                login_id: Some(Uuid::nil().to_string()),
                success: true,
                error: None,
            });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "account/login/completed",
                "params": {
                    "loginId": Uuid::nil().to_string(),
                    "success": true,
                    "error": null,
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_account_rate_limits_notification_serialization() {
        let notification =
            ServerNotification::AccountRateLimitsUpdated(AccountRateLimitsUpdatedNotification {
                rate_limits: RateLimitSnapshot {
                    limit_id: Some("codex".to_string()),
                    limit_name: None,
                    primary: Some(RateLimitWindow {
                        used_percent: 25,
                        window_duration_mins: Some(15),
                        resets_at: Some(123),
                    }),
                    secondary: None,
                    credits: None,
                    plan_type: Some(PlanType::Plus),
                    rate_limit_reached_type: None,
                },
            });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "account/rateLimits/updated",
                "params": {
                        "rateLimits": {
                        "limitId": "codex",
                        "limitName": null,
                        "primary": {
                            "usedPercent": 25,
                            "windowDurationMins": 15,
                            "resetsAt": 123
                        },
                        "secondary": null,
                        "credits": null,
                        "planType": "plus",
                        "rateLimitReachedType": null
                    }
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_account_updated_notification_serialization() {
        let notification = ServerNotification::AccountUpdated(AccountUpdatedNotification {
            auth_mode: Some(AuthMode::ApiKey),
            plan_type: None,
        });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "account/updated",
                "params": {
                    "authMode": "apikey",
                    "planType": null
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_config_warning_notification_serialization() {
        let notification = ServerNotification::ConfigWarning(ConfigWarningNotification {
            summary: "Config error: using defaults".to_string(),
            details: Some("error loading config: bad config".to_string()),
            path: None,
            range: None,
        });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!( {
                "method": "configWarning",
                "params": {
                    "summary": "Config error: using defaults",
                    "details": "error loading config: bad config",
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_guardian_warning_notification_serialization() {
        let notification = ServerNotification::GuardianWarning(GuardianWarningNotification {
            thread_id: "thread-1".to_string(),
            message: "Automatic approval review denied the requested action.".to_string(),
        });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "guardianWarning",
                "params": {
                    "threadId": "thread-1",
                    "message": "Automatic approval review denied the requested action.",
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_model_rerouted_notification_serialization() {
        let notification = ServerNotification::ModelRerouted(ModelReroutedNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            from_model: "gpt-5.3-codex".to_string(),
            to_model: "gpt-5.2".to_string(),
            reason: ModelRerouteReason::HighRiskCyberActivity,
        });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "model/rerouted",
                "params": {
                    "threadId": "thread-1",
                    "turnId": "turn-1",
                    "fromModel": "gpt-5.3-codex",
                    "toModel": "gpt-5.2",
                    "reason": "highRiskCyberActivity",
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[test]
    fn verify_model_verification_notification_serialization() {
        let notification = ServerNotification::ModelVerification(ModelVerificationNotification {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            verifications: vec![ModelVerification::TrustedAccessForCyber],
        });

        let jsonrpc_notification = OutgoingMessage::AppServerNotification(notification);
        assert_eq!(
            json!({
                "method": "model/verification",
                "params": {
                    "threadId": "thread-1",
                    "turnId": "turn-1",
                    "verifications": ["trustedAccessForCyber"],
                },
            }),
            serde_json::to_value(jsonrpc_notification)
                .expect("ensure the notification serializes correctly"),
            "ensure the notification serializes correctly"
        );
    }

    #[tokio::test]
    async fn send_response_routes_to_target_connection() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);
        let request_id = ConnectionRequestId {
            connection_id: ConnectionId(42),
            request_id: RequestId::Integer(7),
        };

        outgoing
            .send_response(request_id.clone(), json!({ "ok": true }))
            .await;

        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("should receive envelope before timeout")
            .expect("channel should contain one message");

        match envelope {
            OutgoingEnvelope::ToConnection {
                connection_id,
                message,
                ..
            } => {
                assert_eq!(connection_id, ConnectionId(42));
                let OutgoingMessage::Response(response) = message else {
                    panic!("expected response message");
                };
                assert_eq!(response.id, request_id.request_id);
                assert_eq!(response.result, json!({ "ok": true }));
            }
            other => panic!("expected targeted response envelope, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_response_clears_registered_request_context() {
        let (tx, _rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);
        let request_id = ConnectionRequestId {
            connection_id: ConnectionId(42),
            request_id: RequestId::Integer(7),
        };

        outgoing
            .register_request_context(RequestContext::new(
                request_id.clone(),
                tracing::info_span!("app_server.request", rpc.method = "thread/start"),
                /*parent_trace*/ None,
            ))
            .await;
        assert_eq!(outgoing.request_context_count().await, 1);

        outgoing
            .send_response(request_id, json!({ "ok": true }))
            .await;

        assert_eq!(outgoing.request_context_count().await, 0);
    }

    #[tokio::test]
    async fn send_error_routes_to_target_connection() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);
        let request_id = ConnectionRequestId {
            connection_id: ConnectionId(9),
            request_id: RequestId::Integer(3),
        };
        let error = JSONRPCErrorError {
            code: INTERNAL_ERROR_CODE,
            message: "boom".to_string(),
            data: None,
        };

        outgoing.send_error(request_id.clone(), error.clone()).await;

        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("should receive envelope before timeout")
            .expect("channel should contain one message");

        match envelope {
            OutgoingEnvelope::ToConnection {
                connection_id,
                message,
                ..
            } => {
                assert_eq!(connection_id, ConnectionId(9));
                let OutgoingMessage::Error(outgoing_error) = message else {
                    panic!("expected error message");
                };
                assert_eq!(outgoing_error.id, RequestId::Integer(3));
                assert_eq!(outgoing_error.error, error);
            }
            other => panic!("expected targeted error envelope, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn notification_coalescing_default_sends_deltas_immediately() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);

        outgoing
            .send_server_notification(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: "hello".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: " world".to_string(),
                },
            ))
            .await;

        let first = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("first delta should be sent immediately")
            .expect("channel should contain the first delta");
        let second = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("second delta should be sent immediately")
            .expect("channel should contain the second delta");

        assert!(matches!(
            first,
            OutgoingEnvelope::Broadcast {
                message: OutgoingMessage::AppServerNotification(
                    ServerNotification::CommandExecutionOutputDelta(_)
                )
            }
        ));
        assert!(matches!(
            second,
            OutgoingEnvelope::Broadcast {
                message: OutgoingMessage::AppServerNotification(
                    ServerNotification::CommandExecutionOutputDelta(_)
                )
            }
        ));
    }

    #[tokio::test]
    async fn notification_coalescing_merges_command_output_deltas() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new_with_notification_coalescing(
            tx,
            NotificationCoalescing::Enabled,
        );

        outgoing
            .send_server_notification(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: "hello".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: " world".to_string(),
                },
            ))
            .await;

        let early = timeout(Duration::from_millis(25), rx.recv()).await;
        assert!(early.is_err(), "coalesced delta should wait for the batch window");

        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("merged delta should be sent after the batch window")
            .expect("channel should contain the merged delta");
        let OutgoingEnvelope::Broadcast {
            message:
                OutgoingMessage::AppServerNotification(
                    ServerNotification::CommandExecutionOutputDelta(notification),
                ),
        } = envelope
        else {
            panic!("expected merged command output delta envelope");
        };
        assert_eq!(notification.delta, "hello world");

        let extra = timeout(Duration::from_millis(25), rx.recv()).await;
        assert!(extra.is_err(), "merged deltas should produce one outgoing message");
    }

    #[tokio::test]
    async fn notification_coalescing_flushes_pending_delta_before_completion() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new_with_notification_coalescing(
            tx,
            NotificationCoalescing::Enabled,
        );

        outgoing
            .send_server_notification(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "item-1".to_string(),
                    delta: "last output".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::TurnCompleted(
                TurnCompletedNotification {
                    thread_id: "thread-1".to_string(),
                    turn: completed_turn("turn-1"),
                },
            ))
            .await;

        let first = recv_broadcast_notification(&mut rx).await;
        let ServerNotification::CommandExecutionOutputDelta(delta) = first else {
            panic!("expected pending output delta to flush first");
        };
        assert_eq!(delta.delta, "last output");

        let second = recv_broadcast_notification(&mut rx).await;
        assert!(matches!(second, ServerNotification::TurnCompleted(_)));
    }

    #[tokio::test]
    async fn notification_coalescing_flushes_pending_targeted_delta_before_error() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new_with_notification_coalescing(
            tx,
            NotificationCoalescing::Enabled,
        );
        let connection_id = ConnectionId(7);

        outgoing
            .send_server_notification_to_connections(
                &[connection_id],
                ServerNotification::CommandExecutionOutputDelta(
                    CommandExecutionOutputDeltaNotification {
                        thread_id: "thread-1".to_string(),
                        turn_id: "turn-1".to_string(),
                        item_id: "item-1".to_string(),
                        delta: "targeted output".to_string(),
                    },
                ),
            )
            .await;
        outgoing
            .send_error(
                ConnectionRequestId {
                    connection_id,
                    request_id: RequestId::Integer(8),
                },
                JSONRPCErrorError {
                    code: INTERNAL_ERROR_CODE,
                    message: "boom".to_string(),
                    data: None,
                },
            )
            .await;

        let first = recv_targeted_message(&mut rx, connection_id).await;
        let OutgoingMessage::AppServerNotification(
            ServerNotification::CommandExecutionOutputDelta(delta),
        ) = first
        else {
            panic!("expected pending targeted output delta to flush first");
        };
        assert_eq!(delta.delta, "targeted output");

        let second = recv_targeted_message(&mut rx, connection_id).await;
        assert!(matches!(second, OutgoingMessage::Error(_)));
    }

    #[tokio::test]
    async fn notification_coalescing_merges_each_supported_notification_kind() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(8);
        let outgoing = OutgoingMessageSender::new_with_notification_coalescing(
            tx,
            NotificationCoalescing::Enabled,
        );

        outgoing
            .send_server_notification(ServerNotification::FileChangeOutputDelta(
                FileChangeOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "patch-1".to_string(),
                    delta: "diff".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::FileChangeOutputDelta(
                FileChangeOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "patch-1".to_string(),
                    delta: " chunk".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::ThreadTokenUsageUpdated(
                ThreadTokenUsageUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    token_usage: thread_token_usage(/*total_tokens*/ 1),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::ThreadTokenUsageUpdated(
                ThreadTokenUsageUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    token_usage: thread_token_usage(/*total_tokens*/ 2),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::TurnDiffUpdated(
                TurnDiffUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    diff: "old diff".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::TurnDiffUpdated(
                TurnDiffUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    diff: "new diff".to_string(),
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::TurnPlanUpdated(
                TurnPlanUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    explanation: Some("old plan".to_string()),
                    plan: vec![TurnPlanStep {
                        step: "step".to_string(),
                        status: TurnPlanStepStatus::Pending,
                    }],
                },
            ))
            .await;
        outgoing
            .send_server_notification(ServerNotification::TurnPlanUpdated(
                TurnPlanUpdatedNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    explanation: Some("new plan".to_string()),
                    plan: vec![TurnPlanStep {
                        step: "step".to_string(),
                        status: TurnPlanStepStatus::Completed,
                    }],
                },
            ))
            .await;

        let notifications = recv_broadcast_notifications(&mut rx, /*count*/ 4).await;

        let file_delta = notifications.iter().find_map(|notification| match notification {
            ServerNotification::FileChangeOutputDelta(notification) => Some(notification),
            _ => None,
        });
        assert_eq!(
            file_delta.expect("file delta should be emitted").delta,
            "diff chunk"
        );

        let token_usage = notifications.iter().find_map(|notification| match notification {
            ServerNotification::ThreadTokenUsageUpdated(notification) => {
                Some(&notification.token_usage)
            }
            _ => None,
        });
        assert_eq!(
            token_usage
                .expect("token usage should be emitted")
                .total
                .total_tokens,
            2
        );

        let diff = notifications.iter().find_map(|notification| match notification {
            ServerNotification::TurnDiffUpdated(notification) => Some(notification.diff.as_str()),
            _ => None,
        });
        assert_eq!(diff, Some("new diff"));

        let plan = notifications.iter().find_map(|notification| match notification {
            ServerNotification::TurnPlanUpdated(notification) => Some(notification),
            _ => None,
        });
        let plan = plan.expect("plan should be emitted");
        assert_eq!(plan.explanation.as_deref(), Some("new plan"));
        assert_eq!(plan.plan[0].status, TurnPlanStepStatus::Completed);

        let extra = timeout(Duration::from_millis(25), rx.recv()).await;
        assert!(extra.is_err(), "coalescing should emit only one notification per key");
    }

    fn completed_turn(id: &str) -> Turn {
        Turn {
            id: id.to_string(),
            items: Vec::new(),
            status: TurnStatus::Completed,
            error: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }

    fn thread_token_usage(total_tokens: i64) -> ThreadTokenUsage {
        let breakdown = TokenUsageBreakdown {
            total_tokens,
            input_tokens: total_tokens,
            cached_input_tokens: 0,
            output_tokens: 0,
            reasoning_output_tokens: 0,
        };
        ThreadTokenUsage {
            total: breakdown.clone(),
            last: breakdown,
            model_context_window: Some(100),
        }
    }

    async fn recv_broadcast_notifications(
        rx: &mut mpsc::Receiver<OutgoingEnvelope>,
        count: usize,
    ) -> Vec<ServerNotification> {
        let mut notifications = Vec::with_capacity(count);
        for _ in 0..count {
            notifications.push(recv_broadcast_notification(rx).await);
        }
        notifications
    }

    async fn recv_broadcast_notification(
        rx: &mut mpsc::Receiver<OutgoingEnvelope>,
    ) -> ServerNotification {
        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("notification should arrive before timeout")
            .expect("channel should contain an envelope");
        let OutgoingEnvelope::Broadcast {
            message: OutgoingMessage::AppServerNotification(notification),
        } = envelope
        else {
            panic!("expected broadcast app-server notification envelope");
        };
        notification
    }

    async fn recv_targeted_message(
        rx: &mut mpsc::Receiver<OutgoingEnvelope>,
        expected_connection_id: ConnectionId,
    ) -> OutgoingMessage {
        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("message should arrive before timeout")
            .expect("channel should contain an envelope");
        let OutgoingEnvelope::ToConnection {
            connection_id,
            message,
            ..
        } = envelope
        else {
            panic!("expected targeted envelope");
        };
        assert_eq!(connection_id, expected_connection_id);
        message
    }

    #[tokio::test]
    async fn send_server_notification_to_connection_and_wait_tracks_write_completion() {
        let (tx, mut rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);
        let send_task = tokio::spawn(async move {
            outgoing
                .send_server_notification_to_connection_and_wait(
                    ConnectionId(42),
                    ServerNotification::ModelRerouted(ModelReroutedNotification {
                        thread_id: "thread-1".to_string(),
                        turn_id: "turn-1".to_string(),
                        from_model: "gpt-5.3-codex".to_string(),
                        to_model: "gpt-5.2".to_string(),
                        reason: ModelRerouteReason::HighRiskCyberActivity,
                    }),
                )
                .await
        });

        let envelope = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("should receive envelope before timeout")
            .expect("channel should contain one message");
        let OutgoingEnvelope::ToConnection {
            connection_id,
            message,
            write_complete_tx,
        } = envelope
        else {
            panic!("expected targeted server notification envelope");
        };
        assert_eq!(connection_id, ConnectionId(42));
        assert!(matches!(message, OutgoingMessage::AppServerNotification(_)));
        write_complete_tx
            .expect("write completion sender should be attached")
            .send(())
            .expect("receiver should still be waiting");

        timeout(Duration::from_secs(1), send_task)
            .await
            .expect("send task should finish after write completion is signaled")
            .expect("send task should not panic");
    }

    #[tokio::test]
    async fn connection_closed_clears_registered_request_contexts() {
        let (tx, _rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);
        let closed_connection_request = ConnectionRequestId {
            connection_id: ConnectionId(9),
            request_id: RequestId::Integer(3),
        };
        let open_connection_request = ConnectionRequestId {
            connection_id: ConnectionId(10),
            request_id: RequestId::Integer(4),
        };

        outgoing
            .register_request_context(RequestContext::new(
                closed_connection_request,
                tracing::info_span!("app_server.request", rpc.method = "turn/interrupt"),
                /*parent_trace*/ None,
            ))
            .await;
        outgoing
            .register_request_context(RequestContext::new(
                open_connection_request,
                tracing::info_span!("app_server.request", rpc.method = "turn/start"),
                /*parent_trace*/ None,
            ))
            .await;
        assert_eq!(outgoing.request_context_count().await, 2);

        outgoing.connection_closed(ConnectionId(9)).await;

        assert_eq!(outgoing.request_context_count().await, 1);
    }

    #[tokio::test]
    async fn notify_client_error_forwards_error_to_waiter() {
        let (tx, _rx) = mpsc::channel::<OutgoingEnvelope>(4);
        let outgoing = OutgoingMessageSender::new(tx);

        let (request_id, wait_for_result) = outgoing
            .send_request(ServerRequestPayload::ApplyPatchApproval(
                ApplyPatchApprovalParams {
                    conversation_id: ThreadId::new(),
                    call_id: "call-id".to_string(),
                    file_changes: HashMap::new(),
                    reason: None,
                    grant_root: None,
                },
            ))
            .await;

        let error = JSONRPCErrorError {
            code: INTERNAL_ERROR_CODE,
            message: "refresh failed".to_string(),
            data: None,
        };

        outgoing
            .notify_client_error(request_id, error.clone())
            .await;

        let result = timeout(Duration::from_secs(1), wait_for_result)
            .await
            .expect("wait should not time out")
            .expect("waiter should receive a callback");
        assert_eq!(result, Err(error));
    }

    #[tokio::test]
    async fn pending_requests_for_thread_returns_thread_requests_in_request_id_order() {
        let (tx, _rx) = mpsc::channel::<OutgoingEnvelope>(8);
        let outgoing = Arc::new(OutgoingMessageSender::new(tx));
        let thread_id = ThreadId::new();
        let thread_outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing.clone(),
            vec![ConnectionId(1)],
            thread_id,
        );

        let (dynamic_tool_request_id, _dynamic_tool_waiter) = thread_outgoing
            .send_request(ServerRequestPayload::DynamicToolCall(
                DynamicToolCallParams {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    call_id: "call-0".to_string(),
                    namespace: None,
                    tool: "tool".to_string(),
                    arguments: json!({}),
                },
            ))
            .await;
        let (first_request_id, _first_waiter) = thread_outgoing
            .send_request(ServerRequestPayload::ToolRequestUserInput(
                ToolRequestUserInputParams {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "call-1".to_string(),
                    questions: vec![],
                },
            ))
            .await;
        let (second_request_id, _second_waiter) = thread_outgoing
            .send_request(ServerRequestPayload::FileChangeRequestApproval(
                FileChangeRequestApprovalParams {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "call-2".to_string(),
                    reason: None,
                    grant_root: None,
                },
            ))
            .await;
        let pending_requests = outgoing.pending_requests_for_thread(thread_id).await;
        assert_eq!(
            pending_requests
                .iter()
                .map(ServerRequest::id)
                .collect::<Vec<_>>(),
            vec![
                &dynamic_tool_request_id,
                &first_request_id,
                &second_request_id
            ]
        );
    }

    #[tokio::test]
    async fn cancel_requests_for_thread_cancels_all_thread_requests() {
        let (tx, _rx) = mpsc::channel::<OutgoingEnvelope>(8);
        let outgoing = Arc::new(OutgoingMessageSender::new(tx));
        let thread_id = ThreadId::new();
        let thread_outgoing = ThreadScopedOutgoingMessageSender::new(
            outgoing.clone(),
            vec![ConnectionId(1)],
            thread_id,
        );

        let (_dynamic_tool_request_id, dynamic_tool_waiter) = thread_outgoing
            .send_request(ServerRequestPayload::DynamicToolCall(
                DynamicToolCallParams {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    call_id: "call-0".to_string(),
                    namespace: None,
                    tool: "tool".to_string(),
                    arguments: json!({}),
                },
            ))
            .await;
        let (_request_id, user_input_waiter) = thread_outgoing
            .send_request(ServerRequestPayload::ToolRequestUserInput(
                ToolRequestUserInputParams {
                    thread_id: thread_id.to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "call-1".to_string(),
                    questions: vec![],
                },
            ))
            .await;
        let error = JSONRPCErrorError {
            code: INTERNAL_ERROR_CODE,
            message: "tracked request cancelled".to_string(),
            data: None,
        };

        outgoing
            .cancel_requests_for_thread(thread_id, Some(error.clone()))
            .await;

        let dynamic_tool_result = timeout(Duration::from_secs(1), dynamic_tool_waiter)
            .await
            .expect("dynamic tool waiter should resolve")
            .expect("dynamic tool waiter should receive a callback");
        let user_input_result = timeout(Duration::from_secs(1), user_input_waiter)
            .await
            .expect("user input waiter should resolve")
            .expect("user input waiter should receive a callback");
        assert_eq!(dynamic_tool_result, Err(error.clone()));
        assert_eq!(user_input_result, Err(error));
        assert!(
            outgoing
                .pending_requests_for_thread(thread_id)
                .await
                .is_empty()
        );
    }
}
