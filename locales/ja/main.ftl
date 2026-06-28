# Tab Titles
tabs-collections = 📁 コレクション
tabs-composer = 🚀 コンポーザー
tabs-logs = 📊 ログ
tabs-inspector = 🔍 インスペクター
tabs-multicast = 🌐 マルチキャスト

# Title Bar / Bind Controls
titlebar-bind-addr = バインド先:
titlebar-btn-stop = ⏹ 停止
titlebar-btn-bind = ▶ バインド
titlebar-status-active = ● 有効
titlebar-status-offline = ● オフライン
titlebar-preferences = 設定...
titlebar-about = 本アプリについて...
about-title = 本アプリについて
about-desc = UDPパケットの送受信、カスタムパケットの作成（Composer）、ECHONET Lite などのプロトコルデコードをリアルタイムに行う開発者向けのデスクトップツールです。
about-license-label = アプリケーションのライセンス (LICENSE.md):
about-show-oss = 🌐 オープンソースライセンスを表示
about-oss-title = サードパーティライセンス
about-oss-description = 本ソフトウェアは、以下のオープンソースライブラリおよびリソースを使用して構築されています：
about-back = ⬅ アプリ情報に戻る


# Status Bar
statusbar-bound = バインド中: { $addr }
statusbar-broadcast = 📣 ブロードキャスト有効
statusbar-not-bound = ソケット未バインド
statusbar-auto-save-enabled = 💾 自動保存: 有効 ({ $format })
statusbar-auto-save-disabled = 💾 自動保存: 無効

# Settings Dialog
settings-title = ⚙ 設定
settings-lang-section = 言語設定
settings-lang-label = 表示言語:
settings-lang-system = システム設定
settings-lang-ja = 日本語 (Japanese)
settings-lang-en = English
settings-auto-save-section = ログ自動保存設定
settings-auto-save-enable = ログ自動保存を有効にする
settings-auto-save-format = ログフォーマット:
settings-auto-save-dir = 保存先ディレクトリ:
settings-browse = 📁 選択...
settings-close = 閉じる
settings-reset = 設定を初期化
settings-reset-confirm-title = 初期化の確認
settings-reset-confirm-msg = 設定を初期化してもよろしいですか？初期化後にアプリは自動的に再起動します。
settings-ok = OK
settings-cancel = キャンセル

# Collections View
collections-new = + 新規作成
collections-new-tip = 空のコレクションを新規作成します
collections-import = 📥 インポート
collections-import-tip = YAMLファイルからコレクションをインポートします
collections-empty-list = コレクションがありません。「新規作成」をクリックして開始してください。
collections-unnamed-col = 名前のないコレクション
collections-del-col-tip = コレクションを削除
collections-exp-col-tip = コレクションをエクスポート (YAML)
collections-add-req-tip = リクエストを追加
collections-empty-col = コレクションは空です
collections-unnamed-req = 名前のないリクエスト
collections-del-req-tip = リクエストを削除
collections-send-tip = パケット送信
collections-invalid-payload-tip = ペイロードのフォーマットが正しくありません
collections-edit-title = 📝 リクエストの編集
collections-edit-name = 名前:
collections-edit-target = 送信先:
collections-edit-target-ip = 送信先アドレス:
collections-edit-target-port = 送信先ポート:
collections-edit-format = フォーマット:
collections-edit-payload = ペイロード:
collections-edit-hex-tip = 入力例: 10 81 00 01 または 10810001
collections-edit-invalid-payload = ⚠ ペイロードの形式が正しくありません: { $msg }
collections-edit-load = 📂 コンポーザーへ読み込み
collections-edit-send = 🚀 送信

# Tooltips and extra status info
statusbar-auto-save-tip = クリックで自動保存を切り替えます
statusbar-open-log-dir = 📁 ログフォルダを開く
statusbar-open-log-dir-tip = ログの自動保存先ディレクトリを開く
statusbar-logged-packets = 記録されたパケット: { $count } 件

# Collection actions and naming
collections-import-success = コレクションが { $path } から正常にインポートされました
collections-import-fail-parse = YAMLコレクションの解析に失敗しました: { $msg }
collections-import-fail-read = ファイルの読み込みに失敗しました: { $msg }
collections-export-success = コレクションを { $path } にエクスポートしました
collections-export-fail = コレクションのエクスポートに失敗しました: { $msg }
collections-created-name = コレクション { $idx }
collections-req-created-name = リクエスト { $idx }

