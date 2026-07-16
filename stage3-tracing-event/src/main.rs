//! stage3: `tracing` に乗り換える(まずは「イベント」だけ = log の置き換え)。
//!
//! `tracing` は「イベント(event)」と「スパン(span)」の 2 つが柱。
//! この stage ではまだイベントしか使わないので、見た目は log とほぼ同じ。
//! ただし 1 点だけ大きく違う:
//!
//!   info!(order.id = order.id, amount = order.amount, "processing");
//!
//! のように、メッセージとは別に **構造化フィールド(key = value)** を
//! 持たせられる。文字列に埋め込むのではなく「データ」として付くので、
//! 後段(JSON 出力・検索・集計)で機械的に扱える。
//!
//! 一方で「非同期タスクをまたいで文脈を引き回す」うまみはまだ出ていない
//! (それは次の stage4 = span で登場する)。
//!
//! 実行:
//!   cargo run -p stage3-tracing-event
//!   RUST_LOG=debug cargo run -p stage3-tracing-event

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

async fn validate(order: &Order) -> Result<(), String> {
    debug!(order.id, "validate: start");
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        warn!(order.id, reason = "amount is zero", "validate failed");
        return Err("amount is zero".to_string());
    }
    Ok(())
}

async fn reserve_inventory(order: &Order) -> Result<(), String> {
    debug!(order.id, "reserve_inventory: start");
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        warn!(order.id, reason = "out of stock", "reserve_inventory failed");
        return Err("out of stock".to_string());
    }
    Ok(())
}

async fn charge_payment(order: &Order) -> Result<u32, String> {
    debug!(order.id, "charge_payment: start");
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        warn!(order.id, reason = "amount exceeds limit", "charge_payment failed");
        return Err("amount exceeds limit".to_string());
    }
    Ok(order.amount)
}

async fn process_order(order: Order) -> Result<u32, String> {
    // key = value の構造化フィールドを持たせられるのが log との違い
    info!(order.id = order.id, amount = order.amount, "processing");
    validate(&order).await?;
    reserve_inventory(&order).await?;
    let charged = charge_payment(&order).await?;
    info!(order.id = order.id, charged, "done");
    Ok(charged)
}

#[tokio::main]
async fn main() {
    // 最小構成のサブスクライバ。RUST_LOG が未設定なら info を既定にする。
    // (fmt::init() だけだと RUST_LOG 未設定時に error しか出ないので明示する)
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
