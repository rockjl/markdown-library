use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

// ---- 调试日志输出（仅写入 voice_debug.log，无 stdout） ----
fn log_msg(msg: String) {
    use std::sync::Mutex;
    static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);
    if let Ok(mut guard) = FILE.lock() {
        if guard.is_none() {
            *guard = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open("voice_debug.log")
                .ok();
        }
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "{}", msg);
            let _ = f.flush();
        }
    }
}

fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}
// ---- end ----

/// 线程安全的转写结果队列，供 SEND/RECV 双任务与 UI 轮询线程共享
type TranscriptQueue = Arc<Mutex<VecDeque<String>>>;

/// 语音引擎：连接讯飞 RTASR WebSocket，采集麦克风音频，
/// 发送音频流并接收转写结果，通过 poll() 方法供 UI 层轮询。
pub struct VoiceEngine {
    queue: TranscriptQueue,
    running: Arc<AtomicBool>,
}

impl VoiceEngine {
    /// 启动语音引擎
    ///
    /// 创建一个新的 OS 线程，内部运行 tokio 运行时，
    /// 依次完成：鉴权 → WebSocket 连接 → 麦克风初始化 → 双任务收发音频。
    pub fn start(appid: &str, secret_key: &str) -> Self {
        log_msg(format!("[{}] VoiceEngine::start()", timestamp()));
        let queue: TranscriptQueue = Arc::new(Mutex::new(VecDeque::new()));
        let q2 = queue.clone();
        let running = Arc::new(AtomicBool::new(true));
        let r2 = running.clone();

        let appid = appid.to_string();
        let secret_key = secret_key.to_string();
        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(_) => return,
            };
            rt.block_on(run_loop(appid, secret_key, q2, r2));
        });

        Self { queue, running }
    }

    /// 停止语音引擎
    pub fn stop(&self) {
        log_msg(format!("[{}] VoiceEngine::stop()", timestamp()));
        self.running.store(false, Ordering::Relaxed);
    }

    /// 轮询转写结果（线程安全，非阻塞）
    ///
    /// 每次调用弹出最早的一条转写文本。
    /// 如果 Mutex 中毒，则自动恢复并继续返回数据。
    pub fn poll(&self) -> Option<String> {
        let mut guard = match self.queue.lock() {
            Ok(g) => g,
            Err(e) => return e.into_inner().pop_front(),
        };
        guard.pop_front()
    }

    /// 检查引擎是否运行中
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

