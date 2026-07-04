# Rust GUI Development Best Practices Guide (`AGENTS.md`)

This document outlines the architectural standards, code patterns, and development guidelines for building high-performance, robust, and clean GUI applications in Rust, specifically using `egui` and `eframe`.

---

## 🏗 1. Architectural Standards

Immediate-mode GUIs (like `egui`) run their rendering loop at up to 60+ FPS. Blocking the UI thread for even a few milliseconds will result in visual stutter or application freezes.

### 1.1 Separation of Concerns: UI Thread vs. Worker Threads
- **UI Thread**: Strictly handles event processing, rendering, style application, and transient input state (e.g. text buffers).
- **Worker Threads**: Handle all heavy computations, synchronous disk I/O, and networking operations (such as UDP/TCP socket binding, sending, and receiving).

### 1.2 Thread Communication via Channels
Do **NOT** share heavy mutable states directly between threads using `Arc<Mutex<T>>` if those states are accessed frequently in the UI loop, as lock contention will freeze the UI. 
Instead, use message-passing via standard channels:
- Send instructions to the worker thread via a `Sender<Command>`.
- Read updates on the UI thread via a non-blocking `Receiver<Event>` using `try_recv()` at the start of every frame.

```mermaid
graph TD
    UI_Thread[UI Thread egui/eframe] -- Command channel.send --> Worker_Thread[Worker Thread]
    Worker_Thread -- Event channel.send --> UI_Thread
    Worker_Thread -- Blocking Network I/O --> Socket[UdpSocket / Disk]
```

---

## 🦀 2. Satisfying the Rust Borrow Checker in GUI State

Immediate-mode GUIs frequently nest closures (e.g., inside scroll areas, grids, panels). If your application state is monolithic, you will quickly encounter borrow-checker errors where a mutable borrow of `self` conflicts with another borrow of a subfield.

### 2.1 State Division (Wrapper Pattern)
If you are using layout-heavy components like `egui_dock::DockArea`, separate the layout state (`DockState`) from the application logic state. Wrap them in a master structure:

```rust
struct MainApp {
    dock_state: DockState<Tab>,
    state: UdpStudioState, // Application state is kept separately
}

impl eframe::App for MainApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Structuring this way allows borrowing dock_state and state mutably at the same time
        let mut viewer = MyTabViewer { state: &mut self.state };
        DockArea::new(&mut self.dock_state).show_inside(ui, &mut viewer);
    }
}
```

### 2.2 Deferred Mutation Pattern
When iterating over collections (e.g., drawing list items) and selecting/modifying elements, do **NOT** attempt to mutate the parent structure directly inside the iteration loop. Instead, capture actions in local variables and apply the changes *after* the borrow scope ends:

```rust
// ❌ BAD: Borrows self mutably during self.saved_packets iteration
for packet in &self.saved_packets {
    if ui.button("🚀").clicked() {
        self.send_packet(&packet.target, packet.payload_type, &packet.payload); // Compile Error!
    }
}

//  GOOD: Collects actions and executes them outside the immutable borrow
let mut send_trigger = None;
for packet in &self.saved_packets {
    if ui.button("🚀").clicked() {
        send_trigger = Some((packet.target.clone(), packet.payload_type, packet.payload.clone()));
    }
}
if let Some((target, payload_type, payload)) = send_trigger {
    self.send_packet(&target, payload_type, &payload); // Compiles perfectly!
}
```

---

## ⚡ 3. Performance & Resource Optimization

Immediate-mode rendering redraws components frequently. To keep CPU/GPU utilization low, follow these guidelines:

### 3.1 Lazy Repaint Wakeups
By default, `eframe` runs in a reactive loop, repainting only on user events. When background threads receive data, they must explicitly wake the event loop:
- Call `ctx.request_repaint()` in the UI thread event loop immediately when receiving a message from background channels.
- Keep read timeouts in worker threads short (e.g., 100ms) to ensure responsive shutdowns, but avoid hot-looping.

