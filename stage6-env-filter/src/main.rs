//! stage6: `EnvFilter` で「出す/出さない」を実行時に細かく制御する。
//!
//! これまでは `fmt::init()` 任せだったが、サブスクライバを自分で組み立てて
//! `EnvFilter` を差し込むと、`RUST_LOG` で非常に細かい制御ができる:
//!
//!   - レベル指定:            RUST_LOG=info
//!   - ターゲット(モジュール)別: RUST_LOG=warn,stage6_env_filter=debug
//!   - スパン名でフィルタ:       RUST_LOG="[process_order]=debug"
//!   - 特定フィールドでフィルタ:  RUST_LOG="[process_order{order.id=3}]=trace"
//!
//! 最後の 2 つが tracing ならでは。「注文 id=3 の処理だけ trace まで出す」
//! のような、非同期のノイズだらけのログから狙った文脈だけを抜き出す芸当が
//! できる。これが log には無い強み。
//!
//! さらに `.with_target(true).with_line_number(true)` のように
//! 出力フォーマットも自分で決められる。
//!
//! 実行:
//!   cargo run -p stage6-env-filter
//!   RUST_LOG=warn,stage6_env_filter=debug cargo run -p stage6-env-filter
//!   RUST_LOG="[process_order{order.id=3}]=trace" cargo run -p stage6-env-filter

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, instrument, trace};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

#[instrument(skip(order), fields(order.id = order.id), err)]
async fn validate(order: &Order) -> Result<(), String> {
    trace!("checking amount");
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        return Err("amount is zero".to_string());
    }
    Ok(())
}

#[instrument(skip(order), fields(order.id = order.id), err)]
async fn reserve_inventory(order: &Order) -> Result<(), String> {
    debug!("reserving inventory");
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        return Err("out of stock".to_string());
    }
    Ok(())
}

#[instrument(skip(order), fields(order.id = order.id), err)]
async fn charge_payment(order: &Order) -> Result<u32, String> {
    debug!("charging payment");
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
    // RUST_LOG が未設定なら "info" を既定にする。
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // サブスクライバを Layer 方式で自前に組み立てる。
    // Layer を重ねれば「fmt で標準出力」「別 Layer で JSON をファイルへ」
    // 「別 Layer で OpenTelemetry へ」…と多段に出力先を足していける。
    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_line_number(true),
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
