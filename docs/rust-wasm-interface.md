# HariboteBox 設計書 - Rust/Wasmインターフェース

**この文書について**: このファイルはRust/Wasmタスクとの連携インターフェース、メッセージプロトコル、キーボード入力処理について記述しています。

**関連文書**:
- [プロジェクト概要](./overview.md) - 目的と技術スタック
- [状態管理](./state-management.md) - 状態管理とイベント処理
- [ファイルシステム](./filesystem.md) - ファイルシステムと永続化

---

## 1. Rust タスク実装方針

**現在の実装状況:**
- Rust タスク (`run_task`) は主にテスト・デバッグ用のメッセージログとダミー Canvas 描画が中心
- Window API、Canvas 描画、ファイル I/O のインターフェースは動作確認済み
- println/print による端末への出力が正常に機能

**将来の拡張予定:**
- より複雑なアプリケーション実装（画像処理、音声処理、ゲーム等）
- インタラクティブな入力処理（キーボード・マウスイベント）
- Canvas への高頻度描画（アニメーション・ゲームループ）
- Worker 間通信や複数タスクの同時実行
- 外部 Rust クレート（image, audio 等）の統合

現在のインターフェース設計は、これらの将来拡張を想定した設計となっており、既存インターフェースの互換性を保ちつつ拡張可能な構造になっています。

## 2. Rust エントリポイント関数

### `run_task(file_name, command_line, title_bar_height)`
- wasm_bindgen でエクスポートされたメイン実行関数
- Worker から起動される正式なエントリポイント
- パラメータ:
  - `file_name: String`: 実行対象ファイルの名前
  - `command_line: String`: 完全なコマンドライン文字列
  - `title_bar_height: u32`: ウィンドウタイトルバーの高さ（ピクセル）
- Worker の起動時に自動的に呼び出される
- `title_bar_height` は UI の `TITLE_BAR_HEIGHT` 定数値（通常 32px）で、Rust アプリケーションがウィンドウレイアウト計算時に参照可能

## 3. Rust から利用するインターフェース

Rust 側は以下の JavaScript 関数を `#[wasm_bindgen(module = "env")]` 経由で呼び出します：

実装の詳細:
- `src/wasm/env.ts` が `env` モジュールとして提供される
- Worker (`rustTask.worker.ts`) が `createWasmEnv()` で実装関数を作成し、`globalThis.wasmEnv` へ登録
- `env.ts` の各関数が `globalThis.wasmEnv` を参照して実装へ委譲

**インターフェース更新時の注意:**
- Rust 側で新しいインターフェース関数を追加する場合（`lib.rs` の `#[wasm_bindgen(module = "env")]` に追加）、以下を同時に更新必須：
  - `rustTask.worker.ts` の `WasmEnv` 型定義に関数シグネチャを追加
  - `rustTask.worker.ts` の `createWasmEnv()` 内に実装を追加
  - `src/wasm/env.ts` にエクスポート関数を追加（委譲）
  - 本設計書にドキュメント記載

### 3.1 ウィンドウ管理インターフェース

- `js_open_window(width, height, title_ptr, title_len) -> u32`
  - Rust 側が指定したサイズ・タイトルでウィンドウを新規作成
  - Worker が Rust 数値ハンドルを採番し、戻り値として Rust 側へ返却
  - Worker 内で Rust 数値ハンドルと UI UUID を関連付ける
- `js_move_window(window_id, x, y)`
  - 指定ハンドルに対応するウィンドウの位置を変更
- `js_activate_window(window_id)`
  - 指定ハンドルに対応するウィンドウをアクティブ化
- `js_close_window(window_id)`
  - 指定ハンドルに対応するウィンドウを閉じる

### 3.2 描画インターフェース

- `js_draw_image(window_id, x, y, width, height, ptr, len)`
  - 指定ハンドルに対応するウィンドウの Canvas の指定矩形領域に RGBA imageData を描画
  - パラメータ:
    - `x`, `y`: 描画開始位置 (Canvas 内の座標)
    - `width`, `height`: 描画サイズ (ピクセル)

### 3.3 ターミナル出力インターフェース

- `js_print(text_ptr, text_len)`
  - Rust から Worker の `js_print` を呼び、Main の端末出力へテキストを追記する (改行なし)
  - 出力先端末は Worker 起動引数の `terminalWindowId` を使用する

