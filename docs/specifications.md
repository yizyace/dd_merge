# DD Merge — Sublime Merge Clone in Rust + GPUI

## Context

Build a high-performance, cross-platform Git GUI client modeled after Sublime Merge. The user has no C++ experience, so Rust + GPUI (Zed editor's GPU-accelerated UI framework) is the chosen stack. Performance and portability (macOS/Windows/Linux) are top priorities. The app should replicate Sublime Merge's core UX — commit graph, interactive staging, syntax-highlighted diffs, keyboard-driven workflow — while adding AI commit messages and a built-in 3-way merge conflict resolver. Plugin/extension system is explicitly excluded.

---

## Tech Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Language | **Rust** | C++-level performance, memory safety, no GC |
| UI Framework | **GPUI** (`gpui` crate) | GPU-accelerated, cross-platform (Metal/DX11/Vulkan), proven in Zed |
| UI Components | **gpui-component** | 60+ pre-built widgets (tabs, trees, docks, menus, virtual lists) |
| Git reads | **gitoxide** (`gix` crate) | Pure Rust, fast, no C dependency |
| Git writes | **git2** + `git` binary | Mature libgit2 bindings for index ops; shell to git for rebase/merge |
| Diff engine | **imara-diff** | Histogram algorithm, production-grade, handles pathological inputs |
| Syntax highlighting | **tree-sitter** | Already integrated in GPUI ecosystem, 40+ languages |
| File watching | **notify** crate | Detects external git changes (.git dir) |
| AI | **reqwest** + Anthropic/OpenAI APIs | Commit message generation from diffs |
| Background work | **crossbeam-channel** | Job queue for off-main-thread git ops |

---

## Feature Set

### From Sublime Merge (all core features)
- Multiple projects open as browser-like tabs
- Full commit history with visual commit graph (branch lines, merge edges, color-coded lanes)
- Sidebar: local branches, remote branches, tags, stashes, submodules
- Diff viewer with syntax highlighting (side-by-side when wide, inline otherwise)
- Character-level and word-level diff highlighting
- Interactive hunk context expansion (drag to show more lines)
- Line-by-line and hunk-level staging/unstaging
- Hunk splitting
- Commit editing, reordering
- Right-click context menus: checkout, revert, cherry-pick, reset (soft/mixed/hard), create branch/tag
- Instant search across commits (message, author, path, file contents)
- File blame/annotations
- Command palette (Cmd+Shift+P)
- Keyboard-driven workflow
- Git Flow support
- Merge and rebase operations
- Fetch, pull, push with credential handling
- Stash save/pop/apply/drop
- Dark and light themes
- Adaptable/resizable layout

### New Features (not in Sublime Merge)
- **AI-assisted commit messages** — LLM generates commit message suggestions from staged diffs
- **Built-in 3-way merge conflict resolver** — interactive ours/base/theirs view with per-hunk accept

### Excluded
- Plugin/extension system (no embedded scripting runtime)

---

## Project Structure

```
dd_merge/
  Cargo.toml                    # Workspace root
  assets/
    themes/                     # dark.json, light.json
    fonts/                      # Monospace font for diffs
  crates/
    dd_merge/src/main.rs        # Binary: bootstrap GPUI, open window
    dd_core/src/                # AppState, actions, settings, keybindings
      app_state.rs              # AppState (open repos, active tab)
      actions.rs                # All action structs
      settings.rs               # User preferences
      keymap.rs                 # Key binding definitions
    dd_git/src/                 # Git abstraction (read/write split)
      repository.rs             # GitRepository trait + real/fake impls
      status.rs                 # File status types
      commit.rs                 # Commit data types
      branch.rs                 # Branch/ref operations
      graph.rs                  # Commit graph layout algorithm
      diff.rs                   # Diff computation (wraps imara-diff)
      stash.rs                  # Stash operations
      remote.rs                 # Push/pull/fetch
      blame.rs                  # File annotations
      merge.rs                  # Merge/rebase support
      worker.rs                 # Background job queue
    dd_ui/src/                  # All GPUI views and components
      app_view.rs               # Root view
      tab_bar.rs                # Repo tabs
      repo_view.rs              # Main three-panel layout
      sidebar/                  # Branch, remote, tag, stash, submodule lists
      commit_list/              # Virtual-scrolled commit rows + graph painter
        graph_painter.rs        # Custom GPUI Element for graph lines
      diff_view/                # Syntax-highlighted diff (inline + side-by-side)
      staging/                  # File list, hunk staging, line staging
      commit_editor/            # Message input + AI suggest
      search/                   # Search bar + results
      command_palette.rs        # Fuzzy action filter overlay
      context_menus.rs          # Right-click menus
      blame_view.rs             # Per-line blame annotations
      merge_resolver/           # 3-way conflict resolution UI
      theme.rs                  # Theme definitions
    dd_ai/src/                  # AI commit message providers
      provider.rs               # AiProvider trait
      anthropic.rs              # Anthropic Claude implementation
      openai.rs                 # OpenAI GPT implementation
      prompt.rs                 # Prompt templates
  tests/fixtures/               # Test git repositories
```

### Crate Responsibilities

- **dd_merge** — Thin binary. Opens GPUI window, loads settings, registers actions.
- **dd_core** — `AppState` (open repos, active tab), `RepoState` (per-repo git data), action definitions, key bindings, settings. Depends on `dd_git`, NOT on GPUI.
- **dd_git** — `GitRepository` trait with dual backend: `gix` for reads (status, log, diff, blame), `git2` for writes (stage, commit, checkout), `git` binary for complex ops (rebase, interactive). Background `GitWorker` thread pool. File system watcher. **Zero GPUI dependency** — fully testable without GPU.
- **dd_ui** — All views: `AppView` → `TabBar` → `RepoView` → sidebar, commit list, diff view, staging, commit editor, search, command palette, context menus, blame, merge resolver. Depends on `dd_core` + `dd_git`.
- **dd_ai** — `AiProvider` trait with Anthropic/OpenAI implementations. Takes diff text, returns commit message. No GPUI or git dependency.

---

## Core Architecture

### Data Flow
```
User interaction → Action dispatched → Handler updates Entity<RepoState>
  → spawns GitJob on background thread → GitResult sent back via channel
  → Entity<RepoState> updated → cx.notify() → GPUI re-renders
```

### Key Data Structures

**CommitGraph** — topological commit ordering + column layout for branch visualization:
- `GraphEntry`: commit info + column position + color index + incoming/outgoing edges
- `GraphEdge`: from_column, to_column, color, edge type (branch vs merge)
- Layout algorithm: walk commits newest→oldest, track active lanes, assign columns, route merge edges

**FileDiff** — per-file diff with hunks:
- `Hunk`: old/new start+count, header, lines
- `DiffLine`: origin (+/-/context), old/new line numbers, content, word-level changes
- `WordChange`: byte offset range within a line for inline highlighting

**RepoState** — per-repo state:
- Git refs: branches, remotes, tags, stashes, HEAD
- Commit graph (full layout)
- Selected commit, current diff
- File statuses (staged vs unstaged)
- Staging state (selected files, hunks, lines)
- Search state, commit message, AI suggestion

### Git Integration: Read/Write Split

```
┌──────────────────────────────────────────────────┐
│                   dd_git crate                    │
├──────────────────────┬───────────────────────────┤
│   Read Path (gix)    │    Write Path (git2+cli)  │
│                      │                            │
│  status()            │  stage_paths()             │
│  walk_commits()      │  unstage_paths()           │
│  branches/tags()     │  stage_hunks/lines()       │
│  diff_workdir()      │  commit()                  │
│  blame_file()        │  checkout_branch()         │
│  search()            │  cherry_pick/revert()      │
│  read_file_at()      │  reset()                   │
│                      │  push/pull/fetch()         │
│                      │  rebase/merge()            │
├──────────────────────┴───────────────────────────┤
│  GitWorker (background thread pool)               │
│  FileWatcher (notify crate, watches .git/)        │
└──────────────────────────────────────────────────┘
```

### UI Component Tree
```
Window → Root → AppView
  ├── TabBar (browser-like tabs for open repos)
  └── RepoView (active tab)
      └── DockArea (resizable panels)
          ├── Left: Sidebar (branches, remotes, tags, stashes, submodules)
          └── Center: VerticalSplit
              ├── Top: CommitList (virtual-scrolled rows with graph lines)
              └── Bottom: DiffPanel
                  ├── FileList (changed files with status icons)
                  └── DiffView (syntax-highlighted, responsive layout)
  ├── CommitEditor (message input + AI suggest + commit button)
  └── CommandPalette (Cmd+Shift+P overlay)
```

---

## Phased Implementation

### Phase 0: Skeleton (Week 1-2)
- Init Cargo workspace with all 5 crates
- `main.rs` opens a GPUI window with gpui-component Root
- Simple `AppView` rendering "DD Merge" text
- `dd_git`: open a repo with `gix::discover()`, print branches to console
- **Goal**: Validate toolchain compiles and renders on macOS

### Phase 1: Read-Only Repo Viewer (Weeks 3-5)
- `RepoState` entity holding git data
- `GitWorker` background thread for git reads
- Sidebar with branch/tag/remote lists (gpui-component tree)
- Commit list with virtual scrolling (author, date, subject)
- Basic inline diff view for selected commit (raw +/- coloring, no syntax yet)
- File system watcher for external changes

### Phase 2: Commit Graph + Staging (Weeks 6-9)
- Graph layout algorithm (topological sort, column assignment, edge routing)
- Custom `GraphCell` GPUI Element painting lines and dots
- File status display (modified/added/deleted/untracked)
- Stage/unstage: files → hunks → individual lines
- Commit editor with message input and commit button
- **Hardest phase** — budget extra time

### Phase 3: Syntax Highlighting + Side-by-Side Diffs (Weeks 10-12)
- Tree-sitter integration for syntax-highlighted diffs
- Side-by-side diff view (responsive: switches to inline below ~1200px)
- Word-level diff highlighting within changed lines
- File list panel alongside diff view

### Phase 4: Tabs + Keyboard Navigation (Weeks 13-14)
- Tab bar for multiple open repos
- Open/close repo dialogs
- Command palette (Cmd+Shift+P) with fuzzy filtering
- Full keyboard navigation (j/k commits, s/u staging, Enter expand)
- Key binding customization

### Phase 5: Git Operations + Context Menus (Weeks 15-17)
- Right-click context menus on commits, branches, files
- Checkout, create/delete/rename branch
- Cherry-pick, revert, reset (soft/mixed/hard)
- Stash save/pop/apply/drop, tag create/delete
- Fetch, pull, push (with credential prompting)
- Merge and rebase initiation

### Phase 6: Search + Blame (Weeks 18-19)
- Search bar (message, author, path, file contents via git log -S/-G)
- Search results integrated into commit list
- File blame view (per-line annotations)
- Submodule listing

### Phase 7: AI Commit Messages (Weeks 20-21)
- `dd_ai` crate with Anthropic + OpenAI providers
- "Generate" button in commit editor
- Streaming response display
- Settings for API key and provider selection

### Phase 8: 3-Way Merge Conflict Resolver (Weeks 22-24)
- Conflict detection from git status
- Three-column view: ours / base / theirs
- Click to accept left/right/both per hunk
- Manual edit mode for complex conflicts
- Mark resolved and continue merge/rebase

### Phase 9: Themes + Polish (Weeks 25-26)
- Dark and light themes
- Layout persistence (panel sizes saved to disk)
- Error handling UI (toast notifications)
- Loading states (spinners for background ops)
- Windows and Linux testing/fixes

---

## Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `gpui` | 0.2 | UI framework (Metal/DX11/Vulkan) |
| `gpui-component` | 0.5 | Pre-built UI widgets |
| `gpui-component-assets` | 0.5 | Default icons (Lucide) |
| `gix` | 0.70+ | Pure Rust git reads |
| `git2` | 0.20 | C-backed git writes |
| `imara-diff` | 0.2 | Diff algorithm |
| `tree-sitter` | 0.24 | Syntax parsing |
| `tree-sitter-highlight` | 0.24 | Highlight spans |
| `notify` | 7 | File system watching |
| `crossbeam-channel` | 0.5 | Background job queue |
| `serde` / `serde_json` | 1 | Settings persistence |
| `reqwest` | 0.12 | HTTP client for AI APIs |
| `anyhow` / `thiserror` | latest | Error handling |
| `tokio` | 1 | Async runtime (for AI HTTP) |
| `parking_lot` | 0.12 | Fast mutexes |
| `insta` | latest | Snapshot testing (dev) |
| `tempfile` | latest | Test fixtures (dev) |

---

## Key Risks

| Risk | Mitigation |
|------|------------|
| GPUI pre-1.0 breaking changes | Pin exact version, isolate GPUI to `dd_ui` crate only |
| Custom Element API underdocumented | Study Zed source (`crates/editor/src/element.rs`) |
| `gix` missing features for writes | Use `git2` for all writes, `git` binary for complex ops |
| Large repo perf (100K+ commits) | Virtual scrolling, lazy graph computation, all git ops on background threads |
| Hunk staging correctness | Extensive test suite with real git repos, verify index after every stage op |
| Cross-platform build issues | Focus macOS first, test Windows/Linux in Phase 9 |

---

## Verification Plan

1. **Phase 0**: `cargo build` succeeds, window opens on macOS
2. **Phase 1**: Open a real repo, verify branches/commits display correctly
3. **Phase 2**: Verify graph lines match `git log --graph`; stage a hunk, run `git diff --cached` to confirm
4. **Phase 3**: Open a .rs file diff, verify syntax colors; resize window, confirm layout switch
5. **Phase 7**: Generate commit message from a real diff, verify quality
6. **Phase 8**: Create a merge conflict, resolve in UI, verify `git status` shows resolved
7. **All phases**: `cargo test` in each crate passes

---

## References

- [GPUI crate](https://crates.io/crates/gpui) / [gpui.rs](https://www.gpui.rs/)
- [gpui-component](https://github.com/longbridge/gpui-component) — 60+ pre-built components
- [Zed GPUI source](https://github.com/zed-industries/zed/tree/main/crates/gpui)
- [gitoxide](https://github.com/GitoxideLabs/gitoxide)
- [git2-rs](https://github.com/rust-lang/git2-rs)
- [imara-diff](https://github.com/pascalkuthe/imara-diff)
- [Sublime Merge](https://www.sublimemerge.com/) — reference application
- [Disassembling Sublime Text](https://thume.ca/2016/12/03/disassembling-sublime-text/) — architecture analysis
- [Sublime HQ: Hardware Accelerated Rendering](https://www.sublimetext.com/blog/articles/hardware-accelerated-rendering)
