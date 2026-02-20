use std::io::Cursor;

use vifei_core::event::ImportEvent;
use vifei_import::anthropic_messages::parse_anthropic_messages;
use vifei_import::cohere_translate::parse_cohere_translate;
use vifei_import::openai_responses::parse_openai_responses;

#[derive(Clone, Copy)]
enum AdapterCase {
    OpenAiSmall,
    OpenAiNoisy,
    AnthropicSmall,
    CohereSmall,
}

impl AdapterCase {
    fn all() -> [Self; 4] {
        [
            Self::OpenAiSmall,
            Self::OpenAiNoisy,
            Self::AnthropicSmall,
            Self::CohereSmall,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            Self::OpenAiSmall => "openai-small",
            Self::OpenAiNoisy => "openai-noisy",
            Self::AnthropicSmall => "anthropic-small",
            Self::CohereSmall => "cohere-small",
        }
    }
}

#[test]
fn adapter_replay_is_byte_stable_across_runs() {
    for case in AdapterCase::all() {
        let baseline = serialize_events(parse_case(case));
        for _ in 0..10 {
            let rerun = serialize_events(parse_case(case));
            assert_eq!(baseline, rerun, "drift detected for case {}", case.label());
        }
    }
}

#[test]
fn adapter_events_preserve_source_seq_and_shape_contract() {
    for case in AdapterCase::all() {
        let events = parse_case(case);
        assert!(
            !events.is_empty(),
            "expected non-empty fixture for {}",
            case.label()
        );
        for (idx, event) in events.iter().enumerate() {
            assert_eq!(
                event.source_seq,
                Some(idx as u64),
                "source_seq mismatch for {} at index {idx}",
                case.label()
            );
        }
        // Contract guard: importer must never materialize commit_index; this is
        // encoded by using ImportEvent here and serializing only import fields.
        let line = serde_json::to_string(&events[0]).expect("serialize event");
        assert!(
            !line.contains("commit_index"),
            "import event unexpectedly contained commit_index for {}",
            case.label()
        );
    }
}

fn parse_case(case: AdapterCase) -> Vec<ImportEvent> {
    match case {
        AdapterCase::OpenAiSmall => parse_openai_responses(Cursor::new(include_str!(
            "../../../fixtures/openai-responses-small.jsonl"
        ))),
        AdapterCase::OpenAiNoisy => parse_openai_responses(Cursor::new(include_str!(
            "../../../fixtures/openai-responses-noisy.jsonl"
        ))),
        AdapterCase::AnthropicSmall => parse_anthropic_messages(Cursor::new(include_str!(
            "../../../fixtures/anthropic-messages-small.jsonl"
        ))),
        AdapterCase::CohereSmall => parse_cohere_translate(Cursor::new(include_str!(
            "../../../fixtures/cohere-translate-small.jsonl"
        ))),
    }
}

fn serialize_events(events: Vec<ImportEvent>) -> String {
    events
        .iter()
        .map(|e| serde_json::to_string(e).expect("serialize import event"))
        .collect::<Vec<_>>()
        .join("\n")
}
