# UDP Packet Studio

UDP Packet Studio は、Rust と `egui` / `eframe` を使用して構築された、高性能でモダンなデスクトップ向け UDP パケット送受信・解析スタジオです。ECHONET Lite プロトコルの解析補助やマルチキャスト送受信、Wireshark 風のヘキサデシマルインスペクター、ログ自動保存機能などを備えています。

---

![Screenshot](docs/images/screenshot.png)

---

## 🚀 主な機能

### 1. 📁 パケットコレクション管理 (Collections)
- よく使用するパケット定義（送信先 IP/ポート、データ形式、ペイロードなど）をコレクション（フォルダ）単位で管理・整理・保存できます。
- ワンクリックでコンポーザー（送信機）に反映させたり、ダイレクトに送信することができます。
- コレクションは YAML 形式でエクスポート・インポートが可能です。
- コレクション一覧は `collections.json` として独立して保存されます。

### 2. 🚀 パケットコンポーザー (Composer)
- 宛先アドレス（IP:Port）の指定、ペイロードの指定（Text / Hex 形式）をしてパケットを送信できます。
- **ECHONET Lite 送信支援ヘルパー**: ECHONET Lite プロトコルのヘッダー構造（TID, SEOJ, DEOJ, ESV, EPC, EDT）を UI から直感的に構築し、ワンクリックで送信パケットを生成できます。
- **MRA (Machine Readable Appendix) 連携**: ECHONET Lite の MRA 定義を読み込み、機器オブジェクトに応じたプロパティ(EPC)や、値(EDT)のドロップダウン入力候補を自動で解決・提示します。
- 送信したリクエストをコレクションに保存できます。

### 3. 📊 リアルタイム・ログモニター (Logs)
- 送受信（SENT / RECV）したすべてのパケット情報、およびシステムイベントをリアルタイムにリスト化します。
- **Wireshark 風パケットフィルター**: IP、ポート、パケットの送受信方向、ECHONET Lite の ESV・EPC などを指定した強力なクエリフィルタリング機能（シンタックスバリデーション、履歴保持、インタラクティブなヘルプ付き）を搭載しています。
- IP フィルター、オートスクロール、クリア機能を搭載しています。
- ログデータは手動で **CSV**、**JSON**、**PCAP (Wireshark 互換)** 形式にエクスポートできます。

### 4. 🔍 パケットインスペクター (Inspector)
- ログからパケットを選択し、詳細な内部バイナリ構造を調査できます。
- **Hex Dump**: Wireshark 風のオフセット・16進・ASCII 表示。
- **ASCII テキスト**: 文字列パケットのプレーン表示（制御コード可視化付き）。
- **ECHONET Lite デコーダー**: TID・すべてのサービスコード(ESV)・プロパティ（EPC/EDT）などを詳細にパース・解説表示します。

### 5. 🌐 マルチキャスト送受信 (Multicast)
- IPv4 / IPv6 のマルチキャストグループへの参加（Join）・離脱（Leave）とパケット受信ができます。
- クイックプリセット（ECHONET Lite 標準マルチキャストアドレス等）を搭載しています。

### 6. ⚙️ ログ自動保存 (Auto-Save)
- バックグラウンドのワーカースレッドが UI をブロックせず、パケットログをリアルタイムに自動保存します。
- 保存フォーマット：CSV / JSON (JSON Lines) / PCAP から選択可能。
- 自動保存ファイル名にはバインド開始時の時刻（時分秒）が付与されます。

### 7. 🌍 多言語対応
- 日本語・英語・システム設定に従った言語の自動切り替えに対応しています。

---

## 🎨 UI デザイン
- **テーマ切り替え (3タイプ)**: ダーク、ライト、および OS 設定に追従するシステム同期に対応。
- **Mac 風タイトルバー**: ドラッグ移動・ダブルクリック最大化・3 色丸ボタンによるウィンドウ制御。
- **レスポンシブ・ドッキング**: `egui_dock` によるタブ分割レイアウトで各パネルを自由に配置して同時監視可能。ドッキングレイアウトの永続化保存および初期状態へのリセットにも対応しています。
- **ユーザビリティの向上**: ウィンドウやドックスプリッターのドラッグ操作エリア（ヒットエリア）を拡大し、スムーズなリサイズが可能です。
- **ステータスバー**: ソケット状態（🟢 Active / 🔴 Offline）、バインドポート、パケット数、自動保存状態を常時表示。

---

## 🛠 ビルドおよび動作方法

### 動作環境
- **Rust**: Rust 2024 エディション以降

### ビルド & 実行

```bash
cargo run --release
```

### テスト実行

```bash
cargo test
```

### macOS App Store 向け PKG ビルド

```bash
cargo build --release
./scripts/build-macos-appstore.sh target/release/udp-packet-studio
```

---

## 📂 ディレクトリ構成

| パス | 説明 |
|---|---|
| `src/main.rs` | エントリーポイント（軽量な起動用ランチャー） |
| `src/lib.rs` | アプリ状態（`UdpStudioState`）とメインUIフレームワーク・描画制御 |
| `src/filter.rs` | パケットフィルターの文法定義とフィルタリングロジック |
| `src/mra.rs` | ECHONET Lite MRA (Machine Readable Appendix) の JSON 定義解析ロジック |
| `src/mra_defs.rs` | MRA 定義に関するデータ構造・型定義 |
| `src/config.rs` | 設定の永続化管理（`settings.json` / `collections.json`） |
| `src/types.rs` | パケットログ・ペイロード種別・PCAP・ECHONET Lite の共通型定義 |
| `src/udp_worker.rs` | バックグラウンドスレッドでの UDP Bind / Send / Recv |
| `src/styling.rs` | ダーク/ライトテーマおよびデザインシステム（フォント、マージン等）定義 |
| `src/locales.rs` | 多言語対応（日本語 / 英語） |
| `src/views/` | 各パネル（コレクション、インスペクター、ログ、マルチキャスト、コンポーザーなど）の UI 描画ロジック |
| `scripts/` | macOS 向けビルドスクリプト（DMG / App Store PKG） |
| `locales/` | 翻訳ファイル（Fluent 形式） |

### 設定ファイルの保存先（macOS）

```
~/Library/Application Support/udp-packet-studio/
├── settings.json      # 一般設定（IP/ポート・自動保存・言語など）
└── collections.json   # コレクション一覧
```

---

## 📄 ライセンス

本プロジェクトは [LICENSE.md](LICENSE.md) に基づいてライセンスされています。
