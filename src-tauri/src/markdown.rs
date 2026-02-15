use crate::aggregation::{Meeting, Ticket};

pub struct SummaryMarkdownInput<'a> {
    pub date: &'a str,
    pub narrative: &'a str,
    pub tickets_closed: &'a [Ticket],
    pub tickets_in_progress: &'a [Ticket],
    pub meetings: &'a [Meeting],
    pub focus_hours: f32,
    pub blockers: &'a str,
    pub tomorrow_priorities: &'a str,
    pub manual_notes: &'a str,
}

/// Render SummaryResponse to markdown format
pub fn render_summary_to_markdown(input: SummaryMarkdownInput<'_>) -> String {
    let SummaryMarkdownInput {
        date,
        narrative,
        tickets_closed,
        tickets_in_progress,
        meetings,
        focus_hours,
        blockers,
        tomorrow_priorities,
        manual_notes,
    } = input;

    let mut sections = Vec::new();

    // Header
    sections.push(format!("# Work Summary — {}", date));
    sections.push(String::new());

    // Narrative
    sections.push("## Narrative".to_string());
    sections.push(if narrative.is_empty() {
        "(No narrative)".to_string()
    } else {
        narrative.to_string()
    });
    sections.push(String::new());

    // Tickets Closed
    if !tickets_closed.is_empty() {
        sections.push(format!("## Tickets Closed ({})", tickets_closed.len()));
        for ticket in tickets_closed {
            sections.push(format!(
                "- [{}]({}) - {}",
                ticket.id, ticket.url, ticket.title
            ));
        }
        sections.push(String::new());
    }

    // Tickets In Progress
    if !tickets_in_progress.is_empty() {
        sections.push(format!("## In Progress ({})", tickets_in_progress.len()));
        for ticket in tickets_in_progress {
            sections.push(format!(
                "- [{}]({}) - {}",
                ticket.id, ticket.url, ticket.title
            ));
        }
        sections.push(String::new());
    }

    // Meetings
    if !meetings.is_empty() {
        let total_minutes: i32 = meetings.iter().map(|m| m.duration_minutes).sum();
        sections.push(format!(
            "## Meetings ({}, {}m total)",
            meetings.len(),
            total_minutes
        ));
        for meeting in meetings {
            sections.push(format!(
                "- {} ({}m)",
                meeting.title, meeting.duration_minutes
            ));
        }
        sections.push(String::new());
    }

    // Focus Time
    if focus_hours > 0.0 {
        sections.push("## Focus Time".to_string());
        sections.push(format!("{:.1} hours", focus_hours));
        sections.push(String::new());
    }

    // Blockers
    if !blockers.is_empty() {
        sections.push("## Blockers".to_string());
        sections.push(blockers.to_string());
        sections.push(String::new());
    }

    // Tomorrow's Priorities
    if !tomorrow_priorities.is_empty() {
        sections.push("## Tomorrow's Priorities".to_string());
        sections.push(tomorrow_priorities.to_string());
        sections.push(String::new());
    }

    // Additional Notes
    if !manual_notes.is_empty() {
        sections.push("## Notes".to_string());
        sections.push(manual_notes.to_string());
        sections.push(String::new());
    }

    sections.join("\n")
}
