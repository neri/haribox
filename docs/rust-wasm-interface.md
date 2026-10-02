# HariboteBox 設計書 - Rust/Wasmインターフェース

**この文書について**: このファイルはRust/Wasmタスクとの連携インターフェース、メッセージプロトコル、キーボード入力処理について記述しています。

**関連文書**:
- [プロジェクト概要](./overview.md) - 目的と技術スタック
- [状態管理](./state-management.md) - 状態管理とイベント処理
- [ファイルシステム](./filesystem.md) - ファイルシステムと永続化

---

## 1. Rust タスクの構成

Rust タスク（`rust-task/`）は、Haribote OS のアプリケーション（`.hrb`）を Worker 上で実行する。1 つのタスクが 1 つの Worker と 1 つの Wasm インスタンスを持つ。

- **実行方式**: x86 命令をエミュレータ（`lib/ume86`、命令デコードは `lib/ir86`）で実行し、Haribote OS の API 呼び出し（`INT 0x40`）を Rust 側で処理する（HLE）
- **実行ファイル**: HRB 形式（`lib/hrb`）。tek 圧縮されていれば展開してから読み込む（`lib/tek`）
- **API の実装**: `rust-task/src/haribote.rs` の `handle_syscall`。ウィンドウ（`window.rs`）、タイマー（`timer.rs`）、ファイル（`file.rs`）、日本語表示（`lang.rs`、フォントは `nihongo.fnt`）、メモリ確保（`malloc.rs`）に分かれる
- **ホストとの境界**: 画面・ファイル・音・時刻は、3 章の JavaScript 関数を通じて Worker に依頼する。キー入力は Worker から `push_key` で受け取る

タスクは協調的に動く。`step()` を 1 回呼ぶと、アプリが待ちに入る（キー待ち、キーの確認）か終了するまで実行して戻る。待ちに入らずに計算を続けるアプリは `step()` から戻らない（2 章の `step()` と 4.3 を参照）。

## 2. Rust エントリポイント関数

### `run_task(file_name, command_line)`
- wasm_bindgen でエクスポートされたメイン実行関数
- Worker から起動される正式なエントリポイント。実行ファイルを読み込み、アプリを実行できる状態にする（実行自体は `step()` で進める）
- パラメータ:
  - `file_name: String`: 実行対象ファイルの名前
  - `command_line: String`: 完全なコマンドライン文字列
- Worker が `startWithCommand` を受け取ったときに 1 回呼び出す
- 実行ファイルを読めない・HRB 形式でない場合はメッセージを出力して戻る。この場合、最初の `step()` が終了（`None`）を返す

### `step() -> Option<u32>`
- タスクを次に待ちが必要になるまで実行する。Worker が繰り返し呼び出す
- 戻り値: 次の呼び出しまでに待つ時間（ミリ秒）。タスクが終了した場合は `None`（JS 側は `undefined`）
  - キーまたはタイマー待ちの間は、次のタイマー満了までの時間（最大 10 ミリ秒）を返す
  - `0` は「溜まっているメッセージを処理したらすぐに再開する」を表す（`api_getkey(0)` の場合）
- Worker は 0 より大きい値なら `setTimeout` で待ち、`0` なら `MessageChannel` で一度イベントループに戻ってから再開する。入れ子の `setTimeout(0)` は最低 4 ミリ秒待たされるため、`0` には使わない
- タスクは `step()` から戻るまで Worker を占有する。キーの取得（`api_getkey` / `api_getkeyEx`）を呼ばずに計算し続けるアプリは、その間キー入力やファイルシステムの変更を受け取れない。ウィンドウを閉じたときの終了は Main が行うので、この場合も終了できる（4.3 を参照）

### `push_key(code)`
- Worker がキーボードイベントを Rust 側のイベントキューへ追加するために呼び出す
- `code`: イベントコード（下位 8 ビットがキーコード、ビット 8～15 が修飾キー状態）。形式は 5.2 を参照
- `run_task` の完了後から、タスク終了までの間だけ呼び出せる

