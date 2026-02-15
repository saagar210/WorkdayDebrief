use crate::aggregation::AggregatedData;
use crate::commands::SummaryInput;

/// Build a prompt for the LLM based on aggregated data and tone
pub fn build_prompt(
    data: &AggregatedData,
    user_fields: &SummaryInput,
    tone: &str,
) -> String {
    let template = get_template(tone);

    // Build context from aggregated data
    let tickets_closed_count = data.tickets_closed.len();
    let tickets_closed_list = data
        .tickets_closed
        .iter()
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect::<Vec<_>>()
        .join(", ");

    let tickets_in_progress_count = data.tickets_in_progress.len();
    let tickets_in_progress_list = data
        .tickets_in_progress
        .iter()
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect::<Vec<_>>()
        .join(", ");

    let meetings_count = data.meetings.len();
    let meetings_list = data
        .meetings
        .iter()
        .map(|m| format!("{} ({}m)", m.title, m.duration_minutes))
        .collect::<Vec<_>>()
        .join(", ");

    let focus_hours = data.focus_hours;

    let blockers = user_fields.blockers.clone().unwrap_or_default();
    let priorities = user_fields.tomorrow_priorities.clone().unwrap_or_default();

    // Replace placeholders in template
    template
        .replace("{{tickets_closed_count}}", &tickets_closed_count.to_string())
        .replace("{{tickets_closed_list}}", &tickets_closed_list)
        .replace(
            "{{tickets_in_progress_count}}",
            &tickets_in_progress_count.to_string(),
        )
        .replace("{{tickets_in_progress_list}}", &tickets_in_progress_list)
        .replace("{{meetings_count}}", &meetings_count.to_string())
        .replace("{{meetings_list}}", &meetings_list)
        .replace("{{focus_hours}}", &format!("{:.1}", focus_hours))
        .replace("{{blockers}}", &blockers)
        .replace("{{tomorrow_priorities}}", &priorities)
}

/// Get prompt template by tone
pub fn get_template(tone: &str) -> String {
    match tone {
        "professional" => {
            r#"Generate a professional work summary in 4-6 sentences. Use third person perspective. Be factual and concise.

Input data:
- Tickets closed: {{tickets_closed_count}} ({{tickets_closed_list}})
- Tickets in progress: {{tickets_in_progress_count}} ({{tickets_in_progress_list}})
- Meetings attended: {{meetings_count}} ({{meetings_list}})
- Focus time: {{focus_hours}} hours
- Current blockers: {{blockers}}
- Tomorrow's priorities: {{tomorrow_priorities}}

Structure your summary as follows:
1. Accomplishments (tickets closed)
2. Current work (in-progress tickets)
3. Meetings/collaboration
4. Focus time
5. Blockers (if any)
6. Tomorrow's plan

Use formal, professional language. Do not use emojis. Be specific about ticket IDs and accomplishments."#
                .to_string()
        }
        "casual" => {
            r#"Write a casual, first-person summary of my workday in 4-6 sentences. Be conversational and highlight wins.

Today's work:
- Closed {{tickets_closed_count}} tickets: {{tickets_closed_list}}
- Still working on {{tickets_in_progress_count}} tickets: {{tickets_in_progress_list}}
- Attended {{meetings_count}} meetings: {{meetings_list}}
- Got {{focus_hours}} hours of focus time
- Blockers: {{blockers}}
- Tomorrow I'm planning: {{tomorrow_priorities}}

Write this like I'm telling a colleague what I did today. Use "I" and "my". Be upbeat about accomplishments. Keep it natural and conversational. Mention specific ticket IDs where relevant."#
                .to_string()
        }
        "detailed" => {
            r#"Write a comprehensive work summary in 6-8 sentences with specific details and time breakdowns.

Detailed input:
- Tickets completed ({{tickets_closed_count}}): {{tickets_closed_list}}
- Ongoing work ({{tickets_in_progress_count}}): {{tickets_in_progress_list}}
- Meetings ({{meetings_count}} total): {{meetings_list}}
- Focused work time: {{focus_hours}} hours
- Current blockers: {{blockers}}
- Planned for tomorrow: {{tomorrow_priorities}}

Provide a thorough summary that includes:
1. Specific ticket IDs and what was accomplished in each
2. Meeting topics and their durations
3. Time allocation breakdown
4. Detailed description of in-progress work
5. Specific blockers with context
6. Clear priorities for tomorrow

Use professional language. Include numbers, metrics, and specifics. This summary should give a complete picture of the day's work."#
                .to_string()
        }
        _ => get_template("professional"), // Default to professional
    }
}