# ECHONET Lite Helper
el-err-tid = トランザクションID (TID) は2バイト（4桁の16進数）である必要があります
el-err-seoj = 送信元オブジェクト (SEOJ) は3バイト（6桁の16進数）である必要があります
el-err-deoj = 送信先オブジェクト (DEOJ) は3バイト（6桁の16進数）である必要があります
el-err-epc = プロパティコード (EPC) は1バイト（2桁の16進数）である必要があります
el-err-edt-empty = Set/Write要求ではプロパティデータ (EDT) を空にすることはできません
el-err-edt-even = プロパティデータ (EDT) の16進文字数は偶数である必要があります

el-helper-checkbox = 💡 ECHONET Lite パケット作成補助
el-builder-title = 💡 ECHONET Lite フレームビルダー
el-label-tid = トランザクションID (TID):
el-label-seoj = 送信元オブジェクト (SEOJ):
el-label-deoj = 送信先オブジェクト (DEOJ):
el-deoj-preset-ac = 家庭用エアコン (013001)
el-deoj-preset-meter = スマート電力量メータ (028801)
el-deoj-preset-node = ノードプロファイルオブジェクト (0EF001)
el-deoj-preset-custom = カスタムオブジェクト...

el-label-esv = サービスコード (ESV):
el-esv-preset-get = Get (0x62 - プロパティ値読み出し要求)
el-esv-preset-setc = SetC (0x61 - プロパティ値書き込み要求・応答あり)
el-esv-preset-seti = SetI (0x60 - プロパティ値書き込み要求・応答なし)
el-esv-preset-inf = INF (0x73 - プロパティ値通知)

el-label-epc = プロパティコード (EPC):
el-epc-preset-status = 動作状態 (0x80)
el-epc-preset-mode = 運転モード (0xB0)
el-epc-preset-power = 瞬時パワフル電力 (0xE0)
el-epc-preset-custom = カスタムプロパティ...

el-label-edt = プロパティデータ (EDT, 16進数):
el-edt-on = ON (30)
el-edt-off = OFF (31)

el-btn-generate = ⚙ ECHONET Lite 16進データを生成して挿入
el-btn-add-epc = + プロパティを追加
el-err-prefix = ECHONET Lite ビルダーエラー: { $msg }

# Composer Tab
composer-dest-addr = 送信先アドレス:
composer-payload-format = ペイロード形式:
composer-format-text = テキスト (UTF-8)
composer-format-hex = 16進数 (スペース可)
composer-payload-content = ペイロード内容:
composer-invalid-payload = ⚠ ペイロードの形式が正しくありません: { $msg }
composer-btn-send = 🚀 送信
composer-start-listener-tip = ⚠ 最初にリスナーソケットを開始してください。

composer-save-title = 💾 リクエストをコレクションに保存
composer-save-name = 名前:
composer-save-collection = コレクション:
composer-save-no-collections = コレクションがありません。「保存」をクリックして作成してください。
composer-btn-save = 💾 保存
composer-save-default-col = マイリクエスト
composer-save-created-req = リクエスト { $idx }

composer-ip-preset-section = プリセット
composer-ip-preset-nif-bcast = NIF ブロードキャスト
composer-ip-history-section = 履歴

# Log Viewer Tab
log-btn-clear = 🗑 クリア
log-btn-save = 💾 ログ保存
log-btn-save-tip = 選択した形式でログをエクスポートします
log-checkbox-autoscroll = 自動スクロール
log-label-ip-filter = IPフィルター:
log-hdr-no = No.
log-hdr-time = 時間
log-hdr-type = タイプ
log-hdr-source-ip = 送信元IP
log-hdr-send-port = 送信元ポート
log-hdr-dest-ip = 送信先IP
log-hdr-recv-port = 送信先ポート
log-hdr-length = データ長
log-hdr-info = 情報 (プレビュー)
log-save-success = ログが正常に { $path } に保存されました
log-save-fail = ログの保存に失敗しました: { $msg }

# Inspector Panel
ins-label-timestamp = タイムスタンプ: { $ts }
ins-label-sent-to = 送信先:
ins-label-received-from = 送信元:
ins-label-event-target = イベント対象:
ins-label-error-target = エラー対象:
ins-label-size = データ長: { $len } B
ins-label-decode-as = プロトコル解析:
ins-proto-raw = 🔌 Raw (16進数)
ins-proto-ascii = 📝 テキスト (ASCII)
ins-proto-echonet = 💡 ECHONET Lite
ins-title-hex-dump = 16進数ダンプ表示:
ins-title-ascii-view = ASCIIテキスト表示（制御コード可視化付き）:
ins-title-echonet-decode = ECHONET Lite プロトコル解析:
ins-el-err-too-short = ⚠ パケット長が短すぎるため、有効な ECHONET Lite フレームではありません (最小 12 バイト)。
ins-el-warn-ehd1 = ⚠ EHD1 が 0x{ $val } です (ECHONET Lite では 0x10 を想定)
ins-el-label-ehd1 = EHD1 (ヘッダー1):
ins-el-label-ehd2 = EHD2 (ヘッダー2):
ins-el-format = フォーマット { $fmt }
ins-el-label-tid = トランザクションID (TID):
ins-el-label-seoj = 送信元オブジェクト (SEOJ):
ins-el-label-deoj = 送信先オブジェクト (DEOJ):
ins-el-label-esv = サービスコード (ESV):
ins-el-label-opc = プロパティ数 (OPC):
ins-el-title-props = 解析されたプロパティ:
ins-el-err-malformed = ⚠ 不正な ECHONET Lite プロパティ: パケットが途切れています。
ins-select-log-item = ログパネルからログアイテムを選択して詳細を解析します

