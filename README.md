<div align="center">

<img src="docs/assets/creek.png" alt="Creek Logo">

# Creek: stream to doc, like it knows you.

**Real-time Collaborative Note-taking Agent Powered by Voice**

by [@rexera](https://github.com/rexera) & [@LinJHS](https://github.com/LinJHS), [Monticule Tech](https://github.com/MonticuleTech).

<!-- [![License](https://img.shields.io/github/license/LinJHS/creek?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green?style=flat-square)](package.json) -->

<!-- [**English**](README.md) | [**中文**](docs/README-zh.md) -->

</div>

---

Creek is a powerful, all-platform application that transforms your voice stream into structured markdown documents. 

Built with Rust and Vue 3, it provides a seamless "Talk-to-Doc" experience through intelligent voice processing, real-time collaboration, and context-aware editing.

## ✨ Features

- **Real-time Voice Processing**: ASR with built-in Voice Activity Detection for accurate transcription, NO manual work!
- **Stream-to-Doc**: Continuous document evolution with incremental builds and live streaming updates
- **Active Collaboration, Not Dictation**: The agent actively inspires you during streaming—suggesting ideas, filling knowledge gaps via web search and RAG, and structuring your thoughts as you speak
- **Intent-Aware Routing**: Paralleled, lightweight AI routers automatically determine user intent and fetch relevant context
- **RAG-Enhanced Assistance**: Semantic retrieval from conversation history and imported documents provides grounded, context-aware suggestions in real-time
- **Smart Todo Management**: Automatic task generation and maintenance based on document content and user speech
- **Dual Editing Modes**: Seamlessly switch between voice recording and manual Markdown editing
- **Git Version Control**: Automatic commit generation with AI-powered messages for every document change
- **Dynamic Focus Tracking**: Maintains current focus point and long-range consistency across editing sessions

## 💡 Motivation

Ever forgot to take notes during a project discussion? Tried audio transcription, only to find the output barely usable? 

We have vibe coding. Why not vibe writing?

**Existing pain points:**
- **Not real-time**: You must finish recording before seeing any output
- **Poor output quality**: Either verbose verbatim transcripts or AI summaries that miss context and make assumptions, requiring tedious manual fixes
- **Voice interaction bottlenecks**: Easy to get stuck, hard to maintain flow

Creek leverages modern **agentic engineering:**
- Parallel workflow pipeline with subagents
- Git-based version control for every change
- Multi-scale memory (short-term + current focus + RAG)
- Full Markdown support with live rendering

**The result?** A voice-stream-based real-time collaborative note-taking agent with incremental document building, active thinking assistance, and tool chain integration (web search, memory, knowledge). High-concurrency pipeline processing delivers ultra-low latency for the ultimate "Talk-to-Doc" experience.

**Perfect for:** Team meetings, project brainstorming, course lectures... Control progress in real-time, refine documents on the fly. Creek understands you and writes with you—"like it knows you."

## 🏗️ Architecture

Creek uses a sophisticated pipeline architecture built entirely in Rust for maximum performance:

```
--> [Audio Stream] --> [ASR Service]
    --> [Intent Router] (Parallel)
    --> [Tool Use, RAG Retrieval]
    --> [LLM Processing] (Parallel)
    --> [Document Update]
    --> [Git Commit]
    --> [Database Persistence]
    --> [Frontend Emit]
```

### Core Components

- **Intent Router**: Three parallel AI routers (Document Layer, RAG Layer, Tool Layer) determine user intent
- **State Manager**: Multi-level state management with SQLite persistence (current doc, focus, git history, todos)
- **RAG Service**: Semantic retrieval using fastembed-rs and LanceDB with 0.7 similarity threshold
- **Todo Agent**: Automatic task generation and maintenance driven by LLM analysis
- **Git Manager**: Version control with auto-generated commit messages using git2-rs
- **Document Service**: Incremental editing engine using SEARCH/REPLACE protocol inspired by Aider

## 🚀 Technology Stack

### Backend
- **Framework**: Tauri 2.0 (Rust)
- **LLM Integration**: Alibaba Cloud Qwen models (qwen-flash for routing, qwen3-30b for editing)
- **Vector Database**: LanceDB (Rust-native, zero-copy)
- **Embeddings**: fastembed-rs with BGE-Small-ZH-v1.5
- **Version Control**: git2-rs
- **Async Runtime**: Tokio

### Frontend
- **Framework**: Vue 3 (Composition API)
- **UI Library**: Bootstrap 5
- **State Management**: Pinia
- **Build Tool**: Vite
- **Markdown Rendering**: markdown-it
- **Diagram Support**: Mermaid

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/MonticuleTech/creek.git
cd creek

# Install dependencies
pnpm install

# Set up environment variables
cp .env.example .env
# Edit .env with your API credentials

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## ⚙️ Configuration

Create a `.env` file with your API credentials:

```env
OPENAI_API_KEY='sk-xxx'
BASE_URL='https://api.openai.com/v1'
```

> [!NOTE]
> Currently, the intelligence cores of Creek are `openai` compatible, yet for ASR module, only Alibaba `dashscope` ASR models are supported (note that it's a shared API key). We will expand the configuration options in the future, and welcome contributions!
> 
> Modify `src-tauri/src/services/asr_service.rs` to specify ASR model and URL.

## 🎯 Usage

### Quick Tour

1. **Create a Workspace**: Start by creating a project-based workspace to isolate your context
2. **Start Recording**: Click the microphone button to begin voice recording
3. **Talk Naturally**: Speak your thoughts - Creek will automatically structure them into documents; 
4. **Edit Flexibly**: Switch to manual editing mode anytime to refine the Markdown source
5. (Coming soon) **Import Resources**: Add PDF/Markdown files as reference materials for RAG-enhanced generation
6. **Track Progress**: View auto-generated todos and git history in real-time

### How to Interact with Creek? (Some Practices)

Creek is not just a passive dictation tool—it's an active collaborator. Here's how to get the most out of it:

**1. Speak Directly (Structure Your Thoughts)**
- "Let's outline three key pillars of agentic AI."
- "First, multi-scale memory: context grounding via short-term and long-term retrieval."
- Creek will automatically create headers, bullet points, and structure while you speak.

**2. Ask for Inspiration (Get Unstuck)**
- "What should I talk about next for the memory module?"
- "Can you suggest some challenges in multi-agent systems?"
- Creek actively provides suggestions, fills knowledge gaps via web search and RAG, and helps you continue the flow.

**3. Correct and Refine (Iterate Freely)**
- "Actually, scratch that last point. Let's call it 'Dynamic Context' instead."
- "Undo that."
- "Change the second paragraph to emphasize performance."
- Creek instantly updates the document, and every change is git-versioned for fearless creativity.

**4. Request Actions (Command Your Agent)**
- "Search for the latest research on retrieval-augmented generation."
- "Add a citation from my knowledge base."
- "Create a todo for implementing the search feature."
- Creek proactively executes tool use and keeps your workflow moving.


## 📊 Project Status

### Fully Implemented ✅
- Pipeline orchestration
- Intent Router (3 parallel routers)
- State Manager with SQLite persistence
- RAG Service with fastembed + LanceDB
- Todo Agent with LLM-based maintenance
- Git Manager with auto-commit
- Document Service with Aider SEARCH/REPLACE protocol
- ASR Service integration
- LLM Client with dual model strategy
- Tauri commands and events

### In Progress 🚧
- Frontend UI enhancements with SEXY animations
- Web search integration
- Memory pad, knowledge, ...
- Workspace file RAG
- Advanced context compression and long-document support
- MCP/Skills Marketplace Integration
- ...and more!

## 🤝 Contributing

**Creek is a work in progress, so any contribution is welcome!**

Please feel free to open an Issue or submit a Pull Request. This means a lot for us and the community!

## 📄 License

This project is licensed under the GPL-3.0 License - see [LICENSE](LICENSE) for details.
