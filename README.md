# 🎧 AutoDJ

AutoDJ is a high-performance, AI-integrated DJ application built with **Rust** and **Tauri**. It combines a low-latency audio engine with an intelligent "AI Conductor" to assist with track selection and seamless transitions.

## ✨ Features

- **🚀 High Performance**: Core audio engine written in Rust for minimal latency.
- **🤖 AI Conductor**: Intelligent assistant for track suggestions and transition timing.
- **📦 Workspace Architecture**: Modular design for AI, Analysis, Core Engine, and Storage.
- **💻 Modern UI**: Sleek, responsive interface built with React and Vite.
- **🛠 Multi-Platform**: Native application experience via Tauri.

## 🏗 Project Architecture

The project is structured as a Rust workspace:

- **`autodj-app`**: The main Tauri application entry point and command handlers.
- **`autodj-core`**: The heart of the app—engine, mixer, decks, and transport logic.
- **`autodj-ai`**: AI Conductor logic and integration.
- **`autodj-analysis`**: Audio analysis (BPM, Key, Waveform generation).
- **`autodj-storage`**: Database management and track caching.
- **`ui`**: The React/TypeScript frontend.

## 🚀 Getting Started

### Prerequisites

- **Rust**: [Install via rustup](https://rustup.rs/)
- **Node.js & npm**: [Download Node.js](https://nodejs.org/)
- **Tauri Prerequisites**: Setup your system for Tauri (macOS/Windows/Linux) by following the [official guide](https://v2.tauri.app/start/prerequisites/).

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/JamPilot/AutoDJ.git
   cd AutoDJ
   ```

2. Install UI dependencies:
   ```bash
   cd ui
   npm install
   cd ..
   ```

3. Install the Tauri CLI globally:
   ```bash
   cargo install tauri-cli --version "^2.0.0"
   ```

4. Install FFmpeg (Required for decoding audio logic):
   ```bash
   brew install ffmpeg
   ```

### Running for Development

To start the application in development mode (hot-reloading for both UI and Rust), run from the **project root**:

```bash
cargo tauri dev
```

This will:
1. Start the Vite dev server for the React UI on `http://localhost:1420`
2. Compile and launch the Tauri/Rust backend
3. Watch all crate directories for changes

### Running Tests

To run the Rust test suite across all crates (from the project root):
```bash
cargo test --workspace
```

## 🛠 Building for Production

To create a production-optimized bundle (from the project root):

```bash
cargo tauri build
```

The resulting binaries will be located in `target/release/bundle/`.

## 📜 Documentation

Detailed architecture and specification documents are available in the `/docs` directory:
- [Overview](./docs/00-overview.md)
- [Architecture](./docs/01-architecture.md)
- [Audio Engine](./docs/02-audio-engine.md)

---
*Built with ❤️ by the AutoDJ team.*
