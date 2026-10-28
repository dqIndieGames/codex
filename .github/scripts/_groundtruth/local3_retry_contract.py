"""Product truth: docs/local3-custom-feature-checklist-2026-05-10.md, §§3.1, 3.4, 6.1.

User-approved rules (2026-10-28): ten seconds without valid server advice; a Responses WS request
(sampling or compaction) switches the session to HTTP after three consecutive WS
failures, and a WS 1009 switches immediately without another WS attempt.
@lastVerified 2026-10-14
"""
RETRY_SECONDS = 10
WS_FAILURES_BEFORE_HTTP = 3
WS_RETRY_AFTER_1009 = 0