### 3.2 Debounced Saving (I/O Limiting)
- Do **NOT** serialize state to disk on every single keypress inside `text_edit_singleline`.
- Instead, trigger saves when the field has `.changed()`, or write to disk only when focus is lost or key buttons are clicked.

---

## 🎨 4. egui Styling & Modern Design Tokens (egui 0.34+)

To create premium desktop designs, avoid using browser-default aesthetics. Customize your layout using `egui`'s unified styling.

### 4.1 Unified Panels
- Avoid using deprecated `TopBottomPanel` and `SidePanel` directly on the context (`ctx`).
- Instead, use the unified `egui::Panel` struct:
  - `egui::Panel::top("id")`
  - `egui::Panel::bottom("id")`
- Render panels inside parent frames using `.show_inside(ui, |ui| ...)` to ensure correct bounds clipping.

### 4.2 Modern Style Fields
- **Corner Rounding**: `rounding` on `WidgetVisuals` is deprecated. Use `corner_radius` of type `CornerRadius` instead:
  ```rust
  visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
  ```
- **Window Corner Radius**: Setting window rounding via `window_rounding` is deprecated. Use `window_corner_radius` instead:
  ```rust
  visuals.window_corner_radius = egui::CornerRadius::same(8);
  ```
- **Context Styles**: Access styling via `ctx.global_style()` (not `ctx.style()`) and write styles back using `ctx.set_global_style(style)`.
- **Spacing**: Use integer dimensions for margins where required: `egui::Margin::same(12)` instead of float literals.

### 4.3 Monochrome/Text-Style Emojis (色なし絵文字の使用ルール)
To maintain visual consistency and support various OS/font configurations (especially when rendering with monochrome fonts like `Noto Sans Symbols 2`):
- **Do NOT use color emojis (default presentation) directly in the UI text.**
- **Use Monochrome/Text-style emojis instead.**
  - **CRITICAL**: Do **NOT** append the Variation Selector 15 (VS15, `\u{FE0E}`) or VS16 (`\u{FE0F}`) directly to emojis in Rust code or Fluent files. The `egui` text renderer does not automatically hide or zero-width render variation selectors, which causes a "tofu" block (□) to be rendered next to the emoji.
  - Rely on font fallback priority (`Noto Sans Symbols 2` and `FontAwesome`) configured in `styling.rs` to render emojis in monochrome automatically.
- **For emojis without monochrome glyphs:**
  - Replace them with clean monochrome text symbols (e.g., replace `🟢` and `🔴` with `●`, and replace `➕` with `+`).

### 4.4 Theme Support Guidelines (ライト・ダークテーマとシステム設定のガイドライン)
- **テーマ設定の種類**: アプリケーションは「システム設定 (System)」「ライト (Light)」「ダーク (Dark)」の3つのテーマ設定をサポートします。デフォルトは「システム設定」です。
- **システム設定時の挙動**: ユーザーが「システム設定」を選択している場合、OS側の外観モード設定（ライト/ダーク）の変更を検知し、アプリケーションのテーマを動的に追従させて切り替える必要があります。
- **テーマ設定の適用ルール**: 
  - フォントやテキストスタイルなどの共通設定は初期化時（`MainApp::new`）にのみ実行します。
  - カラーテーマ（Visuals）の設定のみを `styling::apply_theme(ctx, theme)` として分離し、設定変更時やシステムテーマ変更時に動的に再適用できるようにします。
  - `MainApp` 構造体内に `last_applied_theme: Option<AppTheme>` フィールドを保持し、毎フレームの監視処理における不要なテーマ再適用処理（Visualsのオーバーヘッド）を防止します。
- **ライトテーマの配色方針**:
  - `egui::Visuals::light()` をベースとしつつ、ダークテーマのトーンと調和するプレミアムなスレートライト（`#F5F7FA` 付近）の配色とします。
  - 選択やフォーカスのアクセントカラー（インディゴ系 `#4F6EF2` など）は、視認性を維持しつつダークテーマと一貫性を持たせます。

