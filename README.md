<div align="center">

<img src="docs/assets/creek.png" alt="Creek Logo" width="500">

# Creek

### *Let that stream, on the go.*

**Your Socratic companion for starting things.**

by [@rexera](https://github.com/rexera) & [@LinJHS](https://github.com/LinJHS), [Monticule Tech](https://github.com/MonticuleTech)

[![Discord](https://img.shields.io/badge/Discord-Join%20Us-5865F2?style=flat-square&logo=discord&logoColor=white)](https://discord.gg/PKrJtqTxgT)
[![License](https://img.shields.io/github/license/MonticuleTech/creek?style=flat-square)](LICENSE)

</div>

---

## What is Creek?

Creek is a voice-powered ideation companion. It doesn't transcribe your thoughts—it **challenges them**.

Through relentless Socratic questioning, Creek helps you break through the blank page and transform raw sparks into concrete, actionable artifacts: PRDs, research proposals, project plans, paper outlines—whatever you're trying to start.

**The philosophy**: Don't let anything stand between you and your next great idea. Capture inspiration whenever it strikes, refine it through dialogue, and walk away with something real.

## Why Creek?

Voice interfaces are broken. They're either:
- **Dumb transcribers** that vomit your words onto a page
- **Over-eager AI** that makes assumptions and hallucinates context

Creek takes a different approach:

- **Active, not passive**: We question, debate, and demand specifics
- **Cold start focused**: We help you *start*, not do your job
- **Human-in-the-loop**: You control the flow, we just push when you're stuck
- **On-the-go first**: Built for mobile, low-friction, zero friction

Think of Creek as a sparring partner for your brain—not a secretary.

## ✨ What It Does

**Socratic Dialogue Engine**
- Relentless questioning to crystallize vague ideas
- Challenges assumptions, demands specifics
- Maintains an evolving artifact throughout the conversation

**Intelligent Tool Use**
- Auto-invokes search, analysis, code generation when needed
- MCP/Skills marketplace for domain expertise
- No manual tool management—just think

**Artifact Generation**
- Living documents that evolve with your thinking
- PRDs, research proposals, project plans, reports
- Markdown-native, version-controlled, exportable

**Context Engineering**
- Smart context window management
- Multi-scale memory (short-term + RAG)
- Extreme context compression without losing meaning

## 🏗️ Architecture

```
[Voice Stream] 
    ↓
[ASR Service] (WebSocket)
    ↓
[Socratic Agent] ← Challenges & Questions
    ↓
[Intent Router] (Parallel: Document/RAG/Tool layers)
    ↓
[Tool Invocation] → Search/Analysis/Skills
    ↓
[Artifact Update] (SEARCH/REPLACE blocks)
    ↓
[Git Commit] + [State Persistence]
    ↓
[Frontend Sync]
```

Built in Rust with Tauri 2.0 for max performance. The entire pipeline—from voice to artifact—is sub-100ms latency.

### Core Components

| Component | Tech | Purpose |
|-----------|------|---------|
| **Socratic Agent** | Custom LLM pipeline | Relentless questioning engine |
| **Intent Router** | Parallel lightweight models | Intent classification (Document/RAG/Tool) |
| **Context Manager** | Compression + RAG | Smart context window optimization |
| **Artifact Service** | SEARCH/REPLACE protocol | Incremental document editing |
| **Skills Engine** | MCP standard | Plugin marketplace |
| **State Manager** | SQLite + Git | Persistence and version control |

## 🚀 Stack

**Backend (Rust)**
- Tauri 2.0, Tokio, LanceDB, fastembed-rs, git2-rs
- LLM: Qwen (routing) + Qwen-Coder (editing)

**Frontend (Vue 3)**
- Composition API, Pinia, Bootstrap 5, Vite
- markdown-it, Mermaid

## 📦 Quick Start

```bash
git clone https://github.com/MonticuleTech/creek.git
cd creek

pnpm install
cp .env.example .env
# Add your API keys

pnpm tauri dev     # Run dev
pnpm tauri build   # Build release
```

## ⚙️ Config

```env
OPENAI_API_KEY='sk-xxx'
BASE_URL='https://api.openai.com/v1'
```

> [!NOTE]
> Intelligence cores are OpenAI-compatible. ASR currently only supports Alibaba Dashscope (shared API key). We'll expand this—PRs welcome!

## 🎯 Usage

**1. Create a Workspace**
Start a project to isolate your ideation context.

**2. Start Talking**
Voice-first when walking, commuting, or pacing. Creek pushes back when you're vague.

**3. Get Challenged**
- You: "I want to build an app for... connecting people..."
- Creek: "What specific problem? Who has it? How painful?"

**4. Iterate**
Accept or reject suggestions. The artifact evolves with your thinking.

**5. Walk Away With Something**
When you're ready, export your artifact and go build it.

## 📊 Status

**Working Now**
- Voice pipeline + ASR integration
- Socratic questioning prototype
- Intent routing (3 parallel layers)
- RAG with LanceDB + fastembed
- Git auto-commit
- Workspace isolation

**Coming Soon**
- Full Socratic engine
- Context compression magic
- Skills marketplace
- MCP integration
- Token proxy infrastructure
- Mobile polish

## 🤝 Contributing

Creek is early. We'd love your help.

Issues, PRs, ideas—all welcome. Join the [Discord](https://discord.gg/PKrJtqTxgT) to chat.

## 📄 License

GPL-3.0

---

*Built with ❤️ by Monticule Tech*

*Let that stream, on the go.* 
