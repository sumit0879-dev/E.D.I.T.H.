# ⚡ E.D.I.T.H. — Stark-Grade Desktop AI Assistant
### *Even Dead, I'm The Hero*

<div align="center">

![Tauri 2](https://img.shields.io/badge/Tauri-v2.0-24C8D5?style=for-the-badge&logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-18-61DAFB?style=for-the-badge&logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5.7-3178C6?style=for-the-badge&logo=typescript&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-2021-DEA584?style=for-the-badge&logo=rust&logoColor=black)
![Tailwind CSS](https://img.shields.io/badge/Tailwind-3.4-38B2AC?style=for-the-badge&logo=tailwind-css&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-Bundled-003B57?style=for-the-badge&logo=sqlite&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)

<p align="center">
  <b>A hyper-advanced, tactical desktop AI assistant inspired by Stark Industries technology.</b><br/>
  Featuring multi-provider LLM streaming, neural speech synthesis & recognition, autonomous dev agents, system plugins, and hardware-grade credential encryption.
</p>

</div>

---

## 🌌 Overview

**E.D.I.T.H.** is a state-of-the-art desktop AI assistant built with **Tauri 2**, **Rust**, and **React 18 / TypeScript**. Designed with a futuristic tactical HUD and cybernetic aesthetics, E.D.I.T.H. seamlessly connects to cloud and local language models, automates PC actions, performs web intelligence recon, and maintains long-term memory while keeping all user data strictly local and secure.

---

## ✨ Key Features

### 🧠 Multi-Provider AI Inference
- **Cloud LLMs**: Native high-speed streaming via **Groq** (Llama 3.3 70B, etc.) and **Google Gemini** (Gemini 1.5/2.0 Flash & Pro).
- **Custom & Local Endpoints**: Connect any OpenAI-compatible API (**DeepSeek**, **Ollama**, **LM Studio**, **vLLM**, **Hugging Face**).
- **Auto-Discovery**: One-click dynamic model fetching directly from your custom endpoints.

### 🎙️ Tactical Voice & Speech Synthesis
- **Neural TTS**: High-quality cloud voice synthesis using **Microsoft Edge TTS**.
- **Offline Voice Engine**: Embedded support for **Kokoro ONNX** neural speech generation.
- **Voice Recognition**: Real-time browser-based speech recognition for hands-free voice commanding.

### 🛡️ Hardware-Grade Security & Privacy
- **Windows DPAPI Encryption**: All API keys and secrets stored in the local SQLite database are protected using Windows Data Protection API (DPAPI).
- **Zero Cloud Leakage**: Local-first architecture — your chat history, vector memories, and credentials never leave your machine.

### ⚡ System Plugins & OS Automation
- **Real-Time Web Recon**: Live web search powered by **Tavily AI Search**.
- **Application Launcher**: Launch installed desktop applications and system utilities via natural language.
- **Media & Audio Control**: Control system volume and automate media playback (YouTube, web media).
- **Screen Capture & HUD Diagnostics**: Capture screens and analyze visual telemetry on demand.
- **Terminal & Shell**: Execute tactical system commands safely.

### 🤖 Helix Autonomous Dev Agent
- Built-in developer workspace to interact with autonomous agents for code generation, file management, and project execution.

### 🎨 Stark Cybernetic HUD UI
- Holographic Arc Reactor visualizer with dynamic pulse animations.
- Tactical Top HUD with system telemetry (CPU load, memory usage, network status).
- Command palette, session switcher, markdown code syntax highlighting, and sleek glassmorphism themes.

---

## 🏗️ Architecture & Tech Stack

| Layer | Technologies |
| :--- | :--- |
| **Desktop Core** | [Tauri 2.0](https://tauri.app/) (Rust-based native windowing & IPC) |
| **Frontend HUD** | [React 18](https://react.dev/), [TypeScript](https://www.typescriptlang.org/), [Vite](https://vitejs.dev/) |
| **Styling & UI** | [Tailwind CSS](https://tailwindcss.com/), [Lucide React Icons](https://lucide.dev/), Custom Glassmorphism HUD |
| **Backend Engine** | [Rust 2021](https://www.rust-lang.org/) (Tokio async runtime, Reqwest, Rodio audio, Scraper) |
| **Database & Vector**| [SQLite](https://www.sqlite.org/) (Rusqlite bundled) + [LanceDB](https://lancedb.com/) Vector Embeddings |
| **Security** | Windows DPAPI (CryptProtectData / CryptUnprotectData) |

---

## 📁 Project Structure

```text
E.D.I.T.H/
├── .cargo/                 # Cargo configuration & environment
├── .vscode/                # VS Code workspace recommendations & launch configs
├── AI Engines/             # Voice & AI model storage (.gitkeep)
│   └── Kokoro/             # Kokoro ONNX model weights and voice banks
├── Llama/                  # Local Llama model storage (.gitkeep)
├── protoc/                 # Protobuf compiler tooling
├── src/                    # Frontend Application (React + TypeScript)
│   ├── components/         # HUD components (ArcReactor, TopHudBar, Sidebar, etc.)
│   ├── context/            # Global AppContext & State Management
│   ├── services/           # Tauri IPC communication layer
│   ├── types/              # TypeScript interfaces & data models
│   ├── views/              # View screens (Chat, DevAgent, MemoryBank, Plugins, Settings)
│   ├── App.tsx             # Root React view router
│   ├── index.css           # Futuristic cyber-HUD styling & animations
│   └── main.tsx            # React application entrypoint
├── src-tauri/              # Native Backend (Rust)
│   ├── capabilities/       # Tauri v2 window security capabilities
│   ├── icons/              # Application window & bundle icons
│   ├── src/
│   │   ├── agent.rs        # Autonomous Dev Agent orchestration
│   │   ├── chat.rs         # Chat streaming & multi-provider routing
│   │   ├── db.rs           # SQLite persistence & migration manager
│   │   ├── embedding.rs    # Vector embeddings & LanceDB connector
│   │   ├── lib.rs          # Tauri command registration & plugin setup
│   │   ├── llm.rs          # HTTP streaming client for LLM providers
│   │   ├── main.rs         # Rust main application entry
│   │   ├── memory.rs       # Memory bank & vector recall routines
│   │   ├── plugins.rs      # System plugins (Web Search, Apps, Volume, Terminal)
│   │   ├── providers.rs    # Custom model discovery & endpoints
│   │   ├── screen.rs       # Screen capture utility
│   │   ├── security.rs     # DPAPI vault & credential encryption
│   │   ├── tts.rs          # Microsoft Edge TTS & Kokoro audio synthesizer
│   │   ├── weather.rs      # Tactical meteorological recon
│   │   ├── window_manager.rs # Multi-window management
│   │   └── windows.rs      # Windows OS specific integrations
│   ├── Cargo.toml          # Rust dependencies & metadata
│   └── tauri.conf.json     # Tauri app configuration & permissions
├── .env.example            # Environment template
├── .gitignore              # Complete Git ignore specifications
├── package.json            # Node.js dependencies and scripts
├── tailwind.config.js      # Custom theme colors, animations & borders
├── tsconfig.json           # TypeScript compiler configuration
└── vite.config.ts          # Vite frontend bundler configuration
```

---

## 🚀 Getting Started

### Prerequisites

Ensure you have the following installed on your system:

1. **[Node.js](https://nodejs.org/)** (v18 or higher recommended)
2. **[Rust](https://www.rust-lang.org/tools/install)** (`rustup` with `msvc` toolchain on Windows)
3. **[C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)** (Visual Studio / MSVC build tools for Windows)

### 1. Clone the Repository

```bash
git clone https://github.com/YOUR_USERNAME/E.D.I.T.H.git
cd E.D.I.T.H
```

### 2. Install Frontend Dependencies

```bash
npm install
```

### 3. Run in Development Mode

Launch the live application with hot-reloading for both the React frontend and Rust backend:

```bash
npm run tauri dev
```

> **Note:** On the first launch, Rust will download and compile all backend crates, which may take a few minutes. Subsequent launches will be fast.

### 4. Build for Production

Compile the optimized standalone Windows executable and installer (.exe / .msi):

```bash
npm run tauri build
```

Compiled installation bundles will be placed in:
`src-tauri/target/release/bundle/`

---

## ⚙️ Configuration & API Keys

E.D.I.T.H. allows you to configure your API keys directly from the UI without touching configuration files:

1. Launch E.D.I.T.H.
2. Navigate to **Settings** (⚙️) from the left tactical navigation rail.
3. Under **AI Models / API Keys**, enter your desired provider keys:
   - **Groq API Key**: Get one from [Groq Console](https://console.groq.com/)
   - **Google Gemini API Key**: Get one from [Google AI Studio](https://aistudio.google.com/)
   - **Tavily Search API Key**: Get one from [Tavily AI](https://tavily.com/)
   - **Custom Providers**: Add your local Ollama (`http://localhost:11434/v1`) or DeepSeek endpoint.
4. Click **Save Settings**. All credentials are automatically encrypted using Windows DPAPI.

---

## 🔒 Security & Safe Uploads

- **No Hardcoded Secrets**: This repository does not contain any hardcoded API keys, personal secrets, or authentication tokens.
- **Database Protection**: Local `.db`, `.sqlite`, and `.env` files are strictly excluded via `.gitignore`.
- **Pre-configured .env.example**: Use `.env.example` as a reference if you need environment variables.

---

## 🤝 Contributing

Contributions are welcome! If you'd like to improve E.D.I.T.H. or add new tactical capabilities:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingTacticalFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingTacticalFeature'`)
4. Push to the branch (`git push origin feature/AmazingTacticalFeature`)
5. Open a Pull Request

---

## 📜 License

Distributed under the **MIT License**. See `LICENSE` for more information.

<div align="center">
  <sub>Built with ❤️ and Stark-Grade Precision.</sub>
</div>
