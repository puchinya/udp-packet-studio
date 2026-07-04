# GitHub Actions リリース・署名・公証 設定ガイド

このドキュメントでは、GitHub Actions を使用して Windows および macOS 向けのバイナリをビルドし、macOS 向けバイナリ（`.dmg`）の署名と公証（Notarization）を行ってリリースするための設定手順を説明します。

本プロジェクトには、以下のリリース用ファイルが設定されています：
1. [.github/workflows/release.yml](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/.github/workflows/release.yml) — GitHub Actions のリリース用ワークフロー定義。
2. [scripts/build-macos.sh](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/scripts/build-macos.sh) — macOS アプリケーションのパッケージング、ディスクイメージ（.dmg）の作成、署名、公証、ステープルを行うスクリプト。
3. [scripts/entitlements.plist](file:///Users/nabeshimamasataka/RustroverProjects/udp-packet-studio/scripts/entitlements.plist) — macOS の Hardened Runtime（強化されたランタイム）に必要なセキュリティ権限設定。

---

## 🔑 必要な GitHub Actions Secrets

GitHub リポジトリの **Settings > Secrets and variables > Actions** から、以下のシークレットを設定する必要があります。

| シークレット名 | 説明 | 取得方法 / 形式 |
| :--- | :--- | :--- |
| `MACOS_CERTIFICATE` | Base64 エンコードされた `.p12` 形式の macOS コード署名証明書。 | [1. macOS 証明書の書き出し](#1-macos-証明書の書き出し) を参照。 |
| `MACOS_CERTIFICATE_PASSWORD` | 書き出した `.p12` 証明書のパスワード。 | 証明書のエクスポート時に設定したパスワード。 |
| `APPLE_ID` | Apple Developer アカウントの Apple ID（メールアドレス）。 | 例: `developer@example.com` |
| `APPLE_TEAM_ID` | 10桁の Apple Developer チーム ID。 | Apple Developer アカウントの Membership Details で確認可能。 |
| `APPLE_APP_SPECIFIC_PASSWORD` | 公証用に生成するアプリ専用パスワード。 | [appleid.apple.com](https://appleid.apple.com) で生成します。詳細は [2. 公証用資格情報の取得](#2-公証用資格情報の取得) を参照。 |

---

## 🛠️ 各種設定手順

### 1. macOS 証明書の書き出し

macOS アプリに署名するには、Apple の **Developer ID Application** 証明書が必要です。

1. Mac で **キーチェーンアクセス（Keychain Access）** を開きます。
2. 分類で **「自己の証明書（My Certificates）」** を選択し、対象の **Developer ID Application: Your Name (Team ID)** 証明書を見つけます。
3. 証明書を右クリックし、**「"Developer ID Application: ..." を書き出す」** を選択します。
4. ファイルフォーマットを `.p12`（個人情報交換）に指定し、パスワードを設定して保存します。
5. 保存した `.p12` ファイルを GitHub Secrets に登録できるように、Base64 でエンコードします：
   ```bash
   base64 -i your_certificate.p12 -o certificate_base64.txt
   # または、クリップボードに直接コピーする場合：
   base64 -i your_certificate.p12 | pbcopy
   ```
6. コピーした Base64 文字列を GitHub Secrets の **`MACOS_CERTIFICATE`** に登録します。
7. エクスポート時に指定したパスワードを **`MACOS_CERTIFICATE_PASSWORD`** に登録します。

### 2. 公証用資格情報の取得

公証（Notarization）は、Apple がアプリやディスクイメージに悪意あるコードが含まれていないことを確認する仕組みです。本ワークフローでは、`.dmg` ファイル自体の署名と公証、さらに公証チケットの埋め込み（Staple）を自動で行います。

1. [appleid.apple.com](https://appleid.apple.com) にログインします。
2. **「サインインとセキュリティ（Sign-In and Security）」 > 「App用パスワード（App-Specific Passwords）」** を選択します。
3. 新しいアプリ専用パスワードを生成し（例: "GitHub Actions Notarization" などの名前を設定）、表示された文字列（形式: `xxxx-xxxx-xxxx-xxxx`）をコピーします。
4. コピーしたパスワードを GitHub Secrets の **`APPLE_APP_SPECIFIC_PASSWORD`** に登録します。
5. [Apple Developer Portal](https://developer.apple.com/account/) にアクセスし、**チーム ID（Team ID）**（10桁の英数字、例: `A1B2C3D4E5`）を確認します。
6. チーム ID を GitHub Secrets の **`APPLE_TEAM_ID`** に登録します。
7. Apple ID アカウントのメールアドレスを **`APPLE_ID`** に登録します。

---

## 🚀 リリースの実行方法

リポジトリで `v` で始まるタグを作成してプッシュすると、GitHub Actions ワークフローが自動的にトリガーされます。

```bash
# バージョンタグを作成
git tag v0.1.0

# タグを GitHub へプッシュ
git push origin v0.1.0
```

ワークフローが完了すると、自動的に GitHub Releases に以下の成果物が添付されたドラフト/公開リリースが作成されます：
- `udp-packet-studio-windows.zip` (Windows 向け。`udp-packet-studio.exe`、`README.md`、`LICENSE.md` を含む)
- `udp-packet-studio.dmg` (macOS 向け。ドラッグ＆ドロップで `/Applications` にインストール可能な、署名・公証・ステープル済みのディスクイメージ)
