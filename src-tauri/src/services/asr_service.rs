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

    pub async fn start_recording(&self, cancel_token: CancellationToken) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        // Force sample rate if possible, or resample. 
        // Aliyun usually expects 16000Hz PCM. 
        // For simplicity here, we assume device supports it or we accept mismatch (might fail).
        // A robust impl would do resampling.
        
        #[allow(deprecated)]
        let device_name_2 = input_device.name().unwrap_or("unknown".into());
        info!("Input device: {}", device_name_2);
        info!("Default config: {:?}", config);
        
        // Spawn async setup to avoid blocking
        tokio::spawn(async move {
            if let Err(e) = Self::run_async_recording(api_key, sender, input_device, config, cancel_token).await {
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
        
        let sample_rate = config.sample_rate();
        info!("Input Sample Rate: {}", sample_rate);
        
        // Simple decimation factor (downsampling)
        let downsample_factor = (sample_rate as f32 / 16000.0).round() as usize;
        let downsample_factor = if downsample_factor < 1 { 1 } else { downsample_factor };
        
        info!("Downsample factor: {}", downsample_factor);

        let stream = input_device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                // Convert f32 samples to i16 (PCM)
                // cpal gives f32 usually, need to scale to i16 range
                // Simple downsampling
                let mut pcm_bytes = Vec::with_capacity((data.len() / downsample_factor) * 2);
                for (i, &sample) in data.iter().enumerate() {
                    if i % downsample_factor == 0 {
                         let s = (sample * 32767.0) as i16;
                         pcm_bytes.extend_from_slice(&s.to_le_bytes());
                    }
                }
                let _ = audio_tx.send(pcm_bytes);
            },
            err_fn,
            None // None = blocking, use default timeout
        )?;
        
        stream.play()?;
        
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
