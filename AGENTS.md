# AGENTS.md

HariboteBox は、はりぼてOS のアプリをブラウザ上で実行するデスクトップ環境です。TypeScript + Vite の UI と、Web Worker 上の Rust/Wasm x86 エミュレータで構成されます。

## 正本とルール

- 仕様の正本は [design.md](./design.md) と `docs/*.md`。作業前に該当する文書を読む。
- 一定以上の変更は、先に設計書を更新してから実装する。
- `README.md` と Onboarding ウィンドウの文言は人間が保守する。AI は変更しない。
- 不足している情報は、実装前に人間へ確認する。

## コマンド

- 開発サーバー: `pnpm dev`
- テスト: `make test`（`cargo test` と vitest）
- フルビルド: `make full-build`（Rust nightly と `wasm-bindgen-cli` が必要）

`src/generated-initial-fs.ts` と `src/wasm/rust_task*` は生成物なので編集しない。初期ファイルを変えるときは `src/initial-fs/` を編集する。

## 壊しやすい構造

- `src/wm/` はアプリを import しない。ウィンドウ種別は各アプリが `src/wm/registry.ts` に登録する。
- `src/fs/` と `src/audio/` は DOM に触れない。
- ファイルシステムの正本と localStorage への永続化は Main スレッドだけが持つ。Worker は手元のコピーを使う。
- Main ⇄ Worker のメッセージ型は `src/protocol.ts` だけに定義する。
- Rust から呼ぶ FFI 関数を追加・変更するときは、次の 3 つを同時に直す。
  - `rust-task/src/lib.rs` の `#[wasm_bindgen(module = "env")]`
  - `src/wasm/env.ts`
  - `docs/rust-wasm-interface.md`
- Wasm から受け取った `Uint8Array` は Wasm メモリのビューで、呼び出し中しか有効でない。`postMessage` や保持の前にコピーする。

## バグに見えるが仕様のもの

- 初期ファイルは、削除してもリロードで復活する。
- 同名への作成・コピー・リネームは、確認なしで上書きする。
- `HELP` は選定した一部のコマンドだけを表示する（理由は [state-management.md](./docs/state-management.md) の 4.1）。
