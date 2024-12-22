use axum::{
    routing::{post, get},
    Router,
    http::StatusCode,
    extract::State,
    Json,
    serve,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::Value;
use std::process::Command;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use bytes::Bytes;
use std::env;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct AppState {
    webhook_secret: String,
}

#[tokio::main]
async fn main() {
    println!("正在启动服务器...");
    
    // 从环境变量获取 webhook secret
    let webhook_secret = env::var("WEBHOOK_SECRET").unwrap_or_else(|_| {
        println!("警告: 未设置 WEBHOOK_SECRET 环境变量，使用空字符串作为默认值");
        String::new()
    });

    let state = Arc::new(AppState {
        webhook_secret,
    });
    println!("Webhook secret 已配置");

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .route("/", get(|| async { "服务器正在运行" }))
        .with_state(state);
    println!("路由已配置");

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("正在绑定地址 {}...", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("服务器启动成功！");
    println!("本地地址: http://localhost:3000");
    println!("Webhook 地址: http://localhost:3000/webhook");
    println!("等待请求中...");
    
    serve(listener, app)
        .await
        .unwrap();
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Result<(), StatusCode> {
    println!("收到 webhook 请求");
    println!("收到的所有请求头: {:?}", headers);
    
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            println!("缺少签名头");
            println!("可用的请求头: {:?}", headers.keys().collect::<Vec<_>>());
            StatusCode::BAD_REQUEST
        })?;

    println!("收到的签名: {}", signature);
    println!("使用的 secret: {}", state.webhook_secret);

    let mut mac = HmacSha256::new_from_slice(state.webhook_secret.as_bytes())
        .map_err(|e| {
            println!("HMAC 初始化失败: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 使用原始请求体计算签名
    mac.update(&body);
    let result = mac.finalize();
    let computed_signature = format!("sha256={}", hex::encode(result.into_bytes()));

    println!("计算的签名: {}", computed_signature);
    println!("原始 payload: {}", String::from_utf8_lossy(&body));

    if signature != computed_signature {
        println!("签名不匹配");
        println!("收到的签名: {}", signature);
        println!("计算的签名: {}", computed_signature);
        return Err(StatusCode::UNAUTHORIZED);
    }

    println!("签名验证成功！");

    // 解析 JSON
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|e| {
            println!("JSON 解析失败: {:?}", e);
            StatusCode::BAD_REQUEST
        })?;

    let pull_output = Command::new("git")
        .current_dir("/www/hugo")
        .arg("pull")
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !pull_output.status.success() {
        eprintln!("Git pull 失败: {:?}", String::from_utf8_lossy(&pull_output.stderr));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let hugo_output = Command::new("hugo")
        .current_dir("/www/hugo")
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !hugo_output.status.success() {
        eprintln!("Hugo 构建失败: {:?}", String::from_utf8_lossy(&hugo_output.stderr));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    println!("所有命令执行成功！");
    Ok(())
}
