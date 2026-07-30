# HariboteBox 設計書

このディレクトリには、HariboteBox プロジェクトの設計書が含まれています。

## 📚 ドキュメント構成

設計書は以下のトピック別に分割されています。各ファイルは役割ごとの一次情報を持ち、必要に応じて相互参照リンクで補完する構成です。

### 1. [プロジェクト概要](./docs/overview.md)
- プロジェクトの目的と適用範囲
- 技術スタック
- 用語定義
- 変更管理ルール

### 2. [UI設計](./docs/ui-design.md)
- 画面構成（デスクトップ、タスクバー）
- スタートメニューとボリュームコントロール
- ウィンドウシステムの仕様
- 各ウィンドウタイプの詳細
  - 端末ウィンドウ
  - Canvas ウィンドウ
  - ファイルマネージャ
  - About ウィンドウ
  - テキストビューア
  - Onboarding ウィンドウ

### 3. [状態管理とイベント処理](./docs/state-management.md)
- AppState モデル
- 実装定数と環境変数
- ターミナル履歴管理
- アクティブウィンドウシステム
- 端末コマンド仕様
- タスクバーウィンドウ一覧機能

### 4. [ファイルシステム](./docs/filesystem.md)
- 基本仕様
- ファイル名正規化ルール
- 初期ファイルと永続化
- 複数Worker対応
- ファイルシステム同期フロー

### 5. [Rust/Wasmインターフェース](./docs/rust-wasm-interface.md)
- Rustエントリポイント関数
- FFIインターフェース仕様
  - ウィンドウ管理
  - Canvas描画
  - ファイルI/O
  - イベント処理
- WorkerとMainのメッセージプロトコル
- キーボード入力処理
- イベントキューとOsContext

### 6. [仕様確認とテスト基準](./docs/specifications.md)
- 実装時の仕様確認リスト（17カテゴリ）
- テスト基準
- 非機能要件

### 7. [未実装項目と将来の課題](./docs/future-work.md)
- 確認待ち事項
- 未実装機能（TBD項目）
- 将来の課題
  - スマホ対応
  - Rust/Wasm機能拡張
  - キーボード入力拡張
  - ウィンドウ機能拡張

## 🗺️ ドキュメントの読み方

### 初めて読む場合
1. まず [プロジェクト概要](./docs/overview.md) を読んで全体像を把握
2. 実装する機能に応じて必要なセクションを参照

### 実装時
- UI実装: [UI設計](./docs/ui-design.md) → [状態管理](./docs/state-management.md)
- ファイル操作: [ファイルシステム](./docs/filesystem.md) → [状態管理](./docs/state-management.md)
- Rust連携: [Rust/Wasmインターフェース](./docs/rust-wasm-interface.md) → [状態管理](./docs/state-management.md)

### 一次情報の見分け方
- 現行仕様の正本: 各トピック別ファイル（`overview.md`, `ui-design.md`, `state-management.md`, `filesystem.md`, `rust-wasm-interface.md`, `specifications.md`）
- `future-work.md`: 未実装項目と将来課題の整理用。現行仕様の再定義は行わない

### レビュー・テスト時
- [仕様確認とテスト基準](./docs/specifications.md) を使用して実装を検証

### 新機能検討時
- [未実装項目と将来の課題](./docs/future-work.md) を確認

## 📝 更新履歴

- 2026-07-30: 設計書を7つのトピック別ファイルに分割（元は単一の `design.md`）
- 2026-07-30: README に一次情報の案内を追加し、仕様確認リストの説明を補正

## 🔗 関連ドキュメント

- [プロジェクトREADME](./README.md) - ユーザー向けドキュメント
- [ライセンス](./LICENSE) - ライセンス情報