---



## 📂 5. File Splitting & Code Organization

Putting all logic, state, networking, and rendering inside a single `main.rs` leads to massive, unmaintainable files. Partition the codebase into clean, dedicated modules:

### 5.1 Recommended Directory Layout
- **`src/main.rs`**: Application entry point, window management, wrapper state definition (`MainApp`), the main event dispatcher loop, and tab routing.
- **`src/udp_worker.rs`**: Handles raw background thread networking, sockets, timeouts, and channel messaging.
- **`src/types.rs`**: Houses common data structures (e.g. packet definitions, log entries) and shared utility helpers (e.g., hex parsing, hex dump generation).
- **`src/config.rs`**: Manages configuration loading and storage, saving/restoring packets and ports to/from local disk (`updexp_config.json`).
- **`src/styling.rs`**: Configures global visual theme variables, custom color tokens, rounded widgets, and spacing offsets.
- **`src/views/`**: Modulizes the UI layout and rendering code per tab panel.
  - `src/views/mod.rs`: Submodule registry.
  - `src/views/saved_packets.rs`: Rendering for the Preset list and Preset editor.
  - `src/views/sender.rs`: Rendering for the active Packet Composer.
  - `src/views/log_viewer.rs`: Rendering for the Packet Logs list and Wireshark Hex Inspector.
  - `src/views/listener_settings.rs`: Rendering for the Socket bind setups and binding notifications.

### 5.2 Implementation Guidelines for Modular Views
Keep rendering files clean and decoupled by separating view layouts. In `egui`, render views by extending the state type using `impl` blocks inside respective files:

```rust
// In src/views/sender.rs
use crate::types::UdpStudioState;

impl UdpStudioState {
    pub fn show_sender(&mut self, ui: &mut egui::Ui) {
        // UI rendering logic for Composer tab goes here...
    }
}
```

This keeps individual view scopes small and easy to navigate while maintaining a unified mutable application state context.

---

## 🧪 6. Testing Best Practices & Separation

To align with Rust's best practices and keep the main codebase modular and clean:
- **Unit Tests**: Place simple, low-level logic tests (e.g. data deserialization defaults, string formatting, parsing) in the corresponding module file or a nested `tests` submodule.
- **Integration Tests**: Place all high-level integration tests (especially GUI tests like simulating pointer/mouse interactions) in a dedicated `tests/` directory at the project root (e.g. `tests/gui_tests.rs`).
- **Binary/Library Splitting**: To allow integration tests to import the application modules, split the binary crate into a library crate (`src/lib.rs`) containing all the core UI, views, and worker logic, and a lightweight binary entrypoint (`src/main.rs`) that simply runs the library's main loop.
- **GUI変更時のテスト実行義務 (GUI Modification Verification)**:
  - レイアウトやコンポーネント（特に `src/views/` 内の各タブ画面やコントロールの配置）を変更した際は、**必ず** `cargo test` を実行し、GUI操作のエミュレーションテスト（リサイズ操作や、送信ボタンのクリックイベントテストなど）が壊れていないか確認してください。
  - GUI変更によってボタンの位置やID、あるいは座標データ等の更新が必要になった場合は、追従して `tests/gui_tests.rs` 側のテストコードも適切にアップデートする必要があります。

---

## 📄 7. サードパーティライセンスの管理 (Third-Party License Management)

ライブラリ依存関係（Cargo）以外の外部オープンソース資産（フォント、画像、音声、アイコンなど）を追加する際は、将来的にライセンス情報が不明瞭になるのを防ぐため、以下の手順を必ず厳守してください。

- **ライセンス管理ドキュメントの作成・更新**:
  - 新たなアセットを追加する際は、必ず [docs/third_party_licenses.md](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/docs/third_party_licenses.md)（または該当するライセンス管理用ドキュメント）に、対象ファイルのパス、入手元URL、著作権表記、および適用ライセンス（例: SIL OFL 1.1）を追記してください。必要に応じて、ライセンスの全文もドキュメント内に掲載してください。