## 3. Rust から利用するインターフェース

Rust 側は以下の JavaScript 関数を `#[wasm_bindgen(module = "env")]` 経由で呼び出します：

実装の詳細:
- `src/wasm/env.ts` が `env` モジュールとして提供される（`vite.config.ts` の alias で解決）。各関数の実装もこのファイルにある
- Worker (`rustTask.worker.ts`) が状態（`HostState`）を作成し、Wasm モジュールを読み込む前に `initHost()` で `env.ts` へ登録
- Worker と `env.ts` は同じ状態オブジェクトを共有する（ウィンドウ ID 対応表、ファイルシステムなど）
- 文字列は `&str`、バイト列は `&[u8]` / `Option<Vec<u8>>` で宣言し、Wasm メモリとの変換は wasm-bindgen の生成コードに任せる。`env.ts` 側は `string` / `Uint8Array` を受け取る
- `&[u8]` で受け取る `Uint8Array` は Wasm メモリへのビューで、呼び出し中しか有効でない。`postMessage` や保持の前に必ずコピーする

**インターフェース更新時の注意:**
- Rust 側で新しいインターフェース関数を追加する場合（`lib.rs` の `#[wasm_bindgen(module = "env")]` に追加）、以下を同時に更新必須：
  - `src/wasm/env.ts` に同名のエクスポート関数を実装
  - 本設計書にドキュメント記載

### 3.1 ウィンドウ管理インターフェース

- `js_open_window(width, height, title) -> u32`
  - Rust 側が指定したサイズ・タイトルでウィンドウを新規作成
  - `width`, `height` は描画領域（Canvas）のサイズ。タイトルバーや枠は含めず、Main がタイトルバーの高さを足してウィンドウを作る
  - Worker が Rust 数値ハンドルを採番し、戻り値として Rust 側へ返却
  - Worker 内で Rust 数値ハンドルと UI UUID を関連付ける
- `js_move_window(window_id, x, y)`
  - 指定ハンドルに対応するウィンドウの位置を変更
- `js_activate_window(window_id)`
  - 指定ハンドルに対応するウィンドウをアクティブ化
- `js_close_window(window_id)`
  - 指定ハンドルに対応するウィンドウを閉じる
- すでに閉じられたウィンドウのハンドルを指定した操作は無視する（ユーザーが閉じた後も Rust 側はハンドルを持ち続けるため）

### 3.2 描画インターフェース

- `js_draw_image(window_id, x, y, width, height, pixels)`
  - 指定ハンドルに対応するウィンドウの Canvas の指定矩形領域に RGBA imageData を描画
  - パラメータ:
    - `x`, `y`: 描画開始位置 (Canvas 内の座標)
    - `width`, `height`: 描画サイズ (ピクセル)
    - `pixels`: RGBA データ (`width * height * 4` バイト)

### 3.3 ターミナル出力インターフェース

- `js_print(text)`
  - Rust から Worker の `js_print` を呼び、Main の端末出力へテキストを追記する (改行なし)
  - 出力先端末は Main が決める（4.2 の `print` を参照）

### 3.4 ファイルI/Oインターフェース

- `js_read_file(filename) -> Option<Vec<u8>>`
  - 指定ファイル名のファイル内容を返す
  - ファイルが存在しない場合は `None`（JS 側は `undefined`）を返す
  - ファイル名は Main と同じ規則で正規化してから照合する。大文字小文字は区別しない
- `js_write_file(filename, data, mode) -> i32`
  - 指定ファイル名でファイル内容を書き込む
  - `mode` は以下の値：
    - `0`: update (存在する場合は上書き、存在しない場合はエラー)
    - `1`: create (存在しない場合は新規作成、存在する場合はエラー)
    - `2`: upsert (存在する場合は上書き、存在しない場合は新規作成)
  - 成功時は 0、失敗時は負値を返す。ファイル名を正規化できない場合と、`mode` の条件を満たさない場合に失敗する
  - 成否は Worker が手元のファイルシステムのコピーで判定する（[ファイルシステム](./filesystem.md) の 1.4 を参照）

