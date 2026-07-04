# AntigravityエージェントのMCPおよびマイルストーン運用手順

本ドキュメントでは、Google AntigravityエージェントによるGitHub連携（MCPサーバー）およびアクティブなマイルストーンの管理方法について記述します。

---

## 1. GitHub MCP サーバーのセットアップ

AntigravityエージェントにGitHub操作権限（IssueやPRの作成・クローズなど）を付与するため、Docker経由でGitHub MCPサーバーを起動します。

### 設定ファイル
プロジェクトのルートに以下のディレクトリおよびファイルを作成・配置します。
- パス: `.agents/mcp_config.json`

### 設定例 (`.agents/mcp_config.json`)
```json
{
  "mcpServers": {
    "github": {
      "command": "/usr/local/bin/docker",
      "args": [
        "run", "-i", "--rm",
        "-e", "GITHUB_PERSONAL_ACCESS_TOKEN",
        "ghcr.io/github/github-mcp-server"
      ],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "YOUR_GITHUB_PERSONAL_ACCESS_TOKEN"
      }
    }
  }
}
```

> [!IMPORTANT]
> **macOS環境における注意点**
> macOSのGUIアプリケーション（Antigravity 2.0など）から起動されるプロセスは、`/usr/local/bin` などのパスを引き継がない場合があります。そのため、`command` には単なる `"docker"` ではなく、必ず絶対パスの **`"/usr/local/bin/docker"`** を指定してください。

### 設定の適用手順
設定ファイルを更新・作成した後、Antigravityに反映させるには以下のいずれかを実行します。
1. **Antigravity 2.0 アプリケーションの再起動** (最も確実)
2. 左側サイドバーの **「Skills & Customizations」** (または「Settings」) から **「MCP / MCP Servers」** を選択し、**「Refresh」** ボタンを押す。

---

## 2. セキュリティとGit管理（機密情報の保護）

`.agents/mcp_config.json` にはGitHubの個人用アクセストークン（PAT）が直接記述されるため、**リモートリポジトリにコミット・プッシュしてはいけません。**

### 除外設定
`.gitignore` の末尾に以下の設定を追加し、`.agents` ディレクトリ以下のファイルをGit管理対象から完全に除外します。

```gitignore
# .gitignore の末尾に追加
/.agents/
```

### すでにGitの追跡対象になっている場合の解除
もし、すでに `.agents/` ディレクトリ内のファイルがGitの追跡対象（インデックス）に入ってしまっている場合は、以下のコマンドを実行してローカルファイルを残したままGitの追跡から除外します。

```bash
git rm --cached .agents/mcp_config.json
```

---

## 3. アクティブマイルストーンの管理とAIへの認識方法

GitHub MCPサーバーの提供ツールには「マイルストーンの自動取得・作成」機能が含まれていません。AIが自動的に現在開発中のマイルストーンを認識してIssueやPRを作成できるようにするため、ローカルファイル経由で情報を伝えます。

### 設定ファイル
- パス: `.agents/active_milestone.json`

### 設定例 (`.agents/active_milestone.json`)
```json
{
  "milestone": "v1.1.1",
  "number": 1
}
```
*   `milestone`: 該当するマイルストーンのタイトル名
*   `number`: GitHub上でのマイルストーンの固有番号（ID）

### 運用手順
1. GitHubのWeb UIなどから新しいマイルストーン（例: `v1.1.2`）を作成します。
2. 作成されたマイルストーンのURL（例: `.../milestone/3`）から、末尾の番号を確認します（この場合は `3`）。
3. `.agents/active_milestone.json` を更新します：
   ```json
   {
     "milestone": "v1.1.2",
     "number": 3
   }
   ```
4. この状態でAIアシスタントにIssueの作成などを依頼すると、AIは自動的にこのファイルをロードし、マイルストーン `v1.1.2` (番号 `3`) を指定した状態でIssueやPRの作成・管理を実行します。
