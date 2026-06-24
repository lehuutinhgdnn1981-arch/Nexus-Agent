# NEXUS — Phase 6: Frontend (Hoàn thành)

> React 18 + TypeScript + Tailwind + Zustand + Vite frontend. Dark modern UI, 3-cột layout (Sidebar | Chat | Tool Activity), streaming chat, tool timeline, approval dialog, session management.

---

## 1. Cấu trúc đã tạo

```
frontend/
├── package.json                    # React 18 + Vite 5 + Tailwind 3 + Zustand 4
├── tsconfig.json                   # TS strict mode + path aliases
├── tsconfig.node.json
├── vite.config.ts                  # Vite config + aliases (@, @bindings, @store, ...)
├── tailwind.config.ts              # NEXUS dark palette + animations
├── postcss.config.js
├── .eslintrc.cjs
├── .prettierrc
├── index.html                      # Dark theme + Inter/JetBrains Mono fonts
├── public/
│   └── nexus.svg                   # Logo
└── src/
    ├── main.tsx                    # React 18 createRoot
    ├── App.tsx                     # 3-cột layout + ApprovalLayer + hooks
    ├── index.css                   # Tailwind + custom prose styling + scrollbar
    ├── bindings/
    │   ├── types.ts                # Toàn bộ TypeScript types khớp Rust structs
    │   ├── ipc.ts                  # Wrapper quanh tauri invoke + listen
    │   └── index.ts                # Barrel
    ├── store/                      # Zustand stores (7 stores)
    │   ├── chatStore.ts            # Messages + streaming state + AgentEvent handler
    │   ├── sessionStore.ts         # Session CRUD + active session
    │   ├── toolStore.ts            # Tool activity timeline (last 100)
    │   ├── approvalStore.ts        # Pending approval requests
    │   ├── memoryStore.ts          # Long-term memory CRUD
    │   ├── schedulerStore.ts       # Scheduled jobs CRUD
    │   ├── configStore.ts          # App config load/patch
    │   └── index.ts                # Barrel
    ├── lib/
    │   └── format.ts               # date-fns formatters
    ├── hooks/
    │   ├── useAgentEvents.ts       # Subscribe all agent:* events → stores
    │   ├── useApprovalEvents.ts    # Subscribe approval:request
    │   ├── useSchedulerEvents.ts   # Subscribe scheduler:fired
    │   ├── useDebounce.ts
    │   └── useCopyToClipboard.ts
    ├── components/
    │   ├── layout/
    │   │   ├── Sidebar.tsx         # 72-width left panel
    │   │   ├── ChatPanel.tsx       # Center: header + messages + input
    │   │   └── ToolPanel.tsx       # 80-width right panel
    │   ├── chat/
    │   │   ├── ChatHeader.tsx
    │   │   ├── MessageList.tsx
    │   │   ├── MessageBubble.tsx
    │   │   ├── MessageInput.tsx    # Enter to send, Shift+Enter newline
    │   │   ├── MarkdownRenderer.tsx # react-markdown + GFM + highlight.js
    │   │   ├── ToolCallBlock.tsx   # Inline tool call display
    │   │   └── StreamingIndicator.tsx
    │   ├── sidebar/
    │   │   ├── SessionList.tsx
    │   │   ├── SessionItem.tsx     # Click to activate + rename/delete menu
    │   │   ├── NewSessionButton.tsx
    │   │   └── SessionSearch.tsx   # Debounced search
    │   ├── tools/
    │   │   ├── ToolTimelineItem.tsx # Expandable tool activity entry
    │   │   ├── ToolResultViewer.tsx (inline)
    │   │   └── ToolStatusBadge.tsx  (inline)
    │   ├── approval/
    │   │   ├── ApprovalLayer.tsx   # Mounts dialog + toast
    │   │   ├── ApprovalDialog.tsx  # Modal with Approve/Reject
    │   │   └── ApprovalToast.tsx   # "N more pending" indicator
    │   └── ui/                     # Primitive components
    │       ├── Button.tsx          # 5 variants × 4 sizes
    │       ├── Input.tsx           # Input + Textarea
    │       ├── Dialog.tsx          # Modal with ESC close
    │       ├── Badge.tsx           # 6 color variants
    │       ├── ScrollArea.tsx
    │       ├── Spinner.tsx
    │       └── index.ts
    ├── pages/                      # (Reserved for future routing)
    └── types/
        └── env.d.ts
```

---

## 2. Design system

### Palette (Tailwind)
- **nexus-950** (`#0f1115`) — background chính
- **nexus-900** (`#1f2933`) — sidebar / panels
- **nexus-800** (`#323f4b`) — borders / hover states
- **nexus-700** (`#3e4c59`) — borders input
- **nexus-100** (`#e4e7eb`) — primary text
- **accent-600** (`#4f46e5`) — primary actions (indigo)
- **success** (`#10b981`) — done state
- **warning** (`#f59e0b`) — requires_approval
- **danger** (`#ef4444`) — dangerous / errors
- **info** (`#3b82f6`) — running state

### Typography
- **Inter** (sans) — UI text
- **JetBrains Mono** (mono) — code, tool names, IDs

### Animations
- `animate-fade-in` — 200ms opacity
- `animate-slide-up` — 250ms translate Y
- `animate-slide-down` — 250ms translate Y (-8px → 0)
- `animate-pulse-soft` — 2s opacity pulse (for streaming/running indicators)

---

## 3. Layout