### 3.5 時刻インターフェース

- `js_get_tick() -> f64`
  - Worker 初期化からの経過時間をミリ秒単位で返す
  - Worker 起動時に `performance.now()` を記録し、現在の `performance.now()` との差分を計算
  - 戻り値: 経過時間（ミリ秒、小数値）
  - 用途: Rust タスク内でタイマーの満了判定・フレームスキップ・アニメーション制御等に使用

イベントキューとタイマーは Rust 側で管理するため、JavaScript 側にイベント取得やタイマー予約の関数はない。詳細は 5.2 を参照。

### 3.6 音声インターフェース

- `js_play_sound(frequency)`
  - 指定した周波数（Hz）の音を鳴らす。`0` を指定すると止める
  - 音はタスクごとに 1 つで、鳴っている間に呼ぶと周波数を切り替える
  - `api_beep` から呼ばれる

## 4. Worker と Main のメッセージインターフェース

メッセージの型は `src/protocol.ts` に定義し、Main（`task/taskRunner.ts`、`input/keyboard.ts`）と Worker（`rustTask.worker.ts`、`wasm/env.ts`）の両方が import する。Main → Worker は `MainToWorkerMessage`、Worker → Main は `WorkerCommand`。メッセージを追加・変更するときはこのファイルを直す。

### 4.1 Main → Worker (Main が Worker へ送信するメッセージ)

- `startWithCommand`
  - Worker 起動時に Main から送信されるメッセージ
  - 起動引数：`fileName`、`commandLine`、`fileSystemSnapshot`
  - 環境変数（`SET` で設定するもの）は Main 側だけが使い、タスクには渡さない
  - 出力先端末や音声の識別子は渡さない。Main がタスクごとの記録（`taskRunner.ts` の `Task`）に持ち、メッセージの送信元 Worker から引く

- `key(code)`
  - Canvas ウィンドウがアクティブな時に押されたキーを Worker に転送
  - `code`: イベントコード（下位 8 ビットがキーコード、ビット 8～15 が修飾キー状態）。形式は 5.2 を参照
  - DOM のキーイベントからイベントコードへの変換は Main が行う（`src/input/taskKeyCode.ts`）。タスクに届けないキー（修飾キー単体、ファンクションキー、非 ASCII 文字など）と `keyup` は送らない
  - Worker は受け取った `code` をそのまま `push_key(code)` で Rust 側のイベントキューに追加する
  - Canvas ウィンドウ以外がアクティブの場合、転送されない

- `fileSystemChanged(changes)`
  - ファイルシステムが変更された際に、Main から全アクティブ Worker へ配信する
  - `changes`: 変更の配列。各要素は `{ type: 'put', name, content }`（作成・上書き）または `{ type: 'remove', name }`（削除）。名前変更は remove と put の 2 件
  - 送るのは変更分だけで、全ファイルは送らない（全ファイルを送るのは `startWithCommand` のスナップショットだけ）
  - Worker は手元のコピーに順に反映する。その後の `js_read_file` 呼び出しで、他の Worker の書き込み内容が読める
  - 書き込んだ Worker 自身にも届く。詳細は [ファイルシステム](./filesystem.md) の 1.4 を参照

- `windowClose(windowId)`
  - Canvas ウィンドウが閉じられたことを、そのウィンドウを開いたタスクの Worker に通知する。ユーザーがクローズボタンで閉じた場合も、タスクが `js_close_window` で閉じた場合も送る
  - パラメータ:
    - `windowId`: 閉じられたウィンドウの UUID
  - Worker は数値ハンドルと UUID の対応を削除する。以後そのハンドルへの操作（描画など）は Main へ送らない
  - Rust 側には伝えない。Haribote OS にはウィンドウが閉じられたことをアプリへ知らせる API がない
  - タスクを終了するかどうかは Main が決める（4.3 を参照）。Worker はこのメッセージで終了しない

