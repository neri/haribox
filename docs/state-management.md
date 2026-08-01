# HariboteBox 設計書 - 状態管理とイベント処理

**この文書について**: このファイルはHariboteBoxの状態管理、イベント設計、アクティブウィンドウシステム、端末コマンドの仕様を記述しています。

**関連文書**:
- [プロジェクト概要](./overview.md) - 目的と技術スタック
- [UI設計](./ui-design.md) - 画面構成とウィンドウ仕様
- [ファイルシステム](./filesystem.md) - ファイルシステムと永続化

---

## 1. 状態管理設計

### 1.1 状態モデル

```ts
type WindowId = string;  // UUIDv4 形式のウィンドウ固有ID

type AppId = string;  // UUIDv4 形式のアプリケーション種別ID

type WindowKind = 'canvas' | 'terminal' | 'filemanager' | 'about' | 'textviewer' | 'onboarding' | 'systemmodal';

type WindowModel = {
  id: WindowId;  // ウィンドウ個別ID
  appId: AppId;  // ウィンドウタイプのアプリID
  kind: WindowKind;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  isActive: boolean;
};

type EnvironmentVariables = {
  [key: string]: string;
};

type WindowGroupInfo = {
  appId: AppId;
  kind: WindowKind;
  windowIds: WindowId[];  // グループに属するウィンドウID リスト
  isExpanded: boolean;    // グループのドロップダウン展開状態
};

type AppState = {
  windows: WindowModel[];
  nextZIndex: number;
  env: EnvironmentVariables;
  isMenuOpen: boolean;
  defaultTerminalWindowId: WindowId | null;
  activeWindowId: WindowId | null;
  windowGroups: WindowGroupInfo[];  // ウィンドウグループ化情報（タスクバー表示用）
};

// アプリID定数
const APP_IDS = {
  TERMINAL: 'f9c271dc-49ee-4295-811a-dc2bf7bd27a7',
  CANVAS: '8faf5f5e-66ee-4580-8a8f-fe64ed78ea23',
  FILE_MANAGER: 'c1a107bb-20ec-4d3d-ad94-944d5bce863d',
  ABOUT: '88a37a04-c8e5-42e6-9f13-2f7fd31f62c3',
  TEXT_VIEWER: 'e97b8139-cebf-40cd-8805-a0b87192c50f',
  ONBOARDING: 'd1b3e8f4-7a2e-4c9d-b5f1-9a8c6d2e4f3b',
  SYSTEM_MODAL: 'a4e9f2c1-5d7b-4e2a-9c3f-8b1d6a5e2f9c',
} as const;
```

### 1.2 実装定数

| 定数名 | 値 | 説明 |
|--------|-----|------|
| `TASKBAR_HEIGHT_PX` | 48 | タスクバーの高さ |
| `TERMINAL_WINDOW_OFFSET_STEP` | 24 | 新規端末ウィンドウのカスケード配置オフセット |
| `TERMINAL_MAX_LINES` | 4000 | ターミナルのスクロールバッファ最大行数 |
| `TITLE_BAR_HEIGHT` | 32 | ウィンドウのタイトルバーの高さ |
| `Z_INDEX_REFRESH_THRESHOLD` | 10,000 | Z インデックス最適化の閾値 |
| `INITIAL_TERMINAL_WINDOW_HEIGHT` | 320 | 初期端末ウィンドウの高さ |
| `INITIAL_TERMINAL_WINDOW_WIDTH` | 520 | 初期端末ウィンドウの幅 |
| `INITIAL_TERMINAL_X` | 40 | 最初の端末ウィンドウの X 座標 |
| `INITIAL_TERMINAL_Y` | 40 | 最初の端末ウィンドウの Y 座標 |

### 1.3 環境変数初期値

- `PATH_EXT`: `.hrb` (拡張子リスト、`:` で区切る)

### 1.4 メニュー状態初期値

- `isMenuOpen`: `false` (起動時はメニュー非表示)
- `defaultTerminalWindowId`: `null` (起動時は端末がないため null)
- `activeWindowId`: `null` (起動時はアクティブなウィンドウなし)
- `windowGroups`: `[]` (起動時はグループなし)

### 1.5 ターミナル履歴管理

実装でのグローバル状態（AppState に含まれない付帯情報）:

