# HariboteBox 設計書 - ファイルシステム

**この文書について**: このファイルはHariboteBoxのファイルシステム仕様、永続化、複数Worker対応について記述しています。

**関連文書**:
- [プロジェクト概要](./overview.md) - 目的と技術スタック
- [UI設計](./ui-design.md) - ファイルマネージャUIの詳細
- [状態管理](./state-management.md) - 状態管理とイベント処理

---

## 1. ファイルシステム仕様

### 1.1 基本仕様

- ドライブ、ディレクトリは持たないフラット構造
- 各ファイルは以下を保持
  - ファイル名 (表示名)
  - ファイル内容 (`Uint8Array`)
  - `isInitialFile` フラグ (初期ファイルかどうか)
- ファイルアクセスは大文字小文字を区別しない
  - `ABC == abc == Abc`
  - 内部では canonical key (ファイル名の大文字化) で一意管理
  - 同一 canonical key の作成/リネーム/コピーは上書き扱い
- ファイル名の表示は元の大文字小文字を保持

### 1.2 ファイル名正規化ルール

- 入力値はパスの末尾要素 (Last Path Component) を対象とする
- 使用可能文字
  - ASCII 英数字 (`A-Z`, `a-z`, `0-9`)
  - ASCII 記号: `.` (ドット)、`_` (アンダースコア)、`!` (感嘆符)
  - 上記以外の ASCII 記号は `_` に変換
- 文字コード 127 超の文字が多数を占める場合は変換エラー
  - 現実装の判定: 非 ASCII 文字数が全体の過半数
- 連続する `_` は 1 つに圧縮
- 先頭/末尾の `.` は除去
- 長すぎる場合は最大 31 文字に切り詰め
  - 拡張子は可能な限り優先して保持
  - 拡張子が長すぎる場合はベース名を最低 8 文字残す

### 1.3 初期ファイルと永続化

**初期ファイルフラグ:**
- 各ファイルは内部的に `isInitialFile` フラグを保持
  - `true`: ビルド時に生成した初期ファイル群に由来するファイル
  - `false`: ドラッグアンドドロップで取り込んだ、または実行時に新規生成したファイル

**localStorage への保存ルール:**
- 永続化対象: `isInitialFile === false` のファイルのみ
  - 初期ファイルは localStorage に保存しない
  - ドラッグアンドドロップで取り込んだファイルのみ保存
  - 実行時に新規生成したファイルのみ保存
- ファイル作成・更新・削除・リネーム操作時に、対象ファイルが初期ファイルでない場合のみ localStorage に保存
  - キー: `haribote.fs.v1`
  - 保存内容: zlib による圧縮とBase64 エンコード
  - **圧縮形式:**
    - テキスト形式：`{ version: 1, files: [...] }` (JSON)
    - 圧縮：zlib で圧縮
    - エンコード：圧縮済みバイナリを base64 文字列に変換
    - localStorage に保存される値は base64 文字列のみ
  - **復元時の処理:**
    - base64 デコード → zlib 展開 → JSON パース
    - 復元後のファイル配列には `isInitialFile === false` のファイルのみ含まれる

**初期ファイル上書き処理:**
- 初期ファイルと同じ名前 (canonical key 一致) のファイルをドラッグアンドドロップまたは実行時生成した場合
  - 既存の初期ファイルを上書き
  - 上書き後、`isInitialFile` フラグを `false` に変更
  - その時点で新規ファイルとして localStorage に保存
  - 次回起動時に localStorage から復元され、初期ファイルを上書きした状態で起動

**初期ファイル削除の動作:**
- セッション内（起動中）に初期ファイルを削除した場合
  - メモリ上からファイルが削除される
  - 削除情報は localStorage に保存されない（初期ファイルは localStorage に保存されていないため）
  - ブラウザを再読み込みすると初期ファイルが復活する

