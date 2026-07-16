//! stage0: 何もしていない状態。
//!
//! 非同期の注文処理をいくつか並行で走らせるだけ。
//! ログもトレースも一切ないので、動きはするが「中で何が起きているか」は
//! まったく見えない。失敗しても、どの注文が・どのステップで・なぜ落ちたのか
//! を追う手がかりが `main` が最後に集めた Result 一覧しかない。
//!
//! 実行:
//!   cargo run -p stage0-nothing

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
    let orders = vec![
        Order { id: 1, amount: 100 },
        Order { id: 2, amount: 0 },    // validate で失敗する
        Order { id: 3, amount: 500 },  // reserve_inventory で失敗する
        Order { id: 4, amount: 5000 }, // charge_payment で失敗する
        Order { id: 5, amount: 250 },
    ];

    // 5 件を並行で処理する
    let mut handles = Vec::new();
    for order in orders {
        handles.push(tokio::spawn(process_order(order)));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // 得られるのはこの一覧だけ。
    // [Ok(100), Err("amount is zero"), Err("out of stock"), Err("amount exceeds limit"), Ok(250)]
    // 「どの Err がどの注文か」すら位置で推測するしかなく、
    // 処理の順序・所要時間・途中経過はまったく分からない。
    println!("{results:?}");
}