### 3.4 ファイルI/Oインターフェース

- `js_read_file_size(filename_ptr, filename_len) -> i32`
  - 指定ファイル名のファイルサイズをバイトで返す
  - ファイルが存在しない場合は負値を返す
  - ファイル名照合は大文字小文字を区別しない
- `js_read_file_into(buf_ptr, buf_len) -> i32`
  - 前回の `js_read_file_size` で取得したファイル内容をバッファへ読み込む
  - 読み込んだバイト数を返す
- `js_write_file(filename_ptr, filename_len, data_ptr, data_len, mode) -> i32`
  - 指定ファイル名でファイル内容を書き込む
  - `mode` は以下の値：
    - `0`: update (存在する場合は上書き、存在しない場合はエラー)
    - `1`: create (存在しない場合は新規作成、存在する場合はエラー)
    - `2`: upsert (存在する場合は上書き、存在しない場合は新規作成)
  - 成功時は 0 を返す

### 3.5 イベントとタイマーインターフェース

- `js_get_keyboard_event(window_id) -> i32`
  - Worker のグローバルイベントキューから次のキーボードイベントをデキュー
  - 戻り値: イベントコード（文字キーの場合は ASCIIコード、キューが空の場合は -1）
  - `window_id` パラメータは受け付けるが、すべてのウィンドウが同一グローバルキューを共有するため、キュー特定には使用されない
  - 詳細は 5章 を参照
- `js_get_tick() -> f64`
  - Worker 初期化からの経過時間をミリ秒単位で返す
  - Worker 起動時に `performance.now()` を記録し、現在の `performance.now()` との差分を計算
  - 戻り値: 経過時間（ミリ秒、小数値）
  - 用途: Rust タスク内でタイマー・フレームスキップ・アニメーション制御等に使用
- `js_schedule_event(delay_ms, event_code)`
  - 指定時間後にイベントコードをイベントキューにエンキュー
  - `setTimeout` を使用して遅延実行を実現
  - パラメータ:
    - `delay_ms`: 遅延時間（ミリ秒）
    - `event_code`: エンキューするイベントコード（通常は ASCIIコードまたは特殊キーコード）
  - 用途: Rust タスク内でスケジュール化されたイベント（タイマーイベント等）を生成

## 4. Worker と Main のメッセージインターフェース

### 4.1 Main → Worker (Main が Worker へ送信するメッセージ)

- `startWithCommand`
  - Worker 起動時に Main から送信されるメッセージ
  - 起動引数：`terminalWindowId`、`fileName`、`commandLine`、`titleBarHeight`、`fileSystemSnapshot`、`environmentVariables`
  - `titleBarHeight`: UI の `TITLE_BAR_HEIGHT` 定数値（通常 32px）

- `keyboardEvent(eventType, key, code, keyCode, ctrlKey, shiftKey, altKey, metaKey, isAutoRepeat)`
  - Canvas ウィンドウがアクティブ時、キーボード入力イベントを Worker に転送
  - Main スレッドで Canvas ウィンドウの keydown/keyup イベントを捕捉し、アクティブウィンドウ判定後に転送
  - パラメータ:
    - `eventType`: 'keydown' | 'keyup' | 'keypress'
    - `key`: キーの文字表現 (e.g., 'a', 'Enter', 'ArrowUp')
    - `code`: キーボード位置コード (e.g., 'KeyA', 'Enter', 'ArrowUp')
    - `keyCode`: 数値キーコード（廃止予定だが互換性のため含める）
    - `ctrlKey`, `shiftKey`, `altKey`, `metaKey`: 修飾キー状態（boolean）
    - `isAutoRepeat`: キーホールド時の自動リピート状態（boolean）
  - Worker は受け取ったイベント情報をログ出力（将来：Rust FFI 経由で Rust タスクに転送予定）
  - Canvas ウィンドウ以外がアクティブの場合、転送されない

- `updateFileSystemSnapshot(fileSystemSnapshot)`
  - 複数 Worker 同時実行時、ファイルシステムが変更された際に Main から配信
  - Worker が `js_write_file` でファイルを書き込む
    ↓
  - Main がファイルシステムを更新して localStorage へ永続化
    ↓
  - Main が全アクティブ Worker に本メッセージを配信
  - Worker はスナップショットを受け取り、内部の fileSystem を更新
  - その後の `js_read_file_*` 呼び出しで、他の Worker の書き込み内容が読める

