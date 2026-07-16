//! stage1: まず `println!` を撒いてみる。
//!
//! 「とりあえず print デバッグ」の段階。中の動きが見えるようにはなるが、
//! 非同期ならではのつらさがすぐ出てくる:
//!
//!   1. 複数の注文が並行で動くので、出力が入り混じる(インターリーブ)。
//!   2. どの行がどの注文のものか分からないので、毎回 `order.id` を
//!      手で文字列に埋め込む羽目になる。書き忘れると意味不明な行が残る。
//!   3. ログレベルの概念がないので、本番で黙らせる/絞ることができない。
//!   4. タイムスタンプもないので所要時間も分からない。
//!
//! 実行:
//!   cargo run -p stage1-println

use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
struct Order {
    id: u32,
    amount: u32,
}

async fn validate(order: &Order) -> Result<(), String> {
    // ↓ どの注文か分かるように、毎回 id を手で埋め込む必要がある
    println!("[order {}] validate: start", order.id);
    sleep(Duration::from_millis(10)).await;
    if order.amount == 0 {
        println!("[order {}] validate: FAILED (amount is zero)", order.id);
        return Err("amount is zero".to_string());
    }
    println!("[order {}] validate: ok", order.id);
    Ok(())
}

async fn reserve_inventory(order: &Order) -> Result<(), String> {
    println!("[order {}] reserve_inventory: start", order.id);
    sleep(Duration::from_millis(30)).await;
    if order.id == 3 {
        println!("[order {}] reserve_inventory: FAILED (out of stock)", order.id);
        return Err("out of stock".to_string());
    }
    println!("[order {}] reserve_inventory: ok", order.id);
    Ok(())
}

async fn charge_payment(order: &Order) -> Result<u32, String> {
    println!("[order {}] charge_payment: start", order.id);
    sleep(Duration::from_millis(50)).await;
    if order.amount > 1000 {
        println!("[order {}] charge_payment: FAILED (amount exceeds limit)", order.id);
        return Err("amount exceeds limit".to_string());
    }
    println!("[order {}] charge_payment: ok ({})", order.id, order.amount);
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

    println!("---");
    println!("results = {results:?}");
}
