# Auto-DJ — MVP Roadmap

## Milestones

### A — Foundations (Week 1–2)
- [ ] Tauri app skeleton + Rust workspace with 5 crates
- [ ] SQLite storage layer with schema from `docs/07-storage.md`
- [ ] File import: drag-drop → hash → copy to originals/
- [ ] FFmpeg decode → canonical PCM cache
- [ ] Minimal cpal audio output: play a decoded PCM file
- **Done when**: User imports an MP3 and hears it play through cpal

### B — Two Decks + Mixing (Week 3–4)
- [ ] Two independent Deck structs with transport
- [ ] Crossfader + per-deck gain + master limiter
- [ ] 3-band EQ + HP/LP filter (biquad)
- [ ] UI controls driving engine via Command API
- **Done when**: User loads two tracks, adjusts EQ/filter, crossfades between them

### C — Sync + Quantization (Week 5)
- [ ] Master transport clock (BPM, beat, bar counters)
- [ ] SchedulePoint system with command heap in audio callback
- [ ] Tempo sync (match deck BPM to master)
- [ ] Quantised looping (N bars, aligned to beats)
- **Done when**: Loops are beat-aligned, commands execute on downbeats

### D — Analysis (Week 6–7)
- [ ] BPM detection (onset + autocorrelation)
- [ ] Beat tracking + downbeat detection
- [ ] Key detection (chromagram)
- [ ] Loudness, energy curve, section segmentation
- [ ] Waveform peak generation
- [ ] UI displays analysis results
- **Done when**: Imported tracks show BPM/key/energy/waveform

### E — Transitions (Week 8)
- [ ] Template: smooth_beatmix (16-bar bass swap)
- [ ] Template: filter_sweep
- [ ] Template: echo_drop
- [ ] TransitionExpander produces scheduled command sequences
- **Done when**: Three transitions execute correctly via Command API

### F — AI Conductor (Week 9–10)
- [ ] Heuristic scorer (tempo/key/energy/variety)
- [ ] AI API integration with JSON validation
- [ ] Manual / Assisted / AutoDJ mode switching
- [ ] Parameter ownership with timeout + resume
- **Done when**: AutoDJ mode runs a 30-minute set with AI and fallback

### G — Polish (Week 11–12)
- [ ] Offline mix export (render to file)
- [ ] Audio device switching
- [ ] Crash-safe logging
- [ ] CPU profiling + optimization pass
- **Done when**: App runs stable 60-minute sets, exports mix

---

## Test Plan

### Unit Tests
| Test | Crate | Description |
|------|-------|-------------|
| Quantization scheduling | autodj-core | Verify SchedulePoint resolves to correct sample positions |
| Command validation | autodj-core | Verify clamping, rejection of invalid commands |
| Ownership model | autodj-core | Verify user/AI/system ownership transitions + timeout |
| Candidate scoring | autodj-ai | Verify heuristic scorer produces expected rankings |
| PCM cache read/write | autodj-storage | Verify ADJP header parsing + sample data integrity |
| Beat grid generation | autodj-analysis | Verify synthetic grid from known BPM |

### Integration Tests
| Test | Description |
|------|-------------|
| Import→decode→play | Import file, decode to PCM, play through mock audio device |
| Transition execution | Execute a smooth_beatmix, verify command expansion + timing |
| AI JSON validation | Send mock AI responses, verify accept/reject behaviour |

### Audio Soak Test
- Load 10+ tracks, enable AutoDJ, run for 30+ minutes
- Monitor: no glitches (buffer underruns), no memory growth, no panics
- Log CPU usage per buffer callback (must stay < 3ms for 256 samples)

---

## Performance Budgets

| Metric | Budget | Measurement |
|--------|--------|-------------|
| Audio callback duration | < 3 ms (256 samples @ 48kHz) | Instrumented timer |
| State snapshot latency | < 5 ms UI → engine round-trip | Timestamp diff |
| Track decode time | < 10s for a 5-min MP3 | Wall clock |
| Analysis time | < 30s per track | Wall clock |
| Memory per loaded track | < 100 MB (5 min @ 48kHz stereo f32) | RSS measurement |
| UI frame rate | ≥ 30 fps | DevTools perf |
| Binary size | < 50 MB (excluding ffmpeg) | du -sh |