/// 异步主循环：鉴权 → WebSocket 连接 → 麦克风 → 双任务发送/接收
async fn run_loop(
    appid: String,
    secret_key: String,
    queue: TranscriptQueue,
    running: Arc<AtomicBool>,
) {
    // ---- 鉴权 ----
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let signa = build_signa(&appid, &ts, &secret_key);
    let encoded = url::form_urlencoded::byte_serialize(signa.as_bytes()).collect::<String>();
    let url_str = format!(
        "wss://rtasr.xfyun.cn/v1/ws?appid={}&ts={}&signa={}&ent=en&pgs=1",
        appid, ts, encoded
    );

    // ---- WebSocket 连接 ----
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut request = url_str.into_client_request().unwrap();
    request.headers_mut().insert(
        tokio_tungstenite::tungstenite::http::header::ORIGIN,
        tokio_tungstenite::tungstenite::http::HeaderValue::from_static(
            "https://rtasr.xfyun.cn/v1/ws",
        ),
    );
    let (ws, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(r) => {
            log_msg(format!("[{}] WS connected", timestamp()));
            r
        }
        Err(e) => {
            log_msg(format!("[{}] WS connect FAILED: {}", timestamp(), e));
            return;
        }
    };
    let (mut write, mut read) = ws.split();

    // ---- 麦克风 ----
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            log_msg(format!("[{}] No mic found", timestamp()));
            return;
        }
    };
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            log_msg(format!("[{}] Mic config FAILED: {}", timestamp(), e));
            return;
        }
    };
    let dev_rate = config.sample_rate().0;
    let dev_channels = config.channels() as usize;
    let target_rate = 16000u32;

    let (audio_tx, mut audio_rx) = tokio::sync::mpsc::unbounded_channel::<i16>();

    use cpal::SampleFormat;
    let err_fn = move |e| {
        log_msg(format!("[{}] Mic stream error: {}", timestamp(), e));
    };
    let stream: cpal::Stream = match config.sample_format() {
        SampleFormat::F32 => device
            .build_input_stream::<f32, _, _>(
                &config.into(),
                move |data, _| {
                    for frame in data.chunks(dev_channels) {
                        let mono: f32 = if dev_channels == 1 {
                            frame[0]
                        } else {
                            frame.iter().sum::<f32>() / dev_channels as f32
                        };
                        let sample =
                            (mono * i16::MAX as f32).clamp(-32768.0, 32767.0) as i16;
                        let _ = audio_tx.send(sample);
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
        SampleFormat::I16 => device
            .build_input_stream::<i16, _, _>(
                &config.into(),
                move |data, _| {
                    for frame in data.chunks(dev_channels) {
                        let mono: i16 = if dev_channels == 1 {
                            frame[0]
                        } else {
                            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                            (sum / dev_channels as i32) as i16
                        };
                        let _ = audio_tx.send(mono);
                    }
                },
                err_fn,
                None,
            )
            .unwrap(),
        _ => {
            log_msg(format!("[{}] Unsupported audio format", timestamp()));
            return;
        }
    };
    if let Err(e) = stream.play() {
        log_msg(format!("[{}] Stream play FAILED: {}", timestamp(), e));
        return;
    }
    log_msg(format!("[{}] Mic started ({}Hz, {}ch)", timestamp(), dev_rate, dev_channels));

    // ---- 双任务：发送音频 + 接收转写 ----
    let ratio = dev_rate as f64 / target_rate as f64;
    let chunk_target = 640usize;

    let (stop_tx, stop_rx) = tokio::sync::watch::channel(());
    let q3 = queue.clone();

    // ===== 发送任务：降采样麦克风音频 → 每 40ms 向 WS 发送 640 个样本 =====
    let mut stop_rx_send = stop_rx.clone();
    let send_handle = tokio::spawn(async move {
        let mut sample_buf: Vec<i16> = Vec::new();
        let mut decimated_buf: Vec<i16> = Vec::new();
        let mut frac: f64 = 0.0;
        let mut send_interval = tokio::time::interval(Duration::from_millis(40));

        loop {
            tokio::select! {
                _ = send_interval.tick() => {
                    while let Ok(s) = audio_rx.try_recv() {
                        sample_buf.push(s);
                    }

                    let mut i = 0;
                    while i < sample_buf.len() {
                        decimated_buf.push(sample_buf[i]);
                        frac += ratio;
                        let skip = frac as usize;
                        if skip > 0 {
                            i += skip;
                            frac -= skip as f64;
                        } else {
                            i += 1;
                        }
                    }
                    sample_buf.clear();

                    let send_end = decimated_buf.len() / chunk_target * chunk_target;
                    for chunk in decimated_buf[..send_end].chunks(chunk_target) {
                        let mut bytes = Vec::with_capacity(chunk.len() * 2);
                        for &s in chunk {
                            bytes.extend_from_slice(&s.to_le_bytes());
                        }
                        if let Err(_) = write
                            .send(tokio_tungstenite::tungstenite::Message::Binary(bytes))
                            .await
                        {
                            return;
                        }
                    }
                    decimated_buf = decimated_buf[send_end..].to_vec();
                }
                _ = stop_rx_send.changed() => {
                    let _ = write
                        .send(tokio_tungstenite::tungstenite::Message::Binary(
                            b"{\"end\": true}".to_vec(),
                        ))
                        .await;
                    return;
                }
            }
        }
    });

    // ===== 接收任务：读取 WS 响应 → 提取文本 → 推入结果队列 =====
    let mut stop_rx_recv = stop_rx.clone();
    let recv_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = read.next() => {
                    if msg.is_none() {
                        return;
                    }
                    match msg.unwrap() {
                        Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                            if let Some(display) = extract_text(&text) {
                                match q3.lock() {
                                    Ok(mut q) => q.push_back(display),
                                    Err(e) => { e.into_inner().push_back(display); }
                                }
                            }
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => return,
                        Ok(tokio_tungstenite::tungstenite::Message::Ping(_)) => {}
                        Ok(tokio_tungstenite::tungstenite::Message::Pong(_)) => {}
                        Ok(_) => {}
                        Err(_) => return,
                    }
                }
                _ = stop_rx_recv.changed() => {
                    let timeout = tokio::time::sleep(Duration::from_millis(1500));
                    tokio::pin!(timeout);
                    tokio::select! {
                        msg = read.next() => {
                            if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) = msg {
                                if let Some(display) = extract_text(&text) {
                                    match q3.lock() {
                                        Ok(mut q) => q.push_back(display),
                                        Err(e) => { e.into_inner().push_back(display); }
                                    }
                                }
                            }
                        }
                        _ = &mut timeout => {}
                    }
                    return;
                }
            }
        }
    });

    // ===== 等待 UI 层发出停止信号 =====
    while running.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    log_msg(format!("[{}] Stopping, waiting for tasks...", timestamp()));

    drop(stop_tx);
    let _ = send_handle.await;
    let _ = recv_handle.await;
    log_msg(format!("[{}] Voice engine stopped", timestamp()));
}

/// 构建讯飞鉴权签名
///
/// 流程：MD5(appid + ts) → HMAC-SHA1(scret_key, md5_hex) → Base64
fn build_signa(appid: &str, ts: &str, secret_key: &str) -> String {
    use hmac::Mac;
    use sha1::digest::Digest;
    let input = format!("{}{}", appid, ts);
    let result = md5::Md5::digest(input.as_bytes());
    let md5_hex = format!("{:x}", result);

    let mut mac =
        hmac::Hmac::<sha1::Sha1>::new_from_slice(secret_key.as_bytes()).expect("HMAC key");
    mac.update(md5_hex.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    base64::engine::general_purpose::STANDARD.encode(code_bytes)
}

/// 从讯飞 JSON 响应中提取转写文本
///
/// 响应结构：action="result" → data → cn → st → rt[] → ws[] → cw[] → w
fn extract_text(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v.get("action")?.as_str()? != "result" {
        return None;
    }
    let data_raw = v.get("data")?;
    let data_str = data_raw.as_str()?;
    let data: serde_json::Value = serde_json::from_str(data_str).ok()?;

    let mut out = String::new();
    let rt = data.get("cn")?.get("st")?.get("rt")?.as_array()?;
    for item in rt {
        let ws_arr = item.get("ws")?.as_array()?;
        for ws_item in ws_arr {
            let cw_arr = ws_item.get("cw")?.as_array()?;
            for cw_item in cw_arr {
                if let Some(w) = cw_item.get("w")?.as_str() {
                    out.push_str(w);
                }
            }
        }
    }
    Some(out)
}