```
┌─────────────┬────────────────────────────┬────────────────┐
│  Sidebar    │      Chat Panel            │  Tool Panel    │
│  (288px)    │      (flex-1)              │  (320px)       │
│             │                            │                │
│ ┌─────────┐ │ ┌────────────────────────┐ │ ┌────────────┐ │
│ │ Logo +  │ │ │ Header: title + status │ │ │ Header +   │ │
│ │ sessions│ │ ├────────────────────────┤ │ │ stats      │ │
│ │ count   │ │ │                        │ │ ├────────────┤ │
│ ├─────────┤ │ │   Message bubbles      │ │ │ Timeline:  │ │
│ │ + New   │ │ │   (markdown + code     │ │ │ tool calls │ │
│ │ Session │ │ │    highlight + tool    │ │ │ (running / │ │
│ ├─────────┤ │ │    call blocks)        │ │ │  done /    │ │
│ │ Search  │ │ │                        │ │ │  error)    │ │
│ ├─────────┤ │ │                        │ │ │            │ │
│ │ Session │ │ │                        │ │ │ Click to   │ │
│ │ list    │ │ ├────────────────────────┤ │ │ expand I/O │ │
│ │ (scroll)│ │ │ Input: textarea + send │ │ └────────────┘ │
│ │         │ │ │ (Enter=send, S+Ent=nl) │ │                │
│ └─────────┘ │ └────────────────────────┘ │                │
│ v0.1.0      │                            │                │
└─────────────┴────────────────────────────┴────────────────┘
                  ↑ ApprovalLayer (modal overlay)
```

---

## 4. Features đã implement

### 4.1 Chat streaming
- Token-by-token streaming via `agent:delta` events → append to active assistant message
- "Thinking..." spinner khi assistant chưa trả token đầu
- Stop button khi đang streaming → gọi `chat_cancel`
- Cancellation token propagates từ frontend → Rust → agent loop

### 4.2 Markdown rendering
- `react-markdown` + `remark-gfm` (tables, strikethrough, task lists)
- `rehype-highlight` với `github-dark` theme
- Inline code highlight với accent-300 color
- Code blocks: rounded, bordered, horizontal scroll
- Tables: full-width, bordered, header dark
- Links: mở tab mới, accent color underline

### 4.3 Tool activity timeline
- Real-time updates qua `agent:tool_call` + `agent:tool_result` events
- Status badges: running (info, pulsing), done (success), error (danger)
- Click to expand: input JSON + output text (truncated 1500 chars)
- Duration in ms
- "Clear" button to wipe history
- Last 100 items kept

### 4.4 Inline tool call display
- Trong message bubble, hiển thị tool calls ngay sau assistant content
- Expandable: input + output (truncated 2000 chars)
- Status badge: running / done / error
- Monospace font cho tool name (accent-300)

### 4.5 Session management
- Sidebar list, sort by `updated_at DESC`
- Search với 200ms debounce
- New Session button → tạo session mới + set active
- Click session để activate
- Hover → "⋯" menu → Rename / Delete
- Rename dialog với Enter-to-save
- Delete confirmation dialog

### 4.6 Approval flow
- `approval:request` event → ApprovalDialog modal
- Hiển thị: tool name, permission badge, input JSON, session/run info
- Approve / Reject buttons (ESC = reject)
- Dangerous permission: title warning + danger button color + warning box
- Multiple pending: ApprovalToast "N more pending..."
- Respond qua `approval_respond` IPC command

### 4.7 Memory panel (components ready)
- `memoryStore` có `load/save/recall/remove`
- `MemoryPanel` / `MemoryItem` / `MemorySearchBar` reserved (sẽ mount trong settings modal ở Phase 7)

### 4.8 Scheduler panel (components ready)
- `schedulerStore` có `load/add/cancel`
- Reserved cho settings modal

### 4.9 Config management
- `configStore.load()` → `config_get` IPC
- `configStore.patch(json)` → `config_set` IPC
- Reserved cho SettingsModal

### 4.10 IPC wiring
- Tất cả 21 Tauri commands có TypeScript wrapper trong `bindings/ipc.ts`
- Type-safe qua `bindings/types.ts` (matches Rust structs)
- Event listeners subscribe 1 lần ở app root qua `useAgentEvents` + `useApprovalEvents`

---

## 5. UX details

- **Empty states**: "No sessions yet", "No messages yet", "No activity yet" với icons + helpful text
- **Loading states**: Spinners, pulsing badges, "Working..." buttons
- **Error display**: Inline error banners + toast cho IPC errors
- **Keyboard shortcuts**: Enter=send, Shift+Enter=newline, ESC=close dialog
- **Hover effects**: subtle bg color change + opacity transitions
- **Auto-scroll**: Message list auto-scrolls to bottom khi có new message/streaming
- **Dark mode**: mặc định `class="dark"` ở `<html>`, không toggle (sẽ thêm ở Phase 7 nếu cần)

---

## 6. TypeScript safety

- `strict: true` ở tsconfig
- `noUnusedLocals: true` + `noUnusedParameters: true`
- Path aliases: `@/*`, `@bindings/*`, `@store/*`, `@components/*`, `@lib/*`, `@hooks/*`
- Tất cả IPC wrappers typed với return types khớp Rust

---

## 7. Build & dev

```bash
cd frontend

# Install deps
npm install

# Dev mode (Vite hot reload)
npm run dev
# → http://localhost:5173

# Production build (Tauri sẽ pick up từ frontend/dist/)
npm run build

# Lint
npm run lint

# Format
npm run format
```

Tauri dev mode:
```bash
# Từ workspace root
cargo tauri dev
# → Tauri launch cửa sổ, load từ Vite dev server
```

Tauri production build:
```bash
cargo tauri build
# → Installers: src-tauri/target/release/bundle/{msi,dmg,AppImage}
```

---

## 8. Phase 6 completed.

**Chờ xác nhận để tiếp tục Phase 7 — Tests** (bổ sung integration tests cho agent loop với mock LLM, IPC contract tests, browser smoke tests, và final coverage report).
