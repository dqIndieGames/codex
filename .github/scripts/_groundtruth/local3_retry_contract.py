"""Product truth: docs/local3-custom-feature-checklist-2026-05-10.md, §§3.1, 3.4, 6.1.

User-approved rules (2026-10-28): ten seconds without valid server advice; a Responses WS request
(sampling or compaction) switches the session to HTTP after three consecutive WS
failures, and a WS 1009 switches immediately without another WS attempt.
@lastVerified 2026-10-14
"""
RETRY_SECONDS = 10
WS_FAILURES_BEFORE_HTTP = 3
WS_RETRY_AFTER_1009 = 0

# 用户在本会话确认并要求执行的六档规格：同文件 §6.1 / §6.4，2026-11-07。
# 编码参数为待视觉验收候选；测试不能把字节收益当作清晰度通过。
IMAGE_FAILURES_PER_ROUND = 3
IMAGE_LONG_EDGES = {3: 1280, 4: 768}
IMAGE_KEEP_RECENT = 5
IMAGE_PLACEHOLDER_SIZE = (1, 1)
RELEASE_TEST_SECONDS = 300
RELEASE_CHECK_RESERVE_SECONDS = 20
CLEANUP_RESERVE_SECONDS = 20
# 同一清单 §5.4 / 第九节：401 文件重读至少四次，跨越第三次恢复。
AUTH_FILE_RELOAD_FAILURES = 4