**起動時の初期化処理:**
1. ビルド時に生成した初期ファイル群をメモリへ読み込む
   - すべてのファイルに `isInitialFile = true` フラグを設定
   - 初期ファイルの元データは `src/initial-fs` 配下に置く
   - ビルド時に生成スクリプトが `src/generated-initial-fs.ts` を作成し、アプリ側で読み込む
   - **圧縮処理:**
     - ビルド時：各ファイルを zlib で圧縮してから base64 エンコード（`scripts/generate-initial-fs.mjs` で処理）
     - 起動時：base64 デコード後に zlib で展開（`applyInitialFiles()` で処理）
     - 圧縮により初期ファイルシステムのバンドルサイズを削減
     - 起動時の展開処理は `pako` ライブラリで実施（既存の localStorage 圧縮と同一）
2. localStorage から永続化データを読み込み、初期ファイルシステムにマージする
   - localStorage キー `haribote.fs.v1` に保存データがある場合のみ実行
   - zlib 展開 → base64 デコード → JSON パース
   - マージ時の処理:
     - 保存されたファイル配列の各ファイルについて
     - 同じ名前 (canonical key 一致) の初期ファイルが存在する場合、初期ファイルを上書き
     - 初期ファイルに存在しないファイルは新規追加
     - マージ後のすべてのファイルについて、`isInitialFile` フラグを保持（保存データ側は `false`）

**デスクトップへの drag&drop:**
- ファイル取り込み時のサイズチェック
  - 既存ファイルのバイナリサイズ合計と取り込みデータのバイナリサイズ合計が `1.5MiB` (`1,572,864` bytes) を超える場合、エラーダイアログで表示して取り込みを中止
  - エラーメッセージはダイアログで日本語で表示
  - 既存ファイルシステムは変更しない
- 取り込み時
  - `File.webkitRelativePath` または `File.name` の末尾要素をファイル名に使用
  - 同名 (canonical key 一致) は上書き
  - 取り込まれたファイルは `isInitialFile = false` に設定
  - localStorage に保存される

### 1.4 ファイルシステム管理方針（複数 Worker 対応）

- **Main での一元管理**：
  - メイン UI スレッドでファイルシステム全体を管理
  - すべての作成・更新・削除操作を Main 経由で実施
  - localStorage への永続化も Main のみが行う
  - デスクトップへの drag&drop（ファイル取り込み）処理も Main で実施
    - サイズチェックが必要な場合は Main で判定し、エラー時は取り込みを中止
  
- **Worker でのアクセス**：
  - 起動時に Main から最新スナップショットを受け取る
  - ファイル読み込みはスナップショット内で実施
  - ファイル書き込みは Main へメッセージで通知
  - Main からの `updateFileSystemSnapshot` を受信したら内部状態を最新化
  
- **複数 Worker の同期**：
  - 任意の Worker が書き込みを実施
  - Main が永続化
  - Main が全 Worker に更新スナップショットを配信
  - 次のアクセスで全 Worker が最新ファイルシステムを参照可能

## 2. 複数 Worker ファイルシステム同期フロー

### 2.1 複数 Worker 同時実行時のファイルシステム同期

- **初期化時**：
  1. Worker A 起動 → Main から最新スナップショット (v0) を受け取る
  2. Worker B 起動 → Main から最新スナップショット (v0) を受け取る

- **ファイル書き込み時**：
  1. Worker A が `js_write_file("file.txt", data)` を呼び出す
  2. Main がファイル書き込みメッセージを受け取る
  3. Main が fileSystem を更新
  4. Main が `persistFileSystem()` で localStorage へ永続化
  5. Main が全アクティブ Worker（A, B）に `updateFileSystemSnapshot(v1)` メッセージを配信
  6. Worker B が スナップショット更新メッセージを受け取る
  7. Worker B が内部の fileSystem を v1 へ更新
  8. Worker B が次に `js_read_file_size("file.txt")` を呼び出す
  9. Worker B は Worker A の書き込み内容を正常に読み込める ✓

**複数 Worker が並行実行する場合**：
- 各 Worker は Main から受け取ったスナップショットをベースに動作
- Task A が `js_write_file` で新規ファイルを作成
- Main が永続化 → 全 Worker に `updateFileSystemSnapshot` を配信
- Task B が その後 `js_read_file_size` でファイルを検索 → 正常に見つかる
- ファイルの重複上書きは、Main の `fileSystem` Map で最後の write が優先
