//! stage4: `tracing` の主役「スパン(span)」を使う。ここが本題。
//!
//! スパンは「ある処理が動いている期間」を表す。スパンに入っている間に
//! 発行されたイベントには、そのスパンのフィールド(ここでは order.id)が
//! 自動で付く。つまり **もう毎回 order.id を手で書かなくてよい**。
//!
//! 出力例(order.id がスパン名の横に自動で付く):
//!   INFO process_order{order.id=1 amount=100}: stage4...: validating
//!   INFO process_order{order.id=3 amount=500}: stage4...: reserving inventory
//!
//! ★ 非同期での重要な注意点 ★
//!   同期コードなら `let _guard = span.enter();` でスパンに入るが、
//!   これを `.await` をまたいで持つのは NG。タスクが中断して別タスクに
//!   切り替わっても guard が残り、他タスクのイベントに間違ったスパンが
//!   付いてしまう。
//!   非同期では代わりに `future.instrument(span)` を使う。これは
//!   「poll されている間だけスパンに入り、中断中は抜ける」を正しく行う。
//!
//! 実行:
//!   cargo run -p stage4-tracing-span
//!   RUST_LOG=debug cargo run -p stage4-tracing-span

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, info_span, warn, Instrument};

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

// 各サブ処理では order.id を「書かない」。文脈はスパンが運んでくれる。
async fn validate(order: &Order) -> Result<(), String> {
    info!("validating");
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        warn!("amount is zero");
        return Err("amount is zero".to_string());
    }
    Ok(())
}

async fn reserve_inventory(order: &Order) -> Result<(), String> {
    info!("reserving inventory");
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        warn!("out of stock");
        return Err("out of stock".to_string());
    }
    Ok(())
}

async fn charge_payment(order: &Order) -> Result<u32, String> {
    info!("charging payment");
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        warn!("amount exceeds limit");
        return Err("amount exceeds limit".to_string());
    }
    Ok(order.amount)
}

async fn process_order(order: Order) -> Result<u32, String> {
    // この注文の処理全体を包むスパン。フィールドに order.id を持たせる。
    let span = info_span!("process_order", order.id = order.id, amount = order.amount);

    // 非同期処理は enter() ではなく instrument() で包む
    async move {
        info!("start");
        validate(&order).await?;
        reserve_inventory(&order).await?;
        let charged = charge_payment(&order).await?;
        info!(charged, "done");
        Ok(charged)
    }
    .instrument(span)
    .await
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