# ECHONET Lite Decoded objects/values
ins-el-obj-unknown = 不明
ins-el-obj-controller = コントローラ
ins-el-obj-node = ノードプロファイル
ins-el-obj-ac = 家庭用エアコン
ins-el-obj-meter = スマート電力量メータ
ins-el-obj-custom = カスタム/不明なデバイス

ins-el-esv-seti = SetI (プロパティ値書き込み要求・応答不要)
ins-el-esv-setc = SetC (プロパティ値書き込み要求・応答要)
ins-el-esv-get = Get (プロパティ値読み出し要求)
ins-el-esv-inf-req = INF_REQ (プロパティ値通知要求)
ins-el-esv-set-res = Set_Res (プロパティ値書き込み応答)
ins-el-esv-get-res = Get_Res (プロパティ値読み出し応答)
ins-el-esv-inf = INF (プロパティ値通知)
ins-el-esv-infc = INFC (プロパティ値通知応答)
ins-el-esv-seti-sna = SetI_SNA (プロパティ値書き込み不可応答・応答不要)
ins-el-esv-setc-sna = SetC_SNA (プロパティ値書き込み不可応答)
ins-el-esv-get-sna = Get_SNA (プロパティ値読み出し不可応答)
ins-el-esv-inf-sna = INF_SNA (プロパティ値通知不可応答)
ins-el-esv-unknown = 不明なサービス (0x{ $esv })

ins-el-epc-status = -> 動作状態
ins-el-epc-location = -> 設置場所
ins-el-epc-version = -> 規格Version情報
ins-el-epc-id = -> 識別番号
ins-el-epc-fault = -> 異常発生状態
ins-el-epc-manufacturer = -> メーカーコード
ins-el-epc-mode = -> 運転モード
ins-el-epc-temp = -> 設定温度
ins-el-epc-temp-cool = -> 冷房設定温度
ins-el-epc-node-instances = -> 自ノードインスタンスリスト
ins-el-epc-node-classes = -> 自ノードクラスリスト

ins-el-edt-empty = 空
ins-el-edt-on = ON (0x30)
ins-el-edt-off = OFF (0x31)
ins-el-edt-fault = 異常発生 (0x41)
ins-el-edt-normal = 正常 (0x42)
ins-el-edt-auto = 自動 (0x41)
ins-el-edt-cool = 冷房 (0x42)
ins-el-edt-heat = 暖房 (0x43)
ins-el-edt-dehumid = 除湿 (0x44)
ins-el-edt-circulator = 送風 (0x45)
ins-el-edt-unknown = 不明 (0x{ $val })
ins-el-edt-temp = { $temp } °C (0x{ $val })

# Multicast Panel
mc-status-offline = ⚠ リスナーオフライン:
mc-status-offline-tip = マルチキャストグループに参加する前に、タイトルバーでローカルポートにバインドする必要があります。
mc-join-title = 🌐 マルチキャストグループへの参加
mc-label-multicast-ip = マルチキャストIP:
mc-label-interface-ip = インターフェースIP:
mc-label-presets = クイックプリセット:
mc-preset-tip = { $ip } に参加
mc-btn-join = + マルチキャストグループに参加
mc-title-joined-list = 👥 現在参加中のグループ
mc-no-memberships = このソケットにアクティブなマルチキャストメンバーシップはありません。
mc-hdr-multicast-addr = グループ
mc-hdr-interface-addr = インターフェース
mc-btn-leave = 離脱
mc-err-empty-fields = マルチキャストアドレスとインターフェースアドレスは空にできません。

# Sockets Window / Dropdown
sockets-window-title = 🔌 ソケットマネージャー
sockets-btn-add = ➕ ソケット追加
sockets-lbl-name = 名前:
sockets-tooltip-delete = このソケットを削除する
sockets-navbar-btn-list = 🔌 ソケット...