- `terminalHistoryByWindow: Map<WindowId, string[]>`
  - 各ターミナルウィンドウの過去実行コマンド履歴
  - 最新のコマンドが配列の先頭（インデックス 0）
  - 永続化されない（セッション中のみ保持）
  - ウィンドウクローズ時にクリア

- `terminalHistoryIndexByWindow: Map<WindowId, number>`
  - 各ターミナルウィンドウの現在の履歴ブラウズ位置
  - `-1`: 履歴ブラウズ中ではない（入力中の状態）
  - `0以上`: 履歴配列のインデックス
  - ウィンドウクローズ時にクリア

- `terminalPendingInputByWindow: Map<WindowId, string>`
  - 履歴ブラウズ開始時に入力途中のテキストを一時保存
  - 履歴ブラウズ中に下キーで最後の位置に戻った場合に復元
  - コマンド実行時にクリア
  - ウィンドウクローズ時にクリア

**履歴管理ルール:**
- **コマンド実行時の処理**:
  1. `skipHistory` オプションが指定されていない場合のみ履歴に追加
  2. 重複チェック：同じコマンドが履歴に存在する場合、古い方を削除
  3. 先頭に追加：正規化されたコマンドを履歴配列の先頭に `unshift`
  4. ブラウズ位置リセット：`historyIndex = -1`
  5. 保留中入力クリア：`pendingInput` を削除
  6. **初期化コマンド除外**：ターミナルウィンドウ生成時の自動 `VER` 実行では `skipHistory: true` を指定

- **上キー（ArrowUp）での操作**:
  1. 初回ブラウズ時：現在の入力を `pendingInput` に保存
  2. `historyIndex` をインクリメント（古い履歴へ移動）
  3. 最大値（`history.length - 1`）でクランプ
  4. 入力フィールドに履歴内容を表示

- **下キー（ArrowDown）での操作**:
  1. `historyIndex` をデクリメント（新しい履歴へ移動）
  2. `historyIndex === -1` になった場合は `pendingInput` を復元
  3. 負の値にならないよう処理
  4. 入力フィールドを更新

### 1.6 状態更新ルール

- ウィンドウ追加:
  - 新規 ID 発行
  - 1 個限定ウィンドウの判定:
    - `appId` が `APP_IDS.FILE_MANAGER` または `APP_IDS.ABOUT` の場合、同じ `appId` を持つウィンドウが既に存在するかチェック
    - 存在する場合はそのウィンドウを最前面化のみ（新規作成しない）
    - 存在しない場合のみ新規作成
  - 端末ウィンドウの初期座標:
    - **カスケード配置**で左上(40, 40)から開始
    - 新規ウィンドウ作成時に内部カウンタを増やし、毎回オフセット（24px）を加算
    - `x = 40 + (nextTerminalSpawnIndex * 24)`、`y = 40 + (nextTerminalSpawnIndex * 24)`
    - デスクトップ外へはみ出す場合は `clampWindowToDesktop()` でデスクトップ内へ収納
  - Rust/Canvas ウィンドウの初期座標:
    - ウィンドウサイズから計算してデスクトップ中央に配置
  - `zIndex = nextZIndex` で追加後、`nextZIndex` をインクリメント
  - 端末ウィンドウ追加時:
    - `defaultTerminalWindowId` が null の場合のみ、新規追加されたウィンドウ ID を `defaultTerminalWindowId` に設定
    - `defaultTerminalWindowId` が既に設定されている場合は変更しない
- 前面化:
  - 対象の `zIndex` が最大未満の場合のみ、`zIndex = nextZIndex` に更新
  - 更新後に `nextZIndex` をインクリメント
- 移動:
  - ドラッグ差分で `x,y` 更新
  - 通常時は `x,y` をクランプしない (はみ出し許可)
  - ドラッグ中の差分計算に使用するポインタ座標のみデスクトップ境界でクランプ
  - `y` は最小値 0 でクランプする