### 4.2 Worker → Main (Worker が Main へ送信するメッセージ)

- `done`
  - タスクが終了したことを Main へ通知
  - パラメータなし
  - 送信契機: `step()` が終了（`None`）を返したとき
    - アプリが `api_end` を呼んだ
    - 未対応の API を呼んだ
    - `run_task` が実行ファイルを読み込めなかった
  - アプリが開いていたウィンドウは、終了時に Rust 側が `js_close_window` ですべて閉じる
  - Main の処理: タスクを解放する（4.3 を参照）

- `error(message, stack)`
  - Worker でエラーが発生したことを Main へ通知
  - パラメータ:
    - `message`: エラーメッセージ文字列
    - `stack`: スタックトレース（オプション）
  - 発生契機:
    - WASM モジュール読み込み失敗
    - WASM ランタイム初期化失敗
    - アプリの実行時例外（不正な命令、範囲外のメモリアクセスなど）や Rust 側の panic
  - Worker は送信前に、エラーメッセージを `println` でタスクの出力先端末へ出す
  - Main の処理: `console.error` に記録し、タスクを解放する（4.3 を参照）

- `fileWritten(filename, data)`
  - Worker が `js_write_file` でファイルを書き込んだ際、Main へ通知
  - パラメータ:
    - `filename`: 書き込みされたファイル名（Worker が正規化した後の名前）
    - `data`: 書き込み内容の `ArrayBuffer`
  - 名前の検証と書き込みモードの判定は Worker が済ませている。Main は再判定しない
  - Main の処理:
    - ファイルシステムを更新し、localStorage へ永続化
    - 全アクティブ Worker に `fileSystemChanged` で配信

- `println(text)`
  - Worker から Main へ改行付きテキスト出力要求を送る
  - Main は送信元タスクの出力先端末へ `text` を 1 行追加する
  - 出力先は、タスクを起動した端末。その端末が閉じられていれば既定の端末。出力なしで起動したタスク（START/NCST/OPEN）や、端末が 1 つも無い場合は無視する

- `print(text)`
  - Worker から Main へ改行なしテキスト出力要求を送る
  - Main は送信元タスクの出力先端末の現在行末へ `text` を追記する（出力先の決め方は `println` と同じ）

- `playSound(frequency, timestamp)`
  - Rust の `js_play_sound` を Main へ伝える。`frequency` が 0 なら停止
  - 音はタスクごとに 1 つ。Main が送信元タスクのオシレーターを切り替え、タスク終了時に停止する

- `drawImage(windowId, x, y, width, height, pixels)`
  - Worker から Main へ Canvas への画像描画要求を送る
  - Worker が Rust の `js_draw_image` を通じて受け取った RGBA データを Canvas に描画
  - パラメータ:
    - `windowId`: Canvas ウィンドウの UUID
    - `x`, `y`: 描画開始位置 (Canvas 内の座標)
    - `width`: 画像幅 (ピクセル)
    - `height`: 画像高さ (ピクセル)
    - `pixels`: RGBA データ (`ArrayBuffer`、各ピクセルが RGBA で 4 バイト)
  - Worker は Wasm メモリから 1 回だけコピーし、そのバッファを transfer で Main に渡す（`postMessage` での複製はしない）
  - Main は指定ウィンドウの Canvas に `putImageData` で指定座標へ描画
  - `windowId` が Canvas ウィンドウとして存在しない場合は無視する

### 4.3 タスクの終了

タスクの終了は、すべて Main の `releaseTask`（`task/taskRunner.ts`）を通る。`releaseTask` は Worker を terminate し、タスクが鳴らしていた音を止め、ウィンドウとの対応を消す。`releaseTask` 自体は Canvas ウィンドウを閉じない（正常終了ではアプリ側が先に閉じている。`error` で終了した場合はウィンドウが残る）。

