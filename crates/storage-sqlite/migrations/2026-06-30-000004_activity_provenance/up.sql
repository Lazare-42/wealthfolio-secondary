-- Provenance / traceability for activities: where an imported activity came
-- from (email/pdf/csv/bank/chat) and, optionally, which other activity funded
-- it (e.g. a SELL that funded a LOAN_ORIGINATION).
CREATE TABLE IF NOT EXISTS activity_sources (
    id TEXT PRIMARY KEY NOT NULL,
    activity_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,            -- email | pdf | csv | bank | manual | chat
    source_ref TEXT,                      -- msgvault/gmail messageId, file name, url, run id
    funding_activity_id TEXT,             -- self-link: this activity was funded by another
    thread_id TEXT,                       -- chat thread that produced it, when applicable
    detail_json TEXT,                     -- freeform extra (note, amounts, confidence)
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_sources_activity ON activity_sources(activity_id);
CREATE INDEX IF NOT EXISTS idx_activity_sources_funding ON activity_sources(funding_activity_id);

-- Transaction emails the chat chose as the source for an import, snapshotted so
-- the link survives even if the upstream archive changes.
CREATE TABLE IF NOT EXISTS chat_source_emails (
    id TEXT PRIMARY KEY NOT NULL,
    thread_id TEXT,
    message_id TEXT NOT NULL,             -- msgvault / gmail message id
    subject TEXT,
    sender TEXT,
    sent_at TEXT,
    snapshot_json TEXT,                   -- headers + optional body/attachment refs
    linked_activity_id TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chat_source_emails_thread ON chat_source_emails(thread_id);
CREATE INDEX IF NOT EXISTS idx_chat_source_emails_msg ON chat_source_emails(message_id);
