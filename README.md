# Rust の非同期デバッグツールを段階的に学ぶ

κeen さんのスライド
[「Rustの非同期デバッグツールを使いこなせ!」](https://keens.github.io/slide/rustnohidoukidebaggutsu_ruwotsukaikonase_/)
の内容を、**学習用に「何もしていない状態」から 1 つずつツールを足していく**
形で実装したワークスペースです。

同じ題材(非同期の注文処理を並行実行し、注文ごとに別のステップで失敗する)を
各 stage で使い回し、**デバッグ手段だけ**を段階的に差し替えています。
`stageN` と `stageN+1` を `diff` すると「何を足したら何が便利になったか」が
一目で分かるようになっています。

## 題材

`process_order` は 1 件の注文を 3 ステップで処理します(いずれも `.await` する非同期関数):

1. `validate` … 金額が 0 なら失敗
2. `reserve_inventory` … 在庫確保。id=3 は在庫切れで失敗
3. `charge_payment` … 決済。金額が上限超過なら失敗

これを 5 件、`tokio::spawn` で**並行**に流します。狙いは非同期デバッグの
「つらい」を体験できるようにすること:

- 出力が入り混じる(どのログがどの注文のものか分からない)
- どのステップで・なぜ失敗したかを追いにくい
- タスクが詰まっても気づけない

| 注文 | 金額  | 結果                       |
|------|-------|----------------------------|
| 1    | 100   | 成功                       |
| 2    | 0     | `validate` で失敗          |
| 3    | 500   | `reserve_inventory` で失敗 |
| 4    | 5000  | `charge_payment` で失敗    |
| 5    | 250   | 成功                       |

## 段階(stage)一覧

| stage | 足したもの | 何が便利になったか |
|-------|-----------|--------------------|
| **0** | (なし) | 何もなし。動くが中は完全に見えない。最終結果の `Vec` しか手がかりがない |
| **1** | `println!` | 中の動きは見える。が、出力が入り混じり、毎回 `order.id` を手で埋め込む必要がある。レベルもタイムスタンプも無い |
| **2** | `log` + `env_logger` | ログレベルと `RUST_LOG` フィルタ、タイムスタンプが付く。でも「1 行のテキスト」のままで、文脈は手作業 |
| **3** | `tracing` + `tracing-subscriber`(イベントのみ) | `key = value` の**構造化フィールド**が持てる。まだ log 相当だが後段で機械処理できる |
| **4** | `tracing` の **span** | 「この注文の処理中」という文脈を span が自動で運ぶ。**もう `order.id` を毎回書かなくてよい**。非同期では `.instrument()` を使うのが肝 |
| **5** | `#[tracing::instrument]` | span 生成・引数記録・エラー記録を**属性 1 つ**で自動化。ネストで「どの注文の・どのステップか」が階層表示に |
| **6** | `EnvFilter`(細かいフィルタ)+ Layer 構成 | 「id=3 の処理だけ trace」等、**span 名やフィールド単位**で狙い撃ち。Layer を重ねて出力先も多段化できる |
| **7** | `tokio-console`(`console-subscriber`) | ログではなく**ランタイムのタスク状態**を TUI で観察。ログを仕込んでいない箇所で詰まっても検知できる |

## 実行方法

```bash
# 何もしない状態 → 順に見ていく
cargo run -p stage0-nothing
cargo run -p stage1-println
cargo run -p stage2-log
cargo run -p stage3-tracing-event
cargo run -p stage4-tracing-span
cargo run -p stage5-instrument
cargo run -p stage6-env-filter
```

`RUST_LOG` を変えて挙動の違いを試すのがおすすめです:

```bash
RUST_LOG=debug cargo run -p stage2-log
RUST_LOG=warn,stage6_env_filter=debug cargo run -p stage6-env-filter

# tracing ならではの「フィールド単位フィルタ」。注文 id=3 の処理だけ trace まで出す:
RUST_LOG="[process_order{order.id=3}]=trace" cargo run -p stage6-env-filter
```

### stage7(tokio-console)だけは別手順

```bash
# 1. console 側 CLI を入れる(初回のみ)
cargo install --locked tokio-console

# 2. アプリを起動(Ctrl-C で停止するまで走り続ける)
cargo run -p stage7-tokio-console

# 3. 別ターミナルで console を起動して接続(既定 127.0.0.1:6669)
tokio-console
```

console では、動いている全タスク・その running/idle 状態・poll 回数・
busy/idle 時間などが一覧できます。わざと 1 時間眠り続ける `lazy-sleeper`
タスクも仕込んであるので、「ずっと idle なタスク」の見え方も観察できます。

## 各 stage のねらい(コメント参照)

各 `src/main.rs` の冒頭に、その stage で「何を足し、何が良くなり、何がまだ
つらいか」を日本語コメントで書いています。まず `stage0` → `stage1` と順に
ソースを読み、`diff` を取りながら進めると理解しやすいです。

```bash
diff stage3-tracing-event/src/main.rs stage4-tracing-span/src/main.rs
diff stage4-tracing-span/src/main.rs  stage5-instrument/src/main.rs
```

## 補足: tokio_unstable フラグについて

`tokio-console` は tokio 本体のタスク計装(`--cfg tokio_unstable` が必要)を
使います。毎回環境変数を付ける手間を省くため、ワークスペースの
`.cargo/config.toml` でこのフラグを付けています。stage0〜stage6 にとっては
無害(未使用の cfg が増えるだけ)です。

## 使用クレート

- [`tracing`](https://github.com/tokio-rs/tracing) / `tracing-subscriber`
- [`tokio-console`](https://github.com/tokio-rs/console) / `console-subscriber`
- `log` / `env_logger`
- `tokio`

## 参考

- スライド: [Rustの非同期デバッグツールを使いこなせ!](https://keens.github.io/slide/rustnohidoukidebaggutsu_ruwotsukaikonase_/)
- [Tokio: Getting started with Tracing](https://tokio.rs/tokio/topics/tracing)
- [Tokio: Next steps with Tracing](https://tokio.rs/tokio/topics/tracing-next-steps)
