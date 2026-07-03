//! Email order import pipeline: watches JSON envelopes extracted from email and
//! imports executed activities via the existing ActivityImport flow.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tracing::{error, info, warn};
use wealthfolio_core::activities::{ActivityImport, ActivityServiceTrait};

use crate::inbox_fs::{is_json, move_file};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailOrderEnvelope {
    version: u32,
    rule_name: Option<String>,
    source_message_id: Option<String>,
    archive_message_id: Option<i64>,
    subject: Option<String>,
    sent_at: Option<String>,
    activities: Vec<ActivityImport>,
}

pub fn start_email_order_watcher(
    data_root: String,
    activity_service: Arc<dyn ActivityServiceTrait + Send + Sync>,
) {
    tokio::spawn(async move {
        let inbox = PathBuf::from(&data_root).join("email-orders-inbox");
        let processed = PathBuf::from(&data_root).join("email-orders-processed");
        let failed = PathBuf::from(&data_root).join("email-orders-failed");

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            for dir in [&inbox, &processed, &failed] {
                if let Err(err) = tokio::fs::create_dir_all(dir).await {
                    warn!("Failed to create directory {:?}: {}", dir, err);
                }
            }

            let mut read_dir = match tokio::fs::read_dir(&inbox).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if !is_json(&path) {
                    continue;
                }

                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                info!("Email order watcher: processing {}", file_name);

                let content = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(err) => {
                        error!("Email order watcher: failed to read {:?}: {}", path, err);
                        move_file(&path, &failed.join(&file_name)).await;
                        continue;
                    }
                };

                let mut envelope: EmailOrderEnvelope = match serde_json::from_str(&content) {
                    Ok(envelope) => envelope,
                    Err(err) => {
                        error!(
                            "Email order watcher: failed to parse envelope {:?}: {}",
                            path, err
                        );
                        move_file(&path, &failed.join(&file_name)).await;
                        continue;
                    }
                };

                if envelope.version != 1 {
                    warn!(
                        "Email order watcher: unsupported envelope version {} for {}",
                        envelope.version, file_name
                    );
                    move_file(&path, &failed.join(&file_name)).await;
                    continue;
                }

                if envelope.activities.is_empty() {
                    warn!("Email order watcher: no activities found in {}", file_name);
                    move_file(&path, &failed.join(&file_name)).await;
                    continue;
                }

                decorate_activity_comments(&mut envelope);

                let validated = match activity_service
                    .check_activities_import(envelope.activities.clone())
                    .await
                {
                    Ok(validated) => validated,
                    Err(err) => {
                        error!(
                            "Email order watcher: backend validation failed for {}: {}",
                            file_name, err
                        );
                        move_file(&path, &failed.join(&file_name)).await;
                        continue;
                    }
                };

                if has_validation_errors(&validated) {
                    warn!(
                        "Email order watcher: validation errors found in {}, moving to failed",
                        file_name
                    );
                    move_file(&path, &failed.join(&file_name)).await;
                    continue;
                }

                match activity_service.import_activities(validated).await {
                    Ok(result) => {
                        info!(
                            "Email order watcher: imported {} activities from {} (duplicates: {})",
                            result.summary.imported, file_name, result.summary.duplicates
                        );
                        move_file(&path, &processed.join(&file_name)).await;
                    }
                    Err(err) => {
                        error!(
                            "Email order watcher: import failed for {}: {}",
                            file_name, err
                        );
                        move_file(&path, &failed.join(&file_name)).await;
                    }
                }
            }
        }
    });
}

fn decorate_activity_comments(envelope: &mut EmailOrderEnvelope) {
    let mut suffix_parts: Vec<String> = Vec::new();
    if let Some(rule_name) = envelope.rule_name.as_deref() {
        suffix_parts.push(format!("rule={}", rule_name));
    }
    if let Some(source_message_id) = envelope.source_message_id.as_deref() {
        suffix_parts.push(format!("sourceMessageId={}", source_message_id));
    }
    if let Some(archive_message_id) = envelope.archive_message_id {
        suffix_parts.push(format!("archiveMessageId={}", archive_message_id));
    }
    if let Some(sent_at) = envelope.sent_at.as_deref() {
        suffix_parts.push(format!("sentAt={}", sent_at));
    }
    if let Some(subject) = envelope.subject.as_deref() {
        suffix_parts.push(format!("subject={}", subject));
    }

    if suffix_parts.is_empty() {
        return;
    }

    let suffix = format!("Imported from email ({})", suffix_parts.join(", "));
    for activity in &mut envelope.activities {
        activity.comment = match activity.comment.take() {
            Some(comment) if !comment.trim().is_empty() => {
                Some(format!("{}\n\n{}", comment, suffix))
            }
            _ => Some(suffix.clone()),
        };
    }
}

fn has_validation_errors(activities: &[ActivityImport]) -> bool {
    activities.iter().any(|activity| {
        !activity.is_valid
            || activity
                .errors
                .as_ref()
                .is_some_and(|errors| !errors.is_empty())
    })
}