終了の契機は次の 4 つ。

| 契機 | 判定する側 |
|---|---|
| `done` を受け取った（アプリが終了した） | Worker |
| `error` を受け取った | Worker |
| Worker の `error` イベント（スクリプトを読み込めない、捕捉されない例外など） | Main |
| 最後の Canvas ウィンドウが閉じられた | Main |

**最後のウィンドウが閉じられたとき**:

1. Canvas ウィンドウが閉じられる（ユーザーの操作、またはタスクの `js_close_window`）
2. Main がそのタスクのウィンドウが 1 つも残っていないことを確認し、100 ミリ秒のタイマーを張る
3. タイマーが満了した時点で、タスクがまだウィンドウを持っていなければ `releaseTask` する
4. 待っている間にタスクが新しいウィンドウを開いていれば、タスクは継続する

- 100 ミリ秒の猶予は、ウィンドウを閉じてすぐ開き直すアプリを終了させないためのもの
- 判定を Main に置いているのは、`step()` から戻らないタスクでも終了できるようにするため。Worker 側で判定すると、応答しないタスクは `windowClose` を処理できず、ウィンドウが消えても動き続ける
- 一度もウィンドウを開いていないタスク（ターミナルに出力するだけのアプリ）は、この判定の対象にならない。`done` か `error` まで動き続ける
- 複数のウィンドウのうち一部だけが閉じられた場合、タスクは継続する

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
   - `getWorkerByWindowId(state.activeWindowId)` で対応する Worker を検索
   - Canvas ウィンドウが `js_open_window` FFI で生成された時に自動登録される
   - Worker が見つからない場合は処理を中断

4. **Main スレッド - イベントコードへの変換と転送**：
   - `keydown` のとき、`toTaskKeyCode(key, code, modifierBitmap)` でイベントコードへ変換する
     - 印字可能ASCII文字（`0x20`～`0x7e`）は文字コード
     - Backspace、Enter、Esc、PageUp/PageDown、Home/End、矢印、Insert、Delete は定義済みの特殊キーコード
     - 修飾キー状態（`modifierBitmap`）を上位8ビットにエンコードする。ビットの対応は後述の「イベントコード形式」を参照
     - それ以外のキーは変換結果なし（送信しない）
   - 変換できた場合、Worker に `{ type: 'key', code }` を `postMessage` で送信
   - `keyup` は修飾キー状態の更新にだけ使い、Worker へは送らない
   - 送信の有無にかかわらず、`event.preventDefault()` でブラウザのデフォルト動作を抑止

5. **Worker スレッド - イベントキューへの追加**：
   - `key` メッセージを受け取り、`push_key(code)` で Rust 側のイベントキューに追加する

6. **Rust タスク - イベント取得**：
   - `api_getkey` / `api_getkeyEx` の処理時に、Rust 側のイベントキューから次のイベントを取り出す

#### 非 Canvas ウィンドウ時の挙動

- Terminal または FileManager がアクティブの場合、キーボードイベントは転送されない
- これらのウィンドウは独自のキーボードハンドラを持つため
- Canvas ウィンドウから他ウィンドウへの切り替え時、キーボード転送も自動的に停止

### 5.2 イベントキュー機能

#### イベントキューの概要

イベントキューは Rust 側（`App`）が 1 つ持ち、タスクのすべてのウィンドウが共有する。キューにはキーイベントとタイマーイベントの 2 種類が入る。

- **キーイベント**: Worker が `push_key(code)` で追加する
- **タイマーイベント**: `api_settimer` で設定した時刻（`js_get_tick()` 基準）を過ぎたタイマーを、Rust 側がイベント取得時とキー追加時にキューへ移す。満了時刻の順に並ぶ
  - `api_freetimer` は未満了のタイマーを取り消す。すでにキューへ入ったイベントは残る
  - 動作中のタイマーに再度 `api_settimer` すると、満了時刻を置き換える

