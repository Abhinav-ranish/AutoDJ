//! Gemini-powered track classification for genre, mood, vibe, and mixing notes.

use crate::llm::LlmClient;
use crate::{AiConfig, AiError};
use serde::{Deserialize, Serialize};

/// The result of AI-powered track classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackClassification {
    pub genre: String,
    pub subgenre: String,
    pub mood: String,
    pub vibe: String,
    pub mix_notes: String,
    pub ai_tags: Vec<String>,
}

/// Audio features extracted from the analysis pipeline, passed to the classifier.
#[derive(Debug, Serialize)]
pub struct ClassificationInput<'a> {
    pub artist: &'a str,
    pub title: &'a str,
    pub id3_genre: Option<&'a str>,
    pub bpm: f64,
    pub perceived_tempo: Option<f64>,
    pub groove_type: Option<&'a str>,
    pub key: &'a str,
    pub camelot: &'a str,
    pub energy_avg: f64,
    pub lufs: f64,
    pub spectral_centroid: Option<f64>,
    pub spectral_flatness: Option<f64>,
    pub bass_energy: Option<f64>,
    pub mid_energy: Option<f64>,
    pub high_energy: Option<f64>,
    pub vocal_presence: Option<f64>,
    pub rhythm_complexity: Option<f64>,
    pub danceability: Option<f64>,
    pub energy_trajectory: Option<&'a str>,
    pub duration_ms: u64,
    pub intro_length_ms: Option<u64>,
    pub outro_length_ms: Option<u64>,
}

const CLASSIFICATION_SYSTEM_PROMPT: &str = r#"You are a music classification AI. Given audio analysis features and metadata, classify the track.
Return valid JSON with these exact fields:
- genre: primary genre (house, hip-hop, techno, pop, r&b, drum-and-bass, dubstep, ambient, rock, latin, afrobeats, etc.)
- subgenre: specific subgenre (deep house, tech house, trap, lo-fi hip-hop, minimal techno, etc.)
- mood: emotional character (euphoric, melancholic, aggressive, chill, dark, uplifting, dreamy, energetic, sensual, mysterious)
- vibe: set role (anthem, filler, opener, closer, peak_time, warm_up, breakdown_tool, transition_tool)
- mix_notes: brief DJ mixing tips (e.g. "16-bar vocal intro, mix over 32 bars; avoid mixing during vocal hook at 1:30")
- ai_tags: array of 3-5 descriptive tags (e.g. ["vocal", "bass-heavy", "festival", "progressive"])"#;

/// Classify a track using the Gemini LLM based on its audio features and metadata.
pub async fn classify_track(
    input: &ClassificationInput<'_>,
    config: &AiConfig,
) -> Result<TrackClassification, AiError> {
    let llm = LlmClient::new();

    let context = serde_json::to_string(input)
        .map_err(|e| AiError::ParseError(format!("Failed to serialize input: {e}")))?;

    let response = llm
        .query(CLASSIFICATION_SYSTEM_PROMPT, &context, config)
        .await?;

    let classification: TrackClassification = serde_json::from_str(&response).map_err(|e| {
        AiError::ParseError(format!(
            "Failed to parse classification response: {e}\nRaw: {response}"
        ))
    })?;

    Ok(classification)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_deserialize() {
        let json = r#"{
            "genre": "house",
            "subgenre": "deep house",
            "mood": "chill",
            "vibe": "warm_up",
            "mix_notes": "Long 16-bar intro, safe to mix over 32 bars",
            "ai_tags": ["vocal", "bass-heavy", "summer"]
        }"#;
        let result: TrackClassification = serde_json::from_str(json).unwrap();
        assert_eq!(result.genre, "house");
        assert_eq!(result.subgenre, "deep house");
        assert_eq!(result.mood, "chill");
        assert_eq!(result.vibe, "warm_up");
        assert_eq!(result.ai_tags.len(), 3);
    }

    #[test]
    fn test_classification_input_serialize() {
        let input = ClassificationInput {
            artist: "Test Artist",
            title: "Test Track",
            id3_genre: Some("Electronic"),
            bpm: 124.0,
            perceived_tempo: Some(124.0),
            groove_type: Some("four_on_floor"),
            key: "C minor",
            camelot: "5A",
            energy_avg: 0.65,
            lufs: -8.5,
            spectral_centroid: Some(2500.0),
            spectral_flatness: Some(0.3),
            bass_energy: Some(0.4),
            mid_energy: Some(0.35),
            high_energy: Some(0.25),
            vocal_presence: Some(0.6),
            rhythm_complexity: Some(0.2),
            danceability: Some(0.85),
            energy_trajectory: Some("building"),
            duration_ms: 360000,
            intro_length_ms: Some(16000),
            outro_length_ms: Some(24000),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("Test Artist"));
        assert!(json.contains("124"));
    }
}