- 個別削除:
  - `id` 一致要素を削除
  - 削除対象が `defaultTerminalWindowId` に一致する場合:
    - 削除後、残存する端末ウィンドウが存在すればそのうち最初のものを新しい `defaultTerminalWindowId` に設定
    - 残存する端末ウィンドウがない場合は `defaultTerminalWindowId` を null に設定
  - 削除対象が `activeWindowId` に一致する場合:
    - 削除後、他のウィンドウが存在すればそのうち最も上前にあるもの（最大 zIndex）を新しい `activeWindowId` に設定
    - 他にウィンドウがない場合は `activeWindowId` を null に設定
- アクティブウィンドウの切り替え:
  - ウィンドウをクリックすると `activeWindowId = windowId` に設定
  - `bringToFrontIfNeeded()` を呼び出した時に自動的にそのウィンドウをアクティブ化
  - ウィンドウ内のサブアプリケーションがアクティブウィンドウのみキー入力を受け付ける
- 全削除:
  - `windows = []`
  - `nextZIndex = 1` にリセット
- 環境変数設定:
  - `env[name] = value` で更新
- メニューの表示/非表示:
  - `isMenuOpen = true` でメニュー表示
  - `isMenuOpen = false` でメニュー非表示
- メニュー項目の実行:
  - ターミナル選択時: 新規端末ウィンドウを作成、メニュー自動クローズ
  - ファイル選択時: ファイルマネージャウィンドウを起動（既存インスタンスがあれば最前面化）、メニュー自動クローズ
  - About... 選択時: About ウィンドウを起動（既存インスタンスがあれば最前面化）、メニュー自動クローズ

## 2. イベント設計

### 2.1 UI イベント

- クリック (ハンバーガーメニューボタン): メニュー表示/非表示トグル
- クリック (メニュー外部): メニューをクローズ
- ESC キー: メニューをクローズ
- クリック (新規端末メニュー項目): 新規端末作成、メニュー自動クローズ
- クリック (ファイルメニュー項目): ファイルマネージャウィンドウを起動、メニュー自動クローズ
- タイマー更新: 時計表示の更新
- ポインタダウン: ウィンドウ前面化判定
- ポインタダウン (タイトルバー): ドラッグ開始
- ポインタムーブ: ドラッグ中位置更新
- ポインタアップ/キャンセル: ドラッグ終了
- クリック (閉じる): 個別削除
- Enter または実行ボタン: 端末コマンド実行
- **上下キー (ターミナルウィンドウ)**: コマンド履歴ブラウズ
  - 上キー（ArrowUp）: 古いコマンドを履歴から呼び出す
  - 下キー（ArrowDown）: 新しいコマンドを履歴から呼び出す、または入力中のテキストに戻す
- dragover: 既定動作を抑止
- drop: ファイル取り込み（サイズチェック: 既存+新規合計が 1.5MiB 超過時はエラーダイアログを表示して中止）
- ファイルマネージャでのカーソルキー (上下): ファイル選択移動
- ファイルマネージャでのマウスクリック: ファイル選択
- ファイルマネージャでの Enter キー/ダブルクリック: ファイル実行（拡張子に応じた動作）
- ファイルシステム変更: ファイルマネージャ自動更新

### 2.2 イベント処理順序

- タイトルバー操作時:
  - 前面化
  - ドラッグ開始
- 本文操作時:
  - 前面化のみ

### 2.3 アプリケーション初期化

**起動時に自動生成されるウィンドウ:**

1. **ファイルシステム初期化**
   - localStorage から保存ファイルを復元（存在しない場合は初期ファイルを読み込み）

2. **デフォルト端末ウィンドウ**
   - 最初の端末ウィンドウを自動作成
   - 自動的に `VER` コマンドを実行
   - デフォルト端末として設定

3. **ファイルマネージャウィンドウ**
   - ファイルシステム初期化後に自動作成
   - 初期位置: デスクトップ右上（右端から20px、上端からターミナルと同じ高さ）
   - デフォルトサイズ: 幅 300px × 高さ 400px
   - アクティブ状態で表示

4. **ようこそ（Onboarding）ウィンドウ**
   - デスクトップ中央に自動生成
   - ユーザーが「開始する」ボタンをクリックするまで表示

## 3. アクティブウィンドウシステム

### 3.1 概要

複数ウィンドウが存在する場合、キーボード入力を受け付けるウィンドウを明確にするための仕組み。アクティブウィンドウのみがサブアプリケーション（ファイルマネージャ、ターミナルなど）のキー操作を受け付ける。