- **メインライセンスの更新**:
  - プロジェクト全体のライセンスファイル（[LICENSE.md](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/LICENSE.md) 等）に「Third-Party Components（サードパーティ・コンポーネント）」セクションを追加または更新し、サードパーティライセンスドキュメントへのリンクと参照説明を追記してください。
- **UI (Aboutダイアログ) への反映**:
  - [docs/third_party_licenses.md](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/docs/third_party_licenses.md) に記載されている外部オープンソース資産（フォント、アセットなど）は、アプリ内の「About」ダイアログの「Third Party Licenses」タブにおいても、他ライブラリと同様のアコーディオン（collapsing）レイアウトで著作権表示やライセンス条件を含めてください。

---

## 🐙 8. Git & Version Control Practices

To ensure user control over the codebase and repository:
- **Do NOT commit (`git commit`) or push (`git push`) automatically.**
- All changes should be made to the local files. Notify the user of the modified files so they can inspect, commit, and push them manually.

---

## 📋 9. 要望整理とGitHub Issue自動作成の義務 (Requirements Refinement & Auto Issue Creation)

ユーザーから新機能の追加やバグ修正などの新たな開発タスクを依頼された際は、**実際のコード変更や実装を開始する前に、必ず以下のプロセスを自律的に実行してください。** ユーザーから個別の指示がない場合でも、この挙動をデフォルトとします。

- **要望の整理とタスクの切り出し**:
  - 依頼内容を分析し、修正対象のファイルや設計案、具体的な実装ToDoリスト（Markdownのチェックボックス形式）を整理します。
- **マイルストーン情報の読み込み**:
  - ローカルの `.agents/active_milestone.json` が存在する場合はその情報をロードし、記述されているアクティブなマイルストーン名およびマイルストーン番号（`number`）を取得します。
- **GitHub Issueの自動起票**:
  - GitHub MCPサーバーの `issue_write` ツールを使用し、整理した要件とToDoリストを本文に含めたIssueをGitHub上に自動で作成します。この際、取得したマイルストーン番号を必ず紐付けます。
- **Issue URLの提示**:
  - 起票完了後、作成されたGitHubのIssue URLをユーザーに提示し、タスクのスコープに合意を得たうえで、コードの編集（実装ステップ）へと移行します。
- **実装完了時のPull Request自動作成とIssue紐付け**:
  - コードの修正とテスト確認が完了した段階で、自動的に作業用ブランチ（例: `feature/...`）をプッシュし、GitHub MCPサーバーの `create_pull_request` ツールを使用してPRを自動で作成します。
  - PRを作成する際は、本文（body）の冒頭に **`Closes #<Issue番号>`** を記述し、対応するIssueとPRを自動で紐付けます（PRマージ時に自動で該当Issueがクローズされます）。

---

## 📸 10. GUI動作確認・スクリーンショット撮影時のマナー (GUI Verification & Screen Capture Best Practices)

GUI アプリケーション（`udp-packet-studio`）における実装後の動作検証は、ユーザーの作業の邪魔にならないようにするため、以下のルールを厳守してください。

- **エージェントによる自動起動およびフォーカス奪取の絶対禁止**:
  - 実機上でエージェントが自律的に `cargo run` を起動してフォーカスを奪う（`activate` や `frontmost` の設定）、あるいはスクリーンショットを強制撮影するスクリプトを走らせることは**厳禁**です。キー入力などの作業が妨げられるため、絶対に行わないでください。
- **動作確認の進め方**:
  - GUI レイアウトや外観、操作確認（特にウィンドウリサイズ時の折り返し挙動など）については、エージェント側でコードを正しく配置・修正したあと、**ユーザーに手動での起動と目視確認を依頼**してください。
  - テストコードの作成や `cargo test` による自動ロジックテストは通常通り行いますが、実際の画面表示のチェックは常にユーザーの操作と確認を優先してください。