- `windowClose(windowId)`
  - Canvas ウィンドウのクローズボタン押下時、Main から Worker に通知
  - パラメータ:
    - `windowId`: クローズするウィンドウの UUID
  - Worker の処理フロー:
    1. `windowIdMap` から対応する数値ハンドルを逆検索
    2. 数値ハンドルと UUID のマッピングを削除（クリーンアップ）
    3. `windowIdMap.size === 0` （すべてのウィンドウが閉じられた）の判定
    4. 全ウィンドウ閉じられた場合、100ミリ秒のタイマーをセット
    5. タイマー経過後、再度 `windowIdMap.size === 0` を確認
    6. 依然としてウィンドウが存在しない場合、Worker は終了（`done` メッセージを Main に送信）
    7. タイマー待機中に新しいウィンドウが開かれた場合は、Worker は継続実行
  - 用途: Rust タスク内でウィンドウクローズイベントを検出し、適切にリソースをクリーンアップして終了する

### 4.2 Worker → Main (Worker が Main へ送信するメッセージ)

- `done`
  - Worker がタスク完了または終了する際に Main へ通知
  - パラメータなし
  - 送信契機:
    1. Rust タスクの main ルーチンが正常に終了（全ウィンドウが閉じられた後、2秒タイマーで確認）
    2. Rust タスクが処理フロー完了
  - Main の処理:
    - Worker スレッドを終了
    - Worker インスタンスをクリーンアップ
    - 関連する UI（ターミナルウィンドウ等）を適切に状態更新

- `error(message, stack)`
  - Worker でエラーが発生した際に Main へ通知
  - パラメータ:
    - `message`: エラーメッセージ文字列
    - `stack`: スタックトレース（オプション）
  - 発生契機:
    - WASM モジュール読み込み失敗
    - WASM ランタイム初期化失敗
    - Rust タスク実行エラー
    - 予期しない例外発生
  - Main の処理:
    - エラーをターミナルに出力
    - Worker を終了
    - ユーザーへのエラー通知

- `fileWritten(filename)`
  - Worker が `js_write_file` でファイルを書き込んだ際、Main へ通知
  - パラメータ:
    - `filename`: 書き込みされたファイル名
  - Main の処理:
    - ファイルシステムを更新して localStorage へ永続化
    - 複数 Worker 同時実行時は、全 Worker に `updateFileSystemSnapshot` メッセージで配信

- `println(windowId, text)`
  - Worker から Main へ改行付きテキスト出力要求を送る
  - Main は指定 `windowId` の端末画面へ `text` を 1 行追加する
  - `windowId` が端末として存在しない場合は無視する

- `print(windowId, text)`
  - Worker から Main へ改行なしテキスト出力要求を送る
  - Main は指定 `windowId` の端末画面の現在行末へ `text` を追記する
  - `windowId` が端末として存在しない場合は無視する

- `drawImage(windowId, x, y, width, height, pixels)`
  - Worker から Main へ Canvas への画像描画要求を送る
  - Worker が Rust の `js_draw_image` を通じて受け取った RGBA データを Canvas に描画
  - パラメータ:
    - `windowId`: Canvas ウィンドウの UUID
    - `x`, `y`: 描画開始位置 (Canvas 内の座標)
    - `width`: 画像幅 (ピクセル)
    - `height`: 画像高さ (ピクセル)
    - `pixels`: RGBA データ (Uint8Array、各ピクセルが RGBA で 4 バイト)
  - Main は指定ウィンドウの Canvas に `putImageData` で指定座標へ描画
  - `windowId` が Canvas ウィンドウとして存在しない場合は無視する

## 5. Canvas ウィンドウのキーボード入力処理

### 5.1 キーボード入力フロー

#### ユーザーが Canvas ウィンドウでキーを押した場合

1. **Main スレッド - キー入力イベント捕捉**：
   - `document.addEventListener('keydown', (event) => {...})`
   - すべてのキーボード keydown イベントを捕捉
   - `state.activeWindowId` でアクティブウィンドウ ID を取得

2. **Main スレッド - ウィンドウ型判定**：
   - `findWindowById(state.activeWindowId)` でウィンドウモデルを検索
   - ウィンドウの `kind` が `'canvas'` であるか確認

