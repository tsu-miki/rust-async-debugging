//! stage5: 手作業のスパンを `#[tracing::instrument]` で自動化する。
//!
//! stage4 の `info_span!` + `.instrument()` は正しいが、関数ごとに書くと
//! ボイラープレートが多い。`#[instrument]` を関数に付けるだけで:
//!
//!   - 関数と同名のスパンが自動生成される
//!   - 引数が自動でフィールドとして記録される(`skip` で除外も可能)
//!   - async 関数でも「poll 中だけスパンに入る」を正しくやってくれる
//!   - `err` を付けると、Err を返したときに自動で error イベントを出す
//!
//! さらにネストしたサブ関数もそれぞれ instrument すると、
//! スパンが入れ子になり「どの注文の・どのステップか」が階層で分かる:
//!
//!   INFO process_order{order.id=3}:reserve_inventory{order.id=3}: reserving
//!   ERROR process_order{order.id=3}:reserve_inventory{order.id=3}: ... err=out of stock
//!
//! 実行:
//!   cargo run -p stage5-instrument
//!   RUST_LOG=debug cargo run -p stage5-instrument

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, instrument};

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

// order 全体は Debug 出力すると大きいので skip し、必要な id だけフィールドに出す。
// `err` により Err 時は自動で error レベルのイベントが出る。
#[instrument(skip(order), fields(order.id = order.id), err)]
async fn validate(order: &Order) -> Result<(), String> {
    info!("validating");
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        return Err("amount is zero".to_string());
    }
    Ok(())
}

#[instrument(skip(order), fields(order.id = order.id), err)]
async fn reserve_inventory(order: &Order) -> Result<(), String> {
    info!("reserving inventory");
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        return Err("out of stock".to_string());
    }
    Ok(())
}

#[instrument(skip(order), fields(order.id = order.id), err)]
async fn charge_payment(order: &Order) -> Result<u32, String> {
    info!("charging payment");
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        return Err("amount exceeds limit".to_string());
    }
    Ok(order.amount)
}

#[instrument(skip(order), fields(order.id = order.id, amount = order.amount), err)]
async fn process_order(order: Order) -> Result<u32, String> {
    validate(&order).await?;
    reserve_inventory(&order).await?;
    let charged = charge_payment(&order).await?;
    info!(charged, "order completed");
    Ok(charged)
}

#[tokio::main]
async fn main() {
    // RUST_LOG が未設定なら info を既定にする。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let orders = vec![
        Order { id: 1, amount: 100 },
        Order { id: 2, amount: 0 },
        Order { id: 3, amount: 500 },
        Order { id: 4, amount: 5000 },
        Order { id: 5, amount: 250 },
    ];

    let mut handles = Vec::new();
    for order in orders {
        handles.push(tokio::spawn(process_order(order)));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    info!(?results, "all orders finished");
}