**イベントコード形式**:
- **文字キー**（`keydown` イベント）: ASCIIコード（0x20-0x7E）
  - 例: `'a'` → 97, `'A'` → 65, `'Z'` → 90, `' '` (スペース) → 32, `'0'` → 48
- **特殊キー**（`keydown` イベント）:

  | キー | コード |
  |------|-------:|
  | Backspace | `0x08` |
  | Enter | `0x0a` |
  | Escape | `0x1b` |
  | PageUp / PageDown | `0x80` / `0x81` |
  | End / Home | `0x82` / `0x83` |
  | ArrowLeft / ArrowRight | `0x84` / `0x85` |
  | ArrowUp / ArrowDown | `0x86` / `0x87` |
  | Insert / Delete | `0x88` / `0x89` |

- **修飾キー**: Main が持つ修飾キー状態（USB HID 形式、左右区別）をHaribote形式に変換し、イベントコードのビット8～15へ格納する。左Shift/Ctrl/Alt/Metaはビット8～11、右Shift/Ctrl/Alt/Metaはビット12～15に対応する
- **`keyup`**: Worker へ転送しない
- **キューが空**: `api_getkey(0)` は `-1` を返し、`api_getkey(1)` はイベントが来るまで待つ

**Haribote OS API への変換**:
- 通常の `api_getkey` はイベントコードの下位8ビットを利用する。ArrowLeft/Right/Up/Down はそれぞれ `0x34` / `0x36` / `0x38` / `0x32` へ変換し、その他の `0x80` 以上の特殊キーは無効値として扱う
- 拡張 `api_getkeyEx` は修飾キーを含むイベントコード全体を返す
- タイマーイベントは、どちらの API でも `api_inittimer` で設定したデータをそのまま返す

#### 呼び出しシーケンス

1. Main スレッドが `keydown` イベントをキャッチ
2. Canvas ウィンドウのアクティブ判定後、イベントコードへ変換して Worker に `key` メッセージ送信
3. Worker が `push_key(code)` を呼ぶ
4. Rust 側が満了済みタイマーをキューへ移してから、キーイベントをキューに追加
5. タスクが `api_getkey` を呼ぶと、Rust 側がキューから次のイベントを取り出して返す

`push_key` は Worker のメッセージハンドラから呼ばれる。`step()` の実行中にメッセージハンドラは動かないため、Rust 側のキューへのアクセスが重なることはない。

## 6. 実行フロー

### 6.1 タスクの起動から終了まで

1. 端末でコマンドライン入力
2. 定義済みコマンドに一致しない場合、先頭トークンをファイル名として探索（[状態管理](./state-management.md) の 4.3 を参照）
3. ファイルが存在する場合、Main が Worker を生成して `startWithCommand` を送る。ファイルマネージャからの起動も同じ経路を通る
4. Worker がファイルシステムのコピーを作り、Wasm モジュールをロードする
5. Worker が `run_task(fileName, commandLine)` を呼ぶ。Rust が起動ログ（`[rust] run_task(...)`）を出力先端末に出す
6. Worker が `step()` を繰り返し呼ぶ。戻り値の時間だけ待ってから次を呼ぶ
7. Rust は 3 章のインターフェースを通じて Main へ操作を依頼し、Main がウィンドウ状態の更新や Canvas 描画を行う
8. `step()` が終了を返したら、Worker は Main へ `done` を送る。例外が起きた場合は `error` を送る
9. Main がタスクを解放する（4.3 を参照）

**複数 Worker が並行実行する場合**：
- 各 Worker は起動時のスナップショットと、その後の変更分をもとに動作
- Task A が `js_write_file` で新規ファイルを作成
- Main が永続化 → 全 Worker に `fileSystemChanged` を配信
- Task B が その後 `js_read_file` でファイルを検索 → 正常に見つかる
- ファイルの重複上書きは、Main が後から処理した write が優先