### 3.2 ビジュアル表現

- **アクティブウィンドウ**:
  - タイトルバー背景色: `#0b5ed7` (青)
  - タイトルバーテキスト色: 白 (#fff)
  - タイトルバーアイコン色: 白 (#fff)
  - フォント太さ: Bold
  - ファイルマネージャのリストボーダー: 2px solid #0b5ed7
  - ターミナルのボーダーセパレーター: 青色
  
- **非アクティブウィンドウ**:
  - タイトルバー背景色: `#c0c0c0` (灰色)
  - タイトルバーテキスト色: 黒 (#111)
  - タイトルバーアイコン色: 黒 (#111)
  - フォント太さ: Normal
  - リスト/セパレーターボーダー: 灰色

### 3.3 キー操作の仕様

- **ファイルマネージャ**:
  - アクティブな場合のみ以下のキー入力を受け付ける
  - `ArrowUp`: ファイルリストで選択を上に移動
  - `ArrowDown`: ファイルリストで選択を下に移動
  - `Enter`: 選択ファイルを実行/オープン
  
- **ターミナル**:
  - アクティブな場合のみ入力テキストボックスでのキー入力が有効
  - 非アクティブ時のキー入力は無視

### 3.4 アクティブ化のトリガー

- **初期アクティブ化**: 起動後、最初のウィンドウ作成時に自動的にアクティブ化
- **クリックによるアクティブ化**: ウィンドウ内の任意領域をクリックするとそのウィンドウがアクティブ化
- **前面化時のアクティブ化**: `bringToFrontIfNeeded()` 呼び出し時に自動的にアクティブ化
- **リスト内のクリック**: ファイルマネージャのファイルリストをクリックするとそのウィンドウがアクティブ化

### 3.5 非機能要件

- マウス/タッチの双方で操作できる実装を推奨
- 画面リサイズ時にも破綻しないレイアウト
- 単一ページ遷移なしで操作継続

## 4. 端末コマンド仕様

- コマンド入力テキストボックスで Enter 押下、または実行ボタン押下でコマンドを実行
- 実行後に入力テキストボックスをクリア

### 4.1 コマンド掲載ポリシー

- 定義済みコマンドの総数が多いと、ユーザの認知負荷が増加する
- そのため、**HELPコマンドには選定されたコマンドのみを掲載する**
- すべてのコマンドをリストアップするのではなく、基本的かつ頻用されるコマンドに限定
- このポリシーにより、ユーザは必要最小限の情報で基本操作を学習できる

### 4.2 定義済みコマンド

- **定義済みコマンド:**
  - `VER`: アプリバージョンと git ハッシュを表示
  - `ECHO`: 入力パラメータをそのまま表示
  - `CLS`: 現在端末の表示をクリア
  - `DIR`: ファイル一覧とサイズ、合計サイズを表示
  - `LS`: DIR のエイリアス
  - `TYPE <filename>`: テキストとしてファイル内容を表示
  - `COPY <source> <destination>`: ファイルを複製
  - `DEL <filename>`: ファイルを削除
  - `REN <source> <destination>`: ファイル名変更
  - `EXIT`: 実行した端末ウィンドウを閉じる
  - `SET <name>=<value>`: 環境変数を設定 (例: `SET PATH_EXT=.hrb:.exe`)
    - 環境変数名に `=` が含まれない場合のみ、既存の環境変数値を表示
    - 例: `SET PATH_EXT` で現在の `PATH_EXT` 値を表示
  - `HELP`: 基本的なコマンドの一覧と簡単な説明を表示（掲載コマンド: VER, DIR, TYPE, NCST）
  - `START <filename> [args...]`: ダミーターミナルでタスク実行（出力非表示）
    - Rust タスクを起動するが、出力をターミナルに表示しない
    - 起動エラーは `console.error` にのみ出力される
    - パラメータは通常のタスク起動と同じ
    - 例: `START foo bar` → `foo.hrb` を起動、コマンドラインは `foo bar` として渡される
    - デフォルトターミナルへのフォールバック出力も行われない
  - `NCST <filename> [args...]`: ダミーターミナルでタスク実行（出力非表示）
    - START コマンドと完全に同じ動作
    - 例: `NCST foo bar` → `foo.hrb` を起動、コマンドラインは `foo bar` として渡される
  - `OPEN <filename> [args...]`: START / NCST のエイリアス（ダミーターミナルでタスク実行、出力非表示）
    - START と NCST と完全に同じ動作

### 4.3 未定義コマンド

- 上記定義コマンドに一致しない場合:
  - 先頭トークンをファイル名として検索
  - 検索手順:
    1. 正規化したコマンド名がファイルシステム上に存在するか確認
    2. 存在しない場合、`PATH_EXT` 環境変数にある拡張子を先頭トークンに追加して検索
    3. `PATH_EXT` は `:` で区切られた拡張子リスト (例: `.hrb:.exe:.bin`)
    4. いずれかのパターンでファイルが見つかればそれを使用
  - ファイルが見つかった場合、Worker を起動 (起動引数は `terminalWindowId`・`fileName`・`commandLine`)
  - ファイルもコマンドも見つからない場合は「Bad command or file name」を表示

## 5. タスクバーウィンドウ一覧機能

### 5.1 目的

タスクバーに開いているウィンドウの一覧を表示し、ウィンドウの切り替えと管理を効率化する機能。

### 5.2 実装方針

**ウィンドウグループ化の計算処理**:
- ウィンドウ一覧の変化（作成・削除・タイトル変更）を監視
- ウィンドウタイプごとにグループ化情報を再計算
- グループ化ロジック：
  ```
  function groupWindowsByKind(windows: WindowModel[]): WindowGroupInfo[] {
    // 1. canvas タイプは個別表示（グループ化しない）
    // 2. terminal, filemanager, about, textviewer は同じ種類ごとにグループ化
    // 3. 各グループの windowIds は作成順（zIndex 順）に保持
    // 4. 新規グループの isExpanded は初期値 false
    // 5. グループが空になった場合は削除
  }
  ```

**アイコン対応表**:
- `terminal` → `terminal-2.svg`
- `filemanager` → `folder.svg`
- `about` → `category.svg`
- `textviewer` → `file-text.svg`
- `canvas` → `app-window.svg`（タスク定義に応じて変更可能）

**ボタンスタイル**:
- アクティブウィンドウ: 背景を青色 (#0b5ed7)、文字を白 (#fff)
- 非アクティブウィンドウ: 背景をデフォルト、文字を黒 (#111)
- ホバー時: 背景を薄い灰色 (#e9ecef) で表示

### 5.3 実装対象ファイル

- `src/main.ts`
  - ウィンドウグループ化ロジック実装
  - グループ化情報の状態管理
  - タスクバーボタンのイベントハンドラー（クリック・ホバー）
  - ドロップダウンメニュー表示/非表示制御

- `src/style.css`
  - タスクバー中央領域のレイアウト (flexbox)
  - ウィンドウボタンのスタイル（サイズ、配色、ホバー効果）
  - ドロップダウンメニューのスタイル（背景、テキスト、アニメーション）
  - グループボタンと個別ボタンの区別表示

### 5.4 状態管理に関わる規則

**ウィンドウ作成時**:
- 新規ウィンドウを `windows` 配列に追加
- `windowGroups` を再計算
- 新規グループが作成される場合は `isExpanded: false` で初期化
- 既存グループにウィンドウが追加される場合、グループの `windowIds` を更新

**ウィンドウ削除時**:
- 対象ウィンドウを `windows` 配列から削除
- `windowGroups` を再計算
- グループが空になった場合、そのグループを `windowGroups` から削除

**ウィンドウタイトル変更時**:
- `windows` 内の対象ウィンドウモデルを更新
- `windowGroups` の再計算は実行しない（グループ化に影響しないため）
- UI 再レンダリングで新タイトルが反映

**ウィンドウアクティブ状態変化時**:
- `activeWindowId` を更新
- タスクバーボタンのスタイル更新（既存実装と同じ）
- グループボタンは子ウィンドウがアクティブな場合、グループボタン自体もハイライト表示

**グループドロップダウン展開状態**:
- グループボタンクリック時に対応 `WindowGroupInfo.isExpanded` をトグル
- ドロップダウン外をクリック時、すべてのグループを自動クローズ（`isExpanded: false`）
- グループ内の個別ウィンドウをクリック時、クリック後に自動クローズ

### 5.5 実装の流れ

1. `WindowGroupInfo` 型定義を `src/main.ts` に追加
2. `groupWindowsByKind()` 関数を実装
3. `AppState` に `windowGroups` フィールドを追加し、初期化時に計算
4. ウィンドウ操作（作成・削除）時に `windowGroups` 再計算ロジックを組み込み
5. タスクバー HTML マークアップを拡張（ハンバーガーメニューと時計の間にボタン領域を追加）
6. タスクバーボタンのレンダリング関数を実装
7. CSS でウィンドウボタンのスタイルとドロップダウンメニューのレイアウトを定義
8. グループボタン・個別ボタン・ドロップダウンメニューのイベントハンドラーを実装
9. テストで以下を確認：
   - 複数ウィンドウ作成時にボタンが正しく表示される
   - グループ化と個別表示が正しく処理される
   - グループボタンのホバー・クリック時にドロップダウンが表示/非表示される
   - ドロップダウン内のウィンドウをクリックするとそのウィンドウが前面化される
   - ウィンドウ削除時にボタンが消える
   - ウィンドウタイトル変更時にボタンテキストが更新される

## 6. アーキテクチャ設計補足

本セクションは、状態管理設計の実装上の詳細を補足。

### 6.1 Main スレッドの状態管理

- AppState は単一インスタンスで、全ウィンドウの状態を一元管理
- ウィンドウの作成・削除・移動などの操作は、AppState を経由して行われる
- 状態変更時は renderWindows() が自動的に呼び出され、画面が更新される

### 6.2 Worker との通信プロトコル

- Message passing による非同期通信
- Rust コード内で js_* 関数呼び出し → Worker が main へメッセージ送信
- main が対応する状態変更処理を実行 → Worker へ応答メッセージ返送

### 6.3 localStorage による永続化

- 起動時に localStorage から初期ファイルシステムを復元
- ファイルシステムの変更は自動的に localStorage へ保存
- 複数 Worker が同時に実行される場合、最後の変更が優先される

## 7. System Modal State Management

### 7.1 System Modal の状態変数

```typescript
// システムモーダルウィンドウの ID
let systemModalWindowId: WindowId | null = null;

// System Modal の動作モードと結果情報を管理
// Key: WindowId, Value: { mode: 'import' | 'complete', filename?: string, error?: string }
const systemModalModeByWindowId = new Map<WindowId, {
  mode: 'import' | 'complete';
  filename?: string;
  error?: string;
}>();

// 現在のドラッグセッション中にModalが作成済みかを追跡
let modalCreated = false;
```

### 7.2 System Modal のライフサイクル

| ステップ | トリガー | 状態変化 | 説明 |
|---------|---------|----------|------|
| 1. dragenter | dragenter イベント | `modalCreated = false` にリセット | 新しいドラッグセッション開始 |
| 2. dragover(初回) | dragover イベント + !modalCreated | `systemModalWindowId` に新 UUID、Mode を 'import' に設定、`modalCreated = true` | Mode 1 Modal 表示開始 |
| 3. dragover(継続) | dragover イベント（繰り返し） | 状態変化なし（既存 Modal 再利用） | ドラッグ継続中、Mode 1 を維持 |
| 4. キャンセル | dragleave イベント（viewport外） | `closeWindow(systemModalWindowId)`、`modalCreated = false` | Modal 非表示 |
| 5. ドロップ | drop イベント | `modalCreated = false`、非同期で `importDroppedFiles()` 実行開始 | ファイル取込開始 |
| 6. 結果反映 | import 完了 | Mode を 'complete' に変更、filename/error をセット、`renderWindows()` 呼び出し | Mode 2 メッセージ更新 |
| 7. 確認 | OK ボタン押下 | `closeWindow(systemModalWindowId)` 呼び出し | Modal 非表示 |

### 7.3 ドラッグ&ドロップのイベント処理

**Window レベルのキャプチャフェーズリスナー**（Document や Desktop ではなく window レベル）:
```typescript
window.addEventListener('dragenter', (event) => {
  event.preventDefault();
  event.stopPropagation();
  modalCreated = false;  // 新ドラッグセッション開始のマーク
}, { capture: true });

window.addEventListener('dragover', (event) => {
  event.preventDefault();
  event.stopPropagation();
  
  // Modal が未作成またはクローズされていれば作成
  if (!modalCreated || !systemModalWindowId || !findWindowById(systemModalWindowId)) {
    createSystemModalWindow('import');
    modalCreated = true;
  }
  
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy';
  }
}, { capture: true });

window.addEventListener('dragleave', (event) => {
  event.preventDefault();
  event.stopPropagation();
  
  // viewport 外に完全に出た場合のみ Modal をクローズ
  const clientX = event.clientX;
  const clientY = event.clientY;
  
  if (clientX < 0 || clientX >= window.innerWidth ||
      clientY < 0 || clientY >= window.innerHeight) {
    modalCreated = false;
    if (systemModalWindowId) {
      closeWindow(systemModalWindowId);
    }
  }
}, { capture: true });

window.addEventListener('drop', (event) => {
  event.preventDefault();
  event.stopPropagation();
  modalCreated = false;
  
  const files = event.dataTransfer?.files;
  
  if (!files || files.length === 0) {
    if (systemModalWindowId && findWindowById(systemModalWindowId)) {
      closeWindow(systemModalWindowId);
    }
    return;
  }
  
  // FileList の async 処理のために配列にコピー
  const fileArray = Array.from(files);
  
  // import 非同期実行
  void (async () => {
    const result = await importDroppedFiles(fileArray as FileList);
    
    if (systemModalWindowId && findWindowById(systemModalWindowId)) {
      if (result.ok) {
        systemModalModeByWindowId.set(systemModalWindowId, {
          mode: 'complete',
          filename: result.filename,
        });
      } else {
        systemModalModeByWindowId.set(systemModalWindowId, {
          mode: 'complete',
          error: result.error,
        });
      }
      renderWindows();
    }
  })();
}, { capture: true });
```

**キャプチャフェーズを使用する理由**:
- ブラウザのネイティブドラッグ&ドロップ処理をアプリケーションレベルで優先
- バブリング段階では Modal や他の要素が既に作成され、イベント対象が変わってしまう
- キャプチャフェーズで最初にイベントをキャッチし、ブラウザのデフォルト動作を確実に防止

**複数回ドラッグ対応**:
- dragenter で `modalCreated = false` にリセット → 新セッション検出
- dragover で既存 Modal をチェック → 前回のドラッグで Modal が残存していても再利用または再作成
- これにより複数回のドロップ操作後も常にファイルの直接開きを防止

### 7.4 File Import Processing

**入力**: FileList オブジェクト

**バリデーション**:
- 既存ファイルの総サイズを計算
- 新規ファイルの総サイズを計算
- (既存 + 新規) が 1.5 MiB (1572864 バイト) 以下か確認
- 超過時は詳細情報付きエラー例）`"合計サイズ (0.19 MiB + 1.56 MiB) が 1.5 MiB の制限を超えています"` を返す

**処理流れ**:
1. 全ファイルサイズ計算（既存 + 新規）
2. 制限チェック（超過時はエラー返却）
3. FileList から各ファイルを順序に処理
4. `file.arrayBuffer()` で Uint8Array に変換
5. `normalizePathLikeName()` でファイル名正規化（ハイフン→アンダースコア、大文字化など）
6. `upsertFile(name, content)` でファイルシステムに追加
7. Terminal に "Imported {filename} ({size})" を出力
8. 成功: `{ ok: true, filename: string }` を返す（最初のファイル名）
9. エラー: `{ ok: false, error: string }` を返す

**エラーハンドリング**:
| 条件 | エラーメッセージ例 |
|------|-----------------|
| ファイルサイズ超過 | "取り込みに失敗しました: 合計サイズ (0.19 MiB + 1.56 MiB) が 1.5 MiB の制限を超えています" |
| ファイル読込失敗 | "取り込みに失敗しました: {詳細エラーメッセージ}" |
| ファイルシステム追加失敗 | "取り込みに失敗しました: {詳細エラーメッセージ}" |
| ファイルなし | "取り込み可能なファイルがありません" |

**File System ウィンドウへの通知**:
- import 完了後 `renderWindows()` が呼ばれる
- File System ウィンドウが開いている場合、ファイルリストが自動更新
