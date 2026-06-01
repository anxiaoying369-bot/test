# AutoCast AI - Project Instructions (GEMINI.md)

This file contains team-shared architecture, conventions, workflows, and guidance for the AutoCast AI project.

## Project Overview

AutoCast AI is a desktop control system for Douyin (TikTok China) content operations and data collection.
- **Backend:** Tauri 2 (Rust) for system operations, process scheduling, and local file management.
- **Frontend:** Vue 3 + Vite 6 + Tailwind CSS v4.
- **Automation/Scraping:** Python scripts using DrissionPage, Chrome CDP, and other libraries for browser automation and data extraction.

## Architecture & Design Patterns

### 1. Backend (Rust/Tauri)
- **Modularization:** Rust commands in `src-tauri/src` are split into submodules (e.g., `commands/`, `ffmpeg.rs`, `db.rs`). Each file should ideally stay under 500 lines.
- **Process Management:** Tauri manages long-running Python processes for scraping and live monitoring.
- **Data Storage:** Local-first approach using JSON/JSONL files and SQLite (via `rusqlite`). No external database dependency.

### 2. Frontend (Vue/TypeScript)
- **Composables:** Business logic is extracted into `src/composables/` (e.g., `useLiveMonitor.ts`, `useVideoStudio.ts`).
- **Atomic Components:** UI components are modularized. Large views are broken down into smaller components in subdirectories (e.g., `src/components/video-studio/`).
- **Types:** Shared type definitions are located in `src/types/`.

### 3. Automation (Python)
- **Communication:** Python scripts communicate with Rust via stdout using JSON/JSONL. Logs should be sent to stderr.
- **Browser Automation:** Primarily uses DrissionPage + Chrome CDP for account validation and interaction.
- **Submodules:** External tools like `DouyinComment` are integrated as git submodules in `scripts/`.

## Key Workflows

### Account Management
- Uses Chrome CDP to hook into an existing Chrome instance.
- Saves Cookies, LocalStorage, and metadata to `~/Library/Application Support/AutoCastAI/cookies/`.

### Live Monitoring
- Spawns a Python process per room (up to 10).
- Streams events (chat, gifts, etc.) as JSONL from Python stdout to Tauri events.

### AI Video Studio
- Manages video generation projects using AI providers (fal.ai, etc.).
- Uses a local FFmpeg runtime for video processing and concatenation.

## Conventions

- **Rust:** Use `tauri::command` for all frontend-invokable logic. Handle errors using `Result<T, String>`.
- **Frontend:** Use Tailwind CSS v4 for styling. Follow Vue 3 Composition API patterns.
- **Python:** Always include a `requirements.txt` in script directories. Prefer `python3` for execution.
- **Security:** NEVER commit `cookie.txt`, `meta.json`, or any files containing sensitive account information.

## Environment Setup

1. **Node.js & npm:** Required for frontend and Tauri CLI.
2. **Rust Toolchain:** Required for Tauri backend.
3. **Python 3:** Required for automation scripts.
4. **FFmpeg:** Built-in runtime in `src-tauri/ffmpeg-runtime/`.
5. **Chrome:** Standard Google Chrome installation for CDP automation.

---
*This file is managed by the team. Update it when significant architectural changes or new conventions are introduced.*
