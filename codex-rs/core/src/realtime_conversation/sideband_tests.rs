use super::*;

#[test]
fn recognizes_terminal_sideband_statuses() {
    for status in [StatusCode::NOT_FOUND, StatusCode::GONE] {
        assert!(webrtc_sideband_session_ended(&ApiError::Api {
            status,
            message: "session ended".to_string(),
        }));
    }
    assert!(!webrtc_sideband_session_ended(&ApiError::Api {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: "retry".to_string(),
    }));
}
