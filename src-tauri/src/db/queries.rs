use sqlx::{Row, SqlitePool};

/// Upsert (INSERT OR REPLACE) a daily summary
/// Returns the summary ID
pub async fn upsert_summary(
    pool: &SqlitePool,
    date: &str,
    blockers: Option<&str>,
    tomorrow_priorities: Option<&str>,
    manual_notes: Option<&str>,
    narrative: Option<&str>,
    tone: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO daily_summaries (
            summary_date,
            blockers,
            tomorrow_priorities,
            manual_notes,
            narrative,
            tone
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(summary_date) DO UPDATE SET
            blockers = COALESCE(?2, blockers),
            tomorrow_priorities = COALESCE(?3, tomorrow_priorities),
            manual_notes = COALESCE(?4, manual_notes),
            narrative = COALESCE(?5, narrative),
            tone = COALESCE(?6, tone),
            updated_at = datetime('now')
        RETURNING id
        "#,
    )
    .bind(date)
    .bind(blockers)
    .bind(tomorrow_priorities)
    .bind(manual_notes)
    .bind(narrative)
    .bind(tone)
    .fetch_one(pool)
    .await?;

    let id: i64 = result.get("id");
    Ok(id)
}

/// Get a summary by date, returns full summary data
pub async fn get_summary_by_date(
    pool: &SqlitePool,
    date: &str,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            summary_date,
            tickets_closed,
            tickets_in_progress,
            meetings,
            focus_hours,
            blockers,
            tomorrow_priorities,
            manual_notes,
            narrative,
            tone,
            delivered_to,
            sources_status,
            created_at,
            updated_at
        FROM daily_summaries
        WHERE summary_date = ?
        "#,
    )
    .bind(date)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let id: i64 = r.get("id");
            let summary_date: String = r.get("summary_date");
            let tickets_closed: String = r.get("tickets_closed");
            let tickets_in_progress: String = r.get("tickets_in_progress");
            let meetings: String = r.get("meetings");
            let focus_hours: f64 = r.get("focus_hours");
            let blockers: String = r.get("blockers");
            let tomorrow_priorities: String = r.get("tomorrow_priorities");
            let manual_notes: String = r.get("manual_notes");
            let narrative: String = r.get("narrative");
            let tone: String = r.get("tone");
            let delivered_to: String = r.get("delivered_to");
            let sources_status: String = r.get("sources_status");
            let created_at: String = r.get("created_at");
            let updated_at: String = r.get("updated_at");

            let summary = serde_json::json!({
                "id": id,
                "summaryDate": summary_date,
                "ticketsClosed": serde_json::from_str::<serde_json::Value>(&tickets_closed).unwrap_or(serde_json::json!([])),
                "ticketsInProgress": serde_json::from_str::<serde_json::Value>(&tickets_in_progress).unwrap_or(serde_json::json!([])),
                "meetings": serde_json::from_str::<serde_json::Value>(&meetings).unwrap_or(serde_json::json!([])),
                "focusHours": focus_hours,
                "blockers": blockers,
                "tomorrowPriorities": tomorrow_priorities,
                "manualNotes": manual_notes,
                "narrative": narrative,
                "tone": tone,
                "deliveredTo": serde_json::from_str::<serde_json::Value>(&delivered_to).unwrap_or(serde_json::json!([])),
                "sourcesStatus": serde_json::from_str::<serde_json::Value>(&sources_status).unwrap_or(serde_json::json!({})),
                "createdAt": created_at,
                "updatedAt": updated_at,
            });
            Ok(Some(summary))
        }
        None => Ok(None),
    }
}

/// List summary metadata for the past N days
pub async fn list_summary_metas(
    pool: &SqlitePool,
    days_back: i32,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    // Validate days_back to prevent SQL injection and invalid dates
    if !(0..=3650).contains(&days_back) {
        return Err(sqlx::Error::Decode(
            format!("days_back must be between 0 and 3650, got {}", days_back).into(),
        ));
    }
    let date_param = format!("-{} days", days_back);
    let rows = sqlx::query(
        r#"
        SELECT
            id,
            summary_date,
            narrative,
            delivered_to
        FROM daily_summaries
        WHERE summary_date >= date('now', ?)
        ORDER BY summary_date DESC
        "#,
    )
    .bind(&date_param)
    .fetch_all(pool)
    .await?;

    let metas: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let id: i64 = r.get("id");
            let summary_date: String = r.get("summary_date");
            let narrative: String = r.get("narrative");
            let delivered_to_str: String = r.get("delivered_to");

            // Extract first 100 chars of narrative as snippet
            let narrative_snippet: String = narrative.chars().take(100).collect();
            let delivered_to: Vec<String> =
                serde_json::from_str(&delivered_to_str).unwrap_or_default();

            serde_json::json!({
                "id": id,
                "summaryDate": summary_date,
                "narrativeSnippet": narrative_snippet,
                "deliveredTo": delivered_to,
            })
        })
        .collect();

    Ok(metas)
}