3. **Main スレッド - Worker 検索**：
   - `workerByWindowId.get(state.activeWindowId)` で対応する Worker を検索
   - Canvas ウィンドウが `js_open_window` FFI で生成された時に自動登録される
   - Worker が見つからない場合は処理を中断

4. **Main スレッド - キーボードイベント転送**：
   - 見つかった Worker に `keyboardEvent` メッセージを `postMessage` で送信
   - メッセージ形式:
     ```typescript
     {
       type: 'keyboardEvent',
       windowId: 'uuid-string',  // Canvas ウィンドウの UUID
       eventType: 'keydown',  // or 'keyup'
       key: 'a',              // キーの文字表現
       code: 'KeyA',          // キーボード位置コード
       keyCode: 65,           // 数値コード（互換性用）
       ctrlKey: false,
       shiftKey: false,
       altKey: false,
       metaKey: false,
       isAutoRepeat: false
     }
     ```
   - `event.preventDefault()` でブラウザのデフォルト動作を抑止
   - keydown と keyup の両イベントを送信

5. **Worker スレッド - キーボードイベント受信**：
   - `self.addEventListener('message', (event) => {...})`
   - `event.data.type === 'keyboardEvent'` で受信判定
   - `handleKeyboardEvent()` 関数を呼び出し

6. **Worker スレッド - イベント処理**：
   - `keydown` イベント時のみ処理（`keyup` は TBD）
   - イベント UUID をキーから対応する numeric window ID を逆引き
   - `key` プロパティが単一文字かつ ASCII 印字文字（0x20-0x7E）か判定
   - 有効な場合、文字の charCode（ASCIIコード）をイベントキューにエンキュー
   - コンソール出力でログ:
     ```
     [worker] Enqueued event code 97 (keydown - a) to window 1
     ```
   - 特殊キー（エンター、矢印キーなど）は現在スキップ

7. **将来の拡張**：
   - 特殊キーのコード割り当て（TBD）
   - `keyup` イベントの処理方法（TBD）
   - 修飾キー（Ctrl, Shift, Alt, Meta）の処理方法（TBD）

#### 修飾キーの例

- 単独キー: `key='a', code='KeyA'` → `"a"`
- Ctrl+A: `key='a', code='KeyA', ctrlKey=true` → `"Ctrl+a"`
- Shift+A: `key='A', code='KeyA', shiftKey=true` → `"Shift+A"`
- Ctrl+Shift+A: `ctrlKey=true, shiftKey=true` → `"Ctrl+Shift+A"`

#### 非 Canvas ウィンドウ時の挙動

- Terminal または FileManager がアクティブの場合、キーボードイベントは転送されない
- これらのウィンドウは独自のキーボードハンドラを持つため
- Canvas ウィンドウから他ウィンドウへの切り替え時、キーボード転送も自動的に停止

### 5.2 イベントキュー機能

#### イベントキューの概要

Worker は 1 つのグローバルなイベントキューを管理し、すべてのウィンドウが共有する。キーボードイベントを数値ベースで Rust タスク に提供する。

**ウィンドウのライフサイクル**:
- `js_open_window()` 呼び出し時に新規ウィンドウを登録（eventQueue は初期化されない）
- `js_close_window()` 呼び出し時にウィンドウを登録解除（eventQueue は変更されない）
- Worker 初期化時に eventQueue を空の配列で初期化（全ウィンドウで共有）

**イベントコード形式**:
- **文字キー**（`keydown` イベント）: ASCIIコード（0x20-0x7E）
  - 例: `'a'` → 97, `'A'` → 65, `'Z'` → 90, `' '` (スペース) → 32, `'0'` → 48
  - Main が `keydown` イベント受信時に `key` プロパティから判定
  - Worker が charCode 変換して enqueue
- **特殊キー**（矢印、Esc、Enter など）: TBD（予約済み、未定義）
  - 将来的に数値コード割り当て予定
  - 現在は処理対象外（キューにエンキューされない）
- **キューが空**: Rust 呼び出し時に 0 を返却

#### イベントキューへのアクセス

Rust タスクが `js_get_keyboard_event(window_id)` FFI 関数を呼び出すことで、対象ウィンドウのイベントキューから次のイベントコードをデキュー（取り出す）。

