"""Product truth: docs/local3-custom-feature-checklist-2026-05-10.md, §§3.1, 3.6, 6.

User-approved rules: ten seconds per failure; normal compaction switches after
three WS failures; first 1009 gets exactly one additional WS attempt.
"""
RETRY_SECONDS = 10
WS_FAILURES_BEFORE_HTTP = 3
WS_RETRY_AFTER_1009 = 1
