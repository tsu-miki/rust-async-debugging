//! stage7: `tokio-console`(console-subscriber)で「ランタイムそのもの」を覗く。
//!
//! ここまでの tracing は「自分のコードが出したログ」を見るものだった。
//! tokio-console はレイヤーが違う。tokio ランタイムに埋め込まれた計装から、
//! **今このランタイムで動いている全タスクの状態** を GUI(TUI)で見せる:
//!
//!   - どのタスクが生きていて、running / idle どちらか
//!   - 各タスクの poll 回数・busy 時間・idle 時間
//!   - 長時間ブロックしている / poll が返ってこないタスク(= 詰まりの発見)
//!   - waker のリークなど
//!
//! 注目すべきは、このファイルには tracing のログ文が 1 つも無いこと。
//! それでも console はすべてのタスクを見せてくれる。「ログを仕込み忘れた
//! 場所」でタスクが固まっても検知できる、というのがログとの決定的な差。
//!
//! ── 使い方 ──────────────────────────────────────────────
//! 1. console 側 CLI を入れる(初回のみ):
//!      cargo install --locked tokio-console
//! 2. このアプリを動かす(このワークスペースは .cargo/config.toml で
//!    tokio_unstable を付けているのでそのまま起動できる):
//!      cargo run -p stage7-tokio-console
//! 3. 別ターミナルで:
//!      tokio-console
//!    既定で 127.0.0.1:6669 の console-subscriber に接続する。
//! 4. Ctrl-C でアプリを終了。
//! ────────────────────────────────────────────────────────

use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

async fn validate(order: &Order) -> Result<(), String> {
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        return Err("amount is zero".to_string());
    }
    Ok(())
}

async fn reserve_inventory(order: &Order) -> Result<(), String> {
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        return Err("out of stock".to_string());
    }
    Ok(())
}

async fn charge_payment(order: &Order) -> Result<u32, String> {
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        return Err("amount exceeds limit".to_string());
    }
    Ok(order.amount)
}

async fn process_order(order: Order) -> Result<u32, String> {
    validate(&order).await?;
    reserve_inventory(&order).await?;
    charge_payment(&order).await
}

#[tokio::main]
async fn main() {
    // これ 1 行で console-subscriber が立ち上がり、127.0.0.1:6669 で待ち受ける。
    console_subscriber::init();

    println!("running... attach with `tokio-console` in another terminal.");
    println!("press Ctrl-C to stop.");

    // 注文バッチを 1 秒ごとに投げ続ける常駐タスク。
    // タスクに名前を付けておくと console 上で識別しやすい(名前付けは unstable API)。
    let worker = tokio::task::Builder::new()
        .name("order-worker")
        .spawn(async {
            let mut round = 0u32;
            loop {
                round += 1;
                let orders = vec![
                    Order { id: 1, amount: 100 },
                    Order { id: 2, amount: 0 },
                    Order { id: 3, amount: 500 },
                    Order { id: 4, amount: 5000 },
                    Order { id: 5, amount: 250 },
                ];
                for order in orders {
                    // 1 注文 = 1 タスク。console にはこれらが次々現れて消える。
                    let _ = tokio::task::Builder::new()
                        .name("process-order")
                        .spawn(process_order(order));
                }
                println!("dispatched batch #{round}");
                sleep(Duration::from_secs(1)).await;
            }
        })
        .expect("failed to spawn worker");

    // わざと 1 時間眠り続けるだけのタスク。
    // console 上で「ずっと idle なタスク」がどう見えるかの観察用。
    let sleeper = tokio::task::Builder::new()
        .name("lazy-sleeper")
        .spawn(async {
            loop {
                sleep(Duration::from_secs(3600)).await;
            }
        })
        .expect("failed to spawn sleeper");

    // Ctrl-C が来るまで走り続ける
    tokio::signal::ctrl_c().await.expect("failed to listen for ctrl-c");
    println!("\nshutting down...");
    worker.abort();
    sleeper.abort();
}