```rust
// Rust 側の使用例
let event_code = js_get_keyboard_event(window_id);
if event_code > 0 {
    let ch = event_code as u8 as char;  // ASCIIコード → 文字
    println!("User pressed: {}", ch);
} else {
    // キューが空、または特殊キー（未実装）
}
```

**呼び出しシーケンス**:

1. Main スレッドが `keydown` イベントをキャッチ
2. Canvas ウィンドウのアクティブ判定後、Worker に `keyboardEvent` メッセージ送信
3. Worker が `handleKeyboardEvent()` で:
   - イベント UUID から numeric window ID を逆引き
   - `key` が単一文字の ASCII 印字文字（0x20-0x7E）か判定
   - 有効な場合、charCode をイベントキューにエンキュー
4. Rust タスクが定期的に `js_get_keyboard_event(window_id)` を呼び出し
5. Worker が キューから次のイベントコードをデキューして返却
6. Rust タスクがコードを処理（表示、入力処理など）

**マルチウィンドウ環境**:
- 各ウィンドウのキー入力がすべて同じグローバルキューに入る
- Rust タスク A が `js_get_keyboard_event(windowId1)` を呼び出し
- Rust タスク B が `js_get_keyboard_event(windowId2)` を呼び出し
- 両者とも同じグローバルキューから順序通りデキュー
- `js_get_keyboard_event()` の windowId パラメータはキューを特定するために使われない（互換性のため受け付けるのみ）

#### スレッドセーフティ

- **イベントキュー操作**は Worker スレッド内のみで実行
- Main スレッドからアクセスなし
- すべてのウィンドウが同じキューを共有（ウィンドウID無関係）
- Worker 内での Rust FFI 呼び出しから逐次アクセス
- 並行アクセス問題なし（Worker は単一スレッド）

## 6. Rust 側のコンテキスト（OsContext）

**OsContext 構造体:**
```rust
pub struct OsContext {
    pub title_bar_height: u32,
}
```

**役割:**
- Rust アプリケーション内でシステム定数や UI パラメータを保持
- ウィンドウ作成時にレイアウト計算に使用可能

**パラメータ詳細:**
- `title_bar_height`: Main UI で定義された `TITLE_BAR_HEIGHT` 定数値（通常 32px）
  - Worker 起動時に `startWithCommand` メッセージで Main から渡される
  - `run_task(...)` 経由で Rust へ伝達される
  - Rust アプリケーションがウィンドウ内容領域をレイアウト計算する際に利用可能
  - 例：コンテンツ描画領域の Y オフセット計算時に使用

**使用例:**
```rust
let context = OsContext { title_bar_height: 32 };
let window = HariWindow::new(&context, "My App", Size::new(320, 152));
// title_bar_height を参考にして、ウィンドウ内の描画領域をレイアウト
let content_y_offset = context.title_bar_height as i32;
```
## 7. 実行フロー

### 7.1 単一 Worker の場合

1. 端末でコマンドライン入力
2. 未定義コマンドかつファイル名一致時、Worker を `startWithCommand` で起動
3. Worker 生成、Wasm モジュールをロード
4. wasm_bindgen でエクスポートされた `run_task(terminalWindowId, fileName, commandLine, titleBarHeight)` を呼び出す
5. Rust はインターフェースを通じてメインスレッドへ操作要求を送信
6. メインスレッドがウィンドウ状態更新および Canvas 描画を実施
7. Rust 関数が終了 (panic または return)
8. Worker は Main へ `done` または `error` メッセージを送信

### 7.2 未定義コマンド実行フロー

1. 端末でコマンドライン入力
2. 定義済みコマンドに一致しない場合、先頭トークンをファイル名として探索
3. ファイルが存在する場合、Worker を `startWithCommand` で起動
4. Worker は起動引数を受け取り、`println(windowId, text)` でデバッグ出力
5. Rust も `println(text)` / `print(text)` を呼べる
6. Worker は通常の Rust(Wasm) タスク実行を継続

**複数 Worker が並行実行する場合**：
- 各 Worker は Main から受け取ったスナップショットをベースに動作
- Task A が `js_write_file` で新規ファイルを作成
- Main が永続化 → 全 Worker に `updateFileSystemSnapshot` を配信
- Task B が その後 `js_read_file_size` でファイルを検索 → 正常に見つかる
- ファイルの重複上書きは、Main の `fileSystem` Map で最後の write が優先