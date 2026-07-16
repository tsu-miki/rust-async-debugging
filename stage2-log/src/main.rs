//! stage2: `log` ファサード + `env_logger` バックエンドを入れる。
//!
//! println からの前進:
//!   - ログレベル(error/warn/info/debug/trace)が付く。
//!   - `RUST_LOG` 環境変数で実行時に出力を絞れる。コード変更・再ビルド不要。
//!   - タイムスタンプ・レベル・モジュールパスが自動で付く。
//!
//! まだ残るつらさ:
//!   - ログはあくまで「1 行のテキスト」。どの注文かは相変わらず手で
//!     `order.id` を埋め込む必要がある(構造化されていない)。
//!   - 非同期タスクをまたぐ「文脈(どのリクエストの処理中か)」を
//!     自動で引き回す仕組みがない。
//!
//! 実行:
//!   cargo run -p stage2-log                        # 既定(info 以上)
//!   RUST_LOG=debug cargo run -p stage2-log         # debug まで表示
//!   RUST_LOG=warn cargo run -p stage2-log          # warn 以上だけ
//!   RUST_LOG=stage2_log=debug cargo run -p stage2-log  # このクレートだけ debug

use log::{debug, info, warn};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

async fn validate(order: &Order) -> Result<(), String> {
    debug!("[order {}] validate: start", order.id);
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        warn!("[order {}] validate failed: amount is zero", order.id);
        return Err("amount is zero".to_string());
    }
    Ok(())
}

async fn reserve_inventory(order: &Order) -> Result<(), String> {
    debug!("[order {}] reserve_inventory: start", order.id);
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        warn!("[order {}] reserve_inventory failed: out of stock", order.id);
        return Err("out of stock".to_string());
    }
    Ok(())
}

async fn charge_payment(order: &Order) -> Result<u32, String> {
    debug!("[order {}] charge_payment: start", order.id);
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        warn!("[order {}] charge_payment failed: amount exceeds limit", order.id);
        return Err("amount exceeds limit".to_string());
    }
    Ok(order.amount)
}

async fn process_order(order: Order) -> Result<u32, String> {
    info!("[order {}] processing (amount={})", order.id, order.amount);
    validate(&order).await?;
    reserve_inventory(&order).await?;
    let charged = charge_payment(&order).await?;
    info!("[order {}] done (charged={})", order.id, charged);
    Ok(charged)
}

#[tokio::main]
async fn main() {
    // RUST_LOG が未設定なら info を既定にする
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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

    info!("results = {results:?}");
}
