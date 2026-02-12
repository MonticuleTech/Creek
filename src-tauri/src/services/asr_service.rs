use tokio::sync::mpsc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use serde_json::json;
use uuid::Uuid;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use log::{info, error, debug};
use tokio_util::sync::CancellationToken;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const QWEN_MODEL: &str = "qwen3-asr-flash-realtime";
const WS_URL: &str = "wss://dashscope.aliyuncs.com/api-ws/v1/realtime";

#[derive(Clone)]
pub struct AsrService {
    api_key: String,
    tx_sender: Option<mpsc::UnboundedSender<String>>, 
}

impl AsrService {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            tx_sender: None,
        }
    }

    pub fn set_callback(&mut self, tx: mpsc::UnboundedSender<String>) {
        self.tx_sender = Some(tx);
    }

    pub async fn start_recording(&self, cancel_token: CancellationToken, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let api_key = self.api_key.clone();
        let sender = self.tx_sender.clone();

        // 1. Setup Audio Input (cpal)
        let host = cpal::default_host();
        
        info!("Available hosts: {:?}", cpal::available_hosts());
        
        let input_device = host.default_input_device()
            .ok_or("No input device available")?;
            
        #[allow(deprecated)]
        let device_name = input_device.name().unwrap_or("unknown".into());
        info!("Default Input device: {}", device_name);
        
        // List all input devices for debugging
        if let Ok(devices) = host.input_devices() {
            for (i, dev) in devices.enumerate() {
                #[allow(deprecated)]
                let name = dev.name().unwrap_or_else(|_| "unknown".into());
                info!("Device {}: {}", i, name);
            }
        }

        let config = input_device.default_input_config()?;
        
        #[allow(deprecated)]
        let device_name_2 = input_device.name().unwrap_or("unknown".into());
        info!("Input device: {}", device_name_2);
        info!("Default config: {:?}", config);
        
        // Spawn async setup to avoid blocking
        tokio::spawn(async move {
            if let Err(e) = Self::run_async_recording(api_key, sender, input_device, config, cancel_token, app_handle).await {
                error!("Async recording error: {}", e);
            }
        });
        
        Ok(())
    }

    async fn run_async_recording(
        api_key: String, 
        sender: Option<mpsc::UnboundedSender<String>>,
        input_device: cpal::Device,
        config: cpal::SupportedStreamConfig,
        cancel_token: CancellationToken,
        app_handle: AppHandle,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

        // 2. Connect WebSocket
        let url_str = format!("{}?model={}", WS_URL, QWEN_MODEL);
        let url = Url::parse(&url_str)?;
        
        // Generate WebSocket Key
        let key_bytes = Uuid::new_v4().into_bytes();
        let key_b64 = BASE64.encode(key_bytes);

        let request = http::Request::builder()
            .uri(url.as_str())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("OpenAI-Beta", "realtime=v1")
            .header("Host", url.host_str().unwrap_or("dashscope.aliyuncs.com"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", key_b64)
            .body(())?;

        let (ws_stream, _) = connect_async(request).await?;
        info!("Connected to Aliyun ASR WebSocket");

        let (mut write, mut read) = ws_stream.split();

        // 3. Send Init Event (Server VAD)
        let session_id = Uuid::new_v4().to_string();
        let init_event = json!({
            "event_id": session_id,
            "type": "session.update",
            "session": {
                "modalities": ["text"],
                "input_audio_format": "pcm",
                "sample_rate": 16000, // We must ensure we send 16k
                "input_audio_transcription": {
                    "language": "zh"
                },
                "turn_detection": {
                    "type": "server_vad",
                    "threshold": 0.5,
                    "silence_duration_ms": 800
                }
            }
        });
        
        write.send(Message::Text(init_event.to_string().into())).await?;

        // 4. Handle Incoming Messages (Transcript)
        let sender_clone = sender.clone();
        
        // We need a way to stop reading when cancelled.
        // We can use select! on read.next() and cancel_token.cancelled()
        
        // BUT `read` and `write` are split.
        // We should run the read loop in a separate task, but linked to cancellation?
        // Or just run it here.
        
        // Let's spawn the read loop, but we need to abort it when token cancels.
        let read_token = cancel_token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = read_token.cancelled() => {
                        info!("ASR Read Loop cancelled");
                        break;
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                debug!("ASR Message: {}", text);
                                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                                    // Check for final transcript
                                     if data["type"] == "conversation.item.input_audio_transcription.completed" {
                                        if let Some(transcript) = data["transcript"].as_str() {
                                            info!("ASR Transcript: {}", transcript);
                                            if let Some(s) = &sender_clone {
                                                let _ = s.send(transcript.to_string());
                                            }
                                        }
                                     }
                                }
                            }
                            Some(Ok(Message::Close(_))) => break,
                            Some(Err(e)) => {
                                error!("WebSocket read error: {}", e);
                                break;
                            },
                            None => break,
                            _ => {}
                        }
                    }
                }
            }
        });

        // 5. Stream Audio to WebSocket
        let err_fn = move |err| {
            error!("an error occurred on stream: {}", err);
        };
        
        // Channel to pass audio data from cpal thread to ws writer
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        
        let source_rate = config.sample_rate();
        let channels = config.channels() as usize;
        info!("Input Sample Rate: {}, Channels: {}", source_rate, channels);
        
        // Fractional resampling: source_rate -> 16000 Hz
        let step = source_rate as f64 / 16000.0;
        info!("Resample step: {:.6} (ratio {}:16000)", step, source_rate);

        // 1-pole low-pass prefilter to reduce aliasing before downsampling.
        // Cutoff at 7000 Hz (below Nyquist of 16 kHz target).
        let cutoff: f64 = 7000.0;
        let alpha: f32 = (2.0 * std::f64::consts::PI * cutoff
            / (2.0 * std::f64::consts::PI * cutoff + source_rate as f64)) as f32;

        // Streaming state shared across callback invocations (mutable, captured by closure).
        // src_pos tracks fractional position carried from previous buffer.
        let mut src_pos: f64 = 0.0;
        let mut last_sample: f32 = 0.0;
        let mut prev_filtered: f32 = 0.0;

        // Atomic volume level for the emission task
        let volume_level = Arc::new(AtomicU32::new(0));
        let volume_writer = volume_level.clone();

        let stream = input_device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                // Step 1: Downmix to mono if multi-channel
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks_exact(channels)
                        .map(|frame| {
                            let sum: f32 = frame.iter().sum();
                            sum / channels as f32
                        })
                        .collect()
                } else {
                    data.to_vec()
                };

                let mono_len = mono.len();
                if mono_len == 0 {
                    return;
                }

                // Step 2: Apply 1-pole low-pass filter in-place
                let mut filtered = Vec::with_capacity(mono_len);
                for &raw in &mono {
                    let f = alpha * raw + (1.0 - alpha) * prev_filtered;
                    prev_filtered = f;
                    filtered.push(f);
                }

                // Step 3: Fractional linear-interpolation resampling
                // Estimate output size: mono_len / step, plus a small margin
                let est_out = (mono_len as f64 / step) as usize + 2;
                let mut pcm_bytes = Vec::with_capacity(est_out * 2);
                let mut resampled_samples: Vec<f32> = Vec::with_capacity(est_out);

                while src_pos < mono_len as f64 {
                    let idx = src_pos.floor() as usize;
                    let frac = (src_pos - idx as f64) as f32;

                    // For idx == 0 and first buffer, use last_sample from previous buffer
                    let s0 = if idx == 0 {
                        last_sample
                    } else {
                        filtered[idx - 1]
                    };
                    let s1 = if idx < mono_len {
                        filtered[idx]
                    } else {
                        // Past end: hold last value
                        filtered[mono_len - 1]
                    };

                    let interpolated = s0 + frac * (s1 - s0);
                    resampled_samples.push(interpolated);
                    let s = (interpolated * 32767.0).round()
                        .max(-32768.0)
                        .min(32767.0) as i16;
                    pcm_bytes.extend_from_slice(&s.to_le_bytes());

                    src_pos += step;
                }

                // Carry state to next callback
                src_pos -= mono_len as f64;
                last_sample = *filtered.last().unwrap_or(&0.0);

                // Step 4: Compute RMS volume on resampled samples
                if !resampled_samples.is_empty() {
                    let sum_sq: f64 = resampled_samples
                        .iter()
                        .map(|&s| (s as f64) * (s as f64))
                        .sum();
                    let rms = (sum_sq / resampled_samples.len() as f64).sqrt();
                    let db = 20.0 * (rms.max(1e-10)).log10();
                    let level = ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u32;
                    volume_writer.store(level, Ordering::Relaxed);
                }

                let _ = audio_tx.send(pcm_bytes);
            },
            err_fn,
            None // None = blocking, use default timeout
        )?;
        
        stream.play()?;

        // Spawn volume emission task (~25 Hz)
        let volume_reader = volume_level.clone();
        let volume_handle = app_handle.clone();
        let volume_token = cancel_token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = volume_token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(40)) => {
                        let level = volume_reader.load(Ordering::Relaxed);
                        let _ = volume_handle.emit("mic-volume", level);
                    }
                }
            }
            // Emit 0 on stop
            let _ = volume_handle.emit("mic-volume", 0u32);
        });
        
        // Loop sending audio
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("Recording cancelled");
                    break;
                }
                Some(pcm_data) = audio_rx.recv() => {
                    // Send audio append event
                    let b64_audio = BASE64.encode(&pcm_data);
                    let audio_event = json!({
                         "event_id": Uuid::new_v4().to_string(),
                         "type": "input_audio_buffer.append",
                         "audio": b64_audio
                    });
                    
                    if let Err(e) = write.send(Message::Text(audio_event.to_string().into())).await {
                        error!("Failed to send audio to WS: {}", e);
                        break;
                    }
                }
                else => break,
            }
        }
        
        // Ensure stream is kept alive as long as loop runs
        drop(stream);
        
        // Close WS cleanly if possible
        let _ = write.close().await;

        Ok(())
    }
}
