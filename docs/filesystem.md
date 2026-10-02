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
  - 起動時に Main からその時点のスナップショット（全ファイルのコピー）を受け取る
  - 以後は Main から変更分だけを `fileSystemChanged` で受け取り、手元のコピーに反映する
  - ファイル読み込みは手元のコピーから行う
  - ファイル書き込みは手元のコピーに反映し、Main へ `fileWritten` で通知する
  - 手元のコピーの操作は `src/fs/taskFileSystem.ts` にまとめてある

- **書き込みの成否は Worker が決める**：
  - `js_write_file` は同期呼び出しで、Main の応答を待てない。そのため Worker が手元のコピーで成否を判定し、その結果を Rust に返す
  - ファイル名は Main と同じ規則（`normalizeFileName`）で正規化する。正規化できない名前は Worker の時点で失敗にする
  - 書き込みモード（update / create / upsert）も Worker が手元のコピーで判定する
  - Main は `fileWritten` を受け取ったら、モードを再判定せずそのまま保存する。Worker が成功を返した書き込みが Main で捨てられることはない
  - 読み込みも同じ正規化を通す。正規化で名前が変わるファイル（例: `my file.txt` → `my_file.txt`）も、書いた名前のまま読める

- **複数 Worker の同期**：
  - 任意の Worker が書き込みを実施
  - Main が永続化
  - Main が全 Worker に変更分を配信する。書き込んだ Worker 自身にも配信する（下記の順序を保つため）
  - Main 起点の変更（ターミナルの `COPY` / `DEL` / `REN`、ドラッグアンドドロップ取り込み）でも、同様に全 Worker へ配信する
  - 次のアクセスで全 Worker が最新ファイルシステムを参照可能

- **変更の順序**：
  - 変更の順序は Main が決める。Main が処理した順に `fileSystemChanged` が全 Worker に届くので、各 Worker のコピーは Main と同じ状態に収束する
  - 例: Main で `DEL X` した直後に Worker が X を書いた場合、Main は「削除 → 書き込み」の順に処理し、Worker にも「削除」「書き込み」の順で届く。書いた本人に配信しないと、Worker だけ X が消えたままになる
  - 名前変更は「削除」と「書き込み」の 2 つの変更として 1 つのメッセージで届く

## 2. 複数 Worker ファイルシステム同期フロー

### 2.1 複数 Worker 同時実行時のファイルシステム同期

- **初期化時**：
  1. Worker A 起動 → Main からスナップショットを受け取る
  2. Worker B 起動 → Main からスナップショットを受け取る

- **ファイル書き込み時**：
  1. Worker A が `js_write_file("file.txt", data, mode)` を呼び出す
  2. Worker A が名前を正規化し、手元のコピーでモードを判定して書き込む。Rust には成否を返す
  3. Worker A が Main に `fileWritten` を送る
  4. Main が fileSystem を更新し、`persistFileSystem()` で localStorage へ永続化
  5. Main が全アクティブ Worker（A, B）に `fileSystemChanged`（`file.txt` の書き込み 1 件）を配信
  6. Worker B が変更を手元のコピーに反映する
  7. Worker B が次に `js_read_file("file.txt")` を呼び出す
  8. Worker B は Worker A の書き込み内容を正常に読み込める ✓

**複数 Worker が並行実行する場合**：
- 各 Worker は起動時のスナップショットと、その後の変更分をもとに動作
- Task A が `js_write_file` で新規ファイルを作成
- Main が永続化 → 全 Worker に `fileSystemChanged` を配信
- Task B が その後 `js_read_file` でファイルを検索 → 正常に見つかる
- ファイルの重複上書きは、Main が後から処理した write が優先
