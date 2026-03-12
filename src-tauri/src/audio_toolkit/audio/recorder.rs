use std::{
    fmt,
    io::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SizedSample,
};

use crate::audio_toolkit::{
    audio::{AudioVisualiser, FrameResampler},
    constants,
    vad::{self, VadFrame},
    VoiceActivityDetector,
};

enum Cmd {
    Start {
        start_id: u64,
        ready_tx: Option<mpsc::Sender<StartReadyAck>>,
    },
    Stop(mpsc::Sender<Vec<f32>>),
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartReadyStatus {
    CaptureReady,
    SupersededByNewStart,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StartReadyAck {
    start_id: u64,
    status: StartReadyStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecorderStartError {
    CommandChannelUnavailable,
    CommandSendFailed,
    CaptureReadyTimeout(Duration),
    SupersededByNewStart,
    UnexpectedAcknowledgement {
        expected_start_id: u64,
        actual_start_id: u64,
    },
    WorkerDisconnected,
}

impl fmt::Display for RecorderStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecorderStartError::CommandChannelUnavailable => {
                write!(f, "Recorder command channel not initialized")
            }
            RecorderStartError::CommandSendFailed => {
                write!(f, "Failed to send start command to recorder worker")
            }
            RecorderStartError::CaptureReadyTimeout(timeout) => write!(
                f,
                "Timed out waiting for capture-ready acknowledgement (timeout_ms={})",
                timeout.as_millis()
            ),
            RecorderStartError::SupersededByNewStart => {
                write!(f, "Start attempt superseded by a newer start request")
            }
            RecorderStartError::UnexpectedAcknowledgement {
                expected_start_id,
                actual_start_id,
            } => write!(
                f,
                "Unexpected start acknowledgement id (expected={}, actual={})",
                expected_start_id, actual_start_id
            ),
            RecorderStartError::WorkerDisconnected => {
                write!(
                    f,
                    "Recorder worker disconnected before capture-ready acknowledgement"
                )
            }
        }
    }
}

impl std::error::Error for RecorderStartError {}

pub struct RecorderStartWait {
    start_id: u64,
    ready_rx: mpsc::Receiver<StartReadyAck>,
}

impl RecorderStartWait {
    pub fn wait(self, timeout: Duration) -> Result<(), RecorderStartError> {
        let ack = self.ready_rx.recv_timeout(timeout).map_err(|err| match err {
            mpsc::RecvTimeoutError::Timeout => RecorderStartError::CaptureReadyTimeout(timeout),
            mpsc::RecvTimeoutError::Disconnected => RecorderStartError::WorkerDisconnected,
        })?;

        if ack.start_id != self.start_id {
            return Err(RecorderStartError::UnexpectedAcknowledgement {
                expected_start_id: self.start_id,
                actual_start_id: ack.start_id,
            });
        }

        match ack.status {
            StartReadyStatus::CaptureReady => Ok(()),
            StartReadyStatus::SupersededByNewStart => Err(RecorderStartError::SupersededByNewStart),
        }
    }
}

pub struct RecorderStopWait {
    samples_rx: mpsc::Receiver<Vec<f32>>,
}

impl RecorderStopWait {
    pub fn wait(self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.samples_rx.recv().map_err(|_| {
            Error::new(
                std::io::ErrorKind::NotConnected,
                "Recorder worker disconnected before stop completed",
            )
            .into()
        })
    }
}

struct PreRollBuffer {
    slots: Vec<Vec<f32>>,
    head: usize,
    len: usize,
}

impl PreRollBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            slots: (0..capacity).map(|_| Vec::new()).collect(),
            head: 0,
            len: 0,
        }
    }

    fn push_frame(&mut self, frame: &[f32]) {
        if self.slots.is_empty() {
            return;
        }

        let slot_idx = if self.len < self.slots.len() {
            let idx = (self.head + self.len) % self.slots.len();
            self.len += 1;
            idx
        } else {
            let idx = self.head;
            self.head = (self.head + 1) % self.slots.len();
            idx
        };

        let slot = &mut self.slots[slot_idx];
        slot.clear();
        slot.extend_from_slice(frame);
    }

    fn drain_into<F>(&mut self, mut sink: F)
    where
        F: FnMut(&[f32]),
    {
        if self.slots.is_empty() {
            return;
        }

        for offset in 0..self.len {
            let idx = (self.head + offset) % self.slots.len();
            sink(&self.slots[idx]);
        }
        self.head = 0;
        self.len = 0;
    }
}

pub struct AudioRecorder {
    device: Option<Device>,
    cmd_tx: Option<mpsc::Sender<Cmd>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
    vad: Option<Arc<Mutex<Box<dyn vad::VoiceActivityDetector>>>>,
    level_cb: Option<Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>>,
    cached_config: Option<cpal::SupportedStreamConfig>,
    next_start_id: AtomicU64,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(AudioRecorder {
            device: None,
            cmd_tx: None,
            worker_handle: None,
            vad: None,
            level_cb: None,
            cached_config: None,
            next_start_id: AtomicU64::new(1),
        })
    }

    pub fn with_vad(mut self, vad: Box<dyn VoiceActivityDetector>) -> Self {
        self.vad = Some(Arc::new(Mutex::new(vad)));
        self
    }

    pub fn with_level_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(Vec<f32>) + Send + Sync + 'static,
    {
        self.level_cb = Some(Arc::new(cb));
        self
    }
    
    pub fn reset_cache(&mut self) {
        self.cached_config = None;
    }

    pub fn open(&mut self, device: Option<Device>) -> Result<(), Box<dyn std::error::Error>> {
        if self.worker_handle.is_some() {
            return Ok(()); // already open
        }
        let open_started = Instant::now();

        let (sample_tx, sample_rx) = mpsc::channel::<Vec<f32>>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();

        let host = crate::audio_toolkit::get_cpal_host();
        let device = match device {
            Some(dev) => dev,
            None => host
                .default_input_device()
                .ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "No input device found"))?,
        };

        let thread_device = device.clone();
        
        // Use cached config if available, otherwise fetch and cache it
        let config = if let Some(ref config) = self.cached_config {
            config.clone()
        } else {
            let config = AudioRecorder::get_preferred_config(&thread_device)
                .expect("failed to fetch preferred config");
            self.cached_config = Some(config.clone());
            config
        };
        
        let vad = self.vad.clone();
        // Move the optional level callback into the worker thread
        let level_cb = self.level_cb.clone();
        
        let (startup_tx, startup_rx) = mpsc::channel::<Result<mpsc::Receiver<()>, Box<dyn std::error::Error + Send + Sync>>>();

        let worker = std::thread::spawn(move || {
            let worker_started = Instant::now();
            let (data_started_tx, data_started_rx) = mpsc::channel::<()>();
            let sample_rate = config.sample_rate().0;
            let channels = config.channels() as usize;

            tracing::info!(
                "Using device: {:?}\nSample rate: {}\nChannels: {}\nFormat: {:?}",
                thread_device.name(),
                sample_rate,
                channels,
                config.sample_format()
            );

            let stream_result = match config.sample_format() {
                cpal::SampleFormat::U8 => {
                    AudioRecorder::build_stream::<u8>(&thread_device, &config, sample_tx, channels)
                }
                cpal::SampleFormat::I8 => {
                    AudioRecorder::build_stream::<i8>(&thread_device, &config, sample_tx, channels)
                }
                cpal::SampleFormat::I16 => {
                    AudioRecorder::build_stream::<i16>(&thread_device, &config, sample_tx, channels)
                }
                cpal::SampleFormat::I32 => {
                    AudioRecorder::build_stream::<i32>(&thread_device, &config, sample_tx, channels)
                }
                cpal::SampleFormat::F32 => {
                    AudioRecorder::build_stream::<f32>(&thread_device, &config, sample_tx, channels)
                }
                _ => panic!("unsupported sample format"),
            };

            let stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = startup_tx.send(Err(Box::new(e)));
                    return;
                }
            };
            tracing::debug!(
                event_code = "stream_open_subphase",
                phase = "stream_built",
                elapsed_ms = worker_started.elapsed().as_millis() as u64,
                "Recorder worker built input stream"
            );

            if let Err(e) = stream.play() {
                let _ = startup_tx.send(Err(Box::new(e)));
                return;
            }
            tracing::debug!(
                event_code = "stream_open_subphase",
                phase = "stream_play_started",
                elapsed_ms = worker_started.elapsed().as_millis() as u64,
                "Recorder worker started stream playback"
            );
            
            // Signal success, providing the receiver for data-started signal
            let _ = startup_tx.send(Ok(data_started_rx));

            // keep the stream alive while we process samples
            run_consumer(sample_rate, vad, sample_rx, cmd_rx, level_cb, Some(data_started_tx), Some(stream));
            // stream is dropped here, after run_consumer returns
        });

        // Wait for the stream to start
        match startup_rx.recv() {
            Ok(Ok(data_started_rx)) => {
                // Wait for the first audio packet to confirm data is flowing (max 3 seconds)
                // This is crucial for Bluetooth devices which take time to actually send data.
                if let Err(_) = data_started_rx.recv_timeout(Duration::from_secs(3)) {
                     // Make this fatal so we can trigger failover
                     let msg = "Timeout waiting for audio data to start flowing. Device might be silent or slow.";
                     tracing::error!("{}", msg);
                     return Err(msg.into());
                }
                tracing::debug!(
                    event_code = "stream_open_subphase",
                    phase = "first_packet_ready",
                    elapsed_ms = open_started.elapsed().as_millis() as u64,
                    "Recorder stream received first audio packet"
                );

                self.device = Some(device);
                self.cmd_tx = Some(cmd_tx);
                self.worker_handle = Some(worker);
                tracing::debug!(
                    event_code = "stream_open_subphase",
                    phase = "open_completed",
                    elapsed_ms = open_started.elapsed().as_millis() as u64,
                    "Recorder open completed"
                );
                Ok(())
            },
            Ok(Err(e)) => Err(e as Box<dyn std::error::Error>), // Stream build/play failed
            Err(_) => Err("Worker thread panicked check logs".into()), // Thread died
        }
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tx) = &self.cmd_tx {
            let start_id = self.next_start_id.fetch_add(1, Ordering::Relaxed);
            tx.send(Cmd::Start {
                start_id,
                ready_tx: None,
            })?;
        }
        Ok(())
    }

    pub fn start_blocking(&self, timeout: Duration) -> Result<(), RecorderStartError> {
        self.begin_start_blocking()?.wait(timeout)
    }

    pub fn begin_start_blocking(&self) -> Result<RecorderStartWait, RecorderStartError> {
        let Some(tx) = &self.cmd_tx else {
            return Err(RecorderStartError::CommandChannelUnavailable);
        };

        let start_id = self.next_start_id.fetch_add(1, Ordering::Relaxed);
        let (ready_tx, ready_rx) = mpsc::channel();
        tx.send(Cmd::Start {
            start_id,
            ready_tx: Some(ready_tx),
        })
        .map_err(|_| RecorderStartError::CommandSendFailed)?;

        Ok(RecorderStartWait { start_id, ready_rx })
    }

    pub fn begin_stop(&self) -> Result<RecorderStopWait, Box<dyn std::error::Error>> {
        let tx = self.cmd_tx.as_ref().ok_or_else(|| {
            Error::new(
                std::io::ErrorKind::NotConnected,
                "Recorder command channel not initialized",
            )
        })?;

        let (resp_tx, resp_rx) = mpsc::channel();
        tx.send(Cmd::Stop(resp_tx))?;
        Ok(RecorderStopWait {
            samples_rx: resp_rx,
        })
    }

    pub fn stop(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.begin_stop()?.wait()
    }

    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(Cmd::Shutdown);
        }
        if let Some(h) = self.worker_handle.take() {
            let _ = h.join();
        }
        self.device = None;
        // Clear cached config so next open() fetches the correct config for the new device
        self.cached_config = None;
        Ok(())
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::SupportedStreamConfig,
        sample_tx: mpsc::Sender<Vec<f32>>,
        channels: usize,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: Sample + SizedSample + Send + 'static,
        f32: cpal::FromSample<T>,
    {
        let mut output_buffer = Vec::new();

        let stream_cb = move |data: &[T], _: &cpal::InputCallbackInfo| {
            output_buffer.clear();

            if channels == 1 {
                // Direct conversion without intermediate Vec
                output_buffer.extend(data.iter().map(|&sample| sample.to_sample::<f32>()));
            } else {
                // Convert to mono directly
                let frame_count = data.len() / channels;
                output_buffer.reserve(frame_count);

                for frame in data.chunks_exact(channels) {
                    let mono_sample = frame
                        .iter()
                        .map(|&sample| sample.to_sample::<f32>())
                        .sum::<f32>()
                        / channels as f32;
                    output_buffer.push(mono_sample);
                }
            }

            if sample_tx.send(output_buffer.clone()).is_err() {
                tracing::error!("Failed to send samples");
            }
        };

        device.build_input_stream(
            &config.clone().into(),
            stream_cb,
            |err| tracing::error!("Stream error: {}", err),
            None,
        )
    }

    fn get_preferred_config(
        device: &cpal::Device,
    ) -> Result<cpal::SupportedStreamConfig, Box<dyn std::error::Error>> {
        let supported_configs = device.supported_input_configs()?;
        let mut best_config: Option<cpal::SupportedStreamConfigRange> = None;

        // Try to find a config that supports 16kHz, prioritizing better formats
        for config_range in supported_configs {
            if config_range.min_sample_rate().0 <= constants::WHISPER_SAMPLE_RATE
                && config_range.max_sample_rate().0 >= constants::WHISPER_SAMPLE_RATE
            {
                match best_config {
                    None => best_config = Some(config_range),
                    Some(ref current) => {
                        // Prioritize F32 > I16 > I32 > others
                        let score = |fmt: cpal::SampleFormat| match fmt {
                            cpal::SampleFormat::F32 => 4,
                            cpal::SampleFormat::I16 => 3,
                            cpal::SampleFormat::I32 => 2,
                            _ => 1,
                        };

                        if score(config_range.sample_format()) > score(current.sample_format()) {
                            best_config = Some(config_range);
                        }
                    }
                }
            }
        }

        if let Some(config) = best_config {
            return Ok(config.with_sample_rate(cpal::SampleRate(constants::WHISPER_SAMPLE_RATE)));
        }

        // If no config supports 16kHz, fall back to default
        Ok(device.default_input_config()?)
    }
}

fn run_consumer(
    in_sample_rate: u32,
    vad: Option<Arc<Mutex<Box<dyn vad::VoiceActivityDetector>>>>,
    sample_rx: mpsc::Receiver<Vec<f32>>,
    cmd_rx: mpsc::Receiver<Cmd>,
    level_cb: Option<Arc<dyn Fn(Vec<f32>) + Send + Sync + 'static>>,
    mut data_started_tx: Option<mpsc::Sender<()>>,
    stream: Option<cpal::Stream>,
) {
    const COMMAND_POLL_TIMEOUT: Duration = Duration::from_millis(8);

    let mut frame_resampler = FrameResampler::new(
        in_sample_rate as usize,
        constants::WHISPER_SAMPLE_RATE as usize,
        Duration::from_millis(30),
    );

    let mut processed_samples = Vec::<f32>::new();
    let mut recording = false;
    let mut pending_start_ready: Option<PendingStartReady> = None;
    
    // Warmup counter: Previously discarded initial frames after recording starts.
    // REMOVED: This was causing issues with short recordings because the SmoothedVad
    // already implements a pre-roll buffer (prefill_frames) that captures early audio.
    // Discarding warmup frames conflicted with the pre-roll design, causing the first
    // ~90ms of speech to be lost. Industry best practice is to use pre-roll buffers
    // to capture early speech, not to discard initial audio.
    const WARMUP_FRAMES: u32 = 0;
    let mut warmup_remaining: u32 = 0;
    // Keep ~270ms of audio before Start so fast speech onset is not clipped.
    // Assumes ~30ms resampler frames: 9 frames * 30ms = 270ms pre-roll.
    // Recalibrate this constant if the frame duration changes.
    const PREROLL_FRAMES: usize = 9;
    let mut pre_roll_frames = PreRollBuffer::new(PREROLL_FRAMES);

    // ---------- spectrum visualisation setup ---------------------------- //
    const BUCKETS: usize = 16;
    const WINDOW_SIZE: usize = 512;
    let mut visualizer = AudioVisualiser::new(
        in_sample_rate,
        WINDOW_SIZE,
        BUCKETS,
        400.0,  // vocal_min_hz
        4000.0, // vocal_max_hz
    );

    fn handle_frame(
        samples: &[f32],
        recording: bool,
        vad: &Option<Arc<Mutex<Box<dyn vad::VoiceActivityDetector>>>>,
        out_buf: &mut Vec<f32>,
    ) {
        if !recording {
            return;
        }

        if let Some(vad_arc) = vad {
            let mut det = vad_arc.lock().unwrap();
            match det.push_frame(samples).unwrap_or(VadFrame::Speech(samples)) {
                VadFrame::Speech(buf) => out_buf.extend_from_slice(buf),
                VadFrame::Noise => {}
            }
        } else {
            out_buf.extend_from_slice(samples);
        }
    }

    struct PendingStartReady {
        start_id: u64,
        ready_tx: mpsc::Sender<StartReadyAck>,
    }

    fn apply_start_command(
        start_id: u64,
        ready_tx: Option<mpsc::Sender<StartReadyAck>>,
        processed_samples: &mut Vec<f32>,
        recording: &mut bool,
        warmup_remaining: &mut u32,
        visualizer: &mut AudioVisualiser,
        vad: &Option<Arc<Mutex<Box<dyn vad::VoiceActivityDetector>>>>,
        pending_start_ready: &mut Option<PendingStartReady>,
        pre_roll_frames: &mut PreRollBuffer,
    ) {
        tracing::debug!(
            event_code = "recorder_start_subphase",
            phase = "worker_start_applied",
            start_id = start_id,
            "Recorder worker applied start command"
        );
        if let Some(previous_pending) = pending_start_ready.take() {
            let _ = previous_pending.ready_tx.send(StartReadyAck {
                start_id: previous_pending.start_id,
                status: StartReadyStatus::SupersededByNewStart,
            });
        }

        processed_samples.clear();
        *recording = true;
        *warmup_remaining = WARMUP_FRAMES;
        visualizer.reset();
        *pending_start_ready = ready_tx.map(|ready_tx| PendingStartReady { start_id, ready_tx });

        if let Some(v) = vad {
            v.lock().unwrap().reset();
        }

        pre_roll_frames.drain_into(|frame| handle_frame(frame, true, vad, processed_samples));
    }

    fn apply_stop_command(
        reply_tx: mpsc::Sender<Vec<f32>>,
        recording: &mut bool,
        pending_start_ready: &mut Option<PendingStartReady>,
        sample_rx: &mpsc::Receiver<Vec<f32>>,
        frame_resampler: &mut FrameResampler,
        vad: &Option<Arc<Mutex<Box<dyn vad::VoiceActivityDetector>>>>,
        processed_samples: &mut Vec<f32>,
    ) {
        let was_recording = *recording;
        *recording = false;
        *pending_start_ready = None;

        if was_recording {
            // Drain any audio chunks that were captured but not yet consumed.
            while let Ok(remaining) = sample_rx.try_recv() {
                frame_resampler.push(&remaining, &mut |frame: &[f32]| {
                    handle_frame(frame, true, vad, processed_samples)
                });
            }

            frame_resampler.finish(&mut |frame: &[f32]| {
                handle_frame(frame, true, vad, processed_samples)
            });
        }

        let _ = reply_tx.send(std::mem::take(processed_samples));
    }

    fn resume_stream(stream: &Option<cpal::Stream>) {
        if let Some(s) = stream {
            if let Err(e) = s.play() {
                tracing::error!("Failed to resume stream: {}", e);
            }
        }
    }

    fn pause_stream(stream: &Option<cpal::Stream>) {
        if let Some(s) = stream {
            if let Err(e) = s.pause() {
                tracing::error!("Failed to pause stream: {}", e);
            }
        }
    }

    loop {
        let raw = match sample_rx.recv_timeout(COMMAND_POLL_TIMEOUT) {
            Ok(s) => s,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Keep command/control path responsive even when no audio packets arrive.
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        Cmd::Start { start_id, ready_tx } => {
                            resume_stream(&stream);
                            apply_start_command(
                                start_id,
                                ready_tx,
                                &mut processed_samples,
                                &mut recording,
                                &mut warmup_remaining,
                                &mut visualizer,
                                &vad,
                                &mut pending_start_ready,
                                &mut pre_roll_frames,
                            )
                        }
                        Cmd::Stop(reply_tx) => {
                            pause_stream(&stream);
                            apply_stop_command(
                                reply_tx,
                                &mut recording,
                                &mut pending_start_ready,
                                &sample_rx,
                                &mut frame_resampler,
                                &vad,
                                &mut processed_samples,
                            )
                        }
                        Cmd::Shutdown => return,
                    }
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break, // stream closed
        };

        // Signal that data has started flowing (one-shot)
        if let Some(tx) = data_started_tx.take() {
            let _ = tx.send(());
        }

        // ---------- spectrum processing ---------------------------------- //
        if let Some(buckets) = visualizer.feed(&raw) {
            if let Some(cb) = &level_cb {
                cb(buckets);
            }
        }

        // Apply only the leading contiguous Start-prefix before processing this packet.
        // Once a non-Start command appears, preserve FIFO order by deferring it and all
        // subsequent commands to post-frame handling.
        let mut pending_cmds = Vec::new();
        while let Ok(cmd) = cmd_rx.try_recv() {
            pending_cmds.push(cmd);
        }

        let mut deferred_cmds = Vec::new();
        let mut defer_remaining_cmds = false;
        for cmd in pending_cmds {
            if defer_remaining_cmds {
                deferred_cmds.push(cmd);
                continue;
            }

            match cmd {
                Cmd::Start { start_id, ready_tx } => {
                    resume_stream(&stream);
                    apply_start_command(
                        start_id,
                        ready_tx,
                        &mut processed_samples,
                        &mut recording,
                        &mut warmup_remaining,
                        &mut visualizer,
                        &vad,
                        &mut pending_start_ready,
                        &mut pre_roll_frames,
                    )
                }
                other => {
                    defer_remaining_cmds = true;
                    deferred_cmds.push(other);
                }
            }
        }

        // ---------- existing pipeline ------------------------------------ //
        frame_resampler.push(&raw, &mut |frame: &[f32]| {
            if !recording {
                pre_roll_frames.push_frame(frame);
                return;
            }

            if let Some(pending) = pending_start_ready.take() {
                tracing::debug!(
                    event_code = "recorder_start_subphase",
                    phase = "worker_capture_ready_evidence",
                    start_id = pending.start_id,
                    "Recorder worker observed first post-start frame"
                );
                if pending
                    .ready_tx
                    .send(StartReadyAck {
                        start_id: pending.start_id,
                        status: StartReadyStatus::CaptureReady,
                    })
                    .is_err()
                {
                    // Caller abandoned the start wait (timeout/cancel), so cancel this start
                    // to avoid recording after the manager has already treated start as failed.
                    tracing::warn!(
                        "Capture-ready receiver dropped before acknowledgement; cancelling pending start"
                    );
                    recording = false;
                    processed_samples.clear();
                    pre_roll_frames.push_frame(frame);
                    return;
                }
            }

            // Skip initial warmup frames to allow microphone/VAD to stabilize
            if warmup_remaining > 0 {
                warmup_remaining -= 1;
                return;
            }
            handle_frame(frame, recording, &vad, &mut processed_samples)
        });

        // Handle deferred non-start commands after processing this packet.
        for cmd in deferred_cmds {
            match cmd {
                Cmd::Stop(reply_tx) => {
                    pause_stream(&stream);
                    apply_stop_command(
                        reply_tx,
                        &mut recording,
                        &mut pending_start_ready,
                        &sample_rx,
                        &mut frame_resampler,
                        &vad,
                        &mut processed_samples,
                    )
                }
                Cmd::Shutdown => return,
                Cmd::Start { start_id, ready_tx } => {
                    resume_stream(&stream);
                    apply_start_command(
                        start_id,
                        ready_tx,
                        &mut processed_samples,
                        &mut recording,
                        &mut warmup_remaining,
                        &mut visualizer,
                        &vad,
                        &mut pending_start_ready,
                        &mut pre_roll_frames,
                    )
                }
            }
        }

        // non-blocking check for a command
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Cmd::Start { start_id, ready_tx } => {
                    resume_stream(&stream);
                    apply_start_command(
                        start_id,
                        ready_tx,
                        &mut processed_samples,
                        &mut recording,
                        &mut warmup_remaining,
                        &mut visualizer,
                        &vad,
                        &mut pending_start_ready,
                        &mut pre_roll_frames,
                    )
                }
                Cmd::Stop(reply_tx) => {
                    pause_stream(&stream);
                    apply_stop_command(
                        reply_tx,
                        &mut recording,
                        &mut pending_start_ready,
                        &sample_rx,
                        &mut frame_resampler,
                        &vad,
                        &mut processed_samples,
                    )
                }
                Cmd::Shutdown => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        run_consumer, AudioRecorder, Cmd, RecorderStartError, RecorderStopWait, StartReadyStatus,
    };
    use crate::audio_toolkit::constants;
    use std::sync::{
        atomic::AtomicU64,
        mpsc, Arc, Mutex,
    };
    use std::time::Duration;

    fn spawn_consumer() -> (
        mpsc::Sender<Vec<f32>>,
        mpsc::Sender<Cmd>,
        std::thread::JoinHandle<()>,
    ) {
        let (sample_tx, cmd_tx, _, worker) = spawn_consumer_with_done();
        (sample_tx, cmd_tx, worker)
    }

    fn spawn_consumer_with_done() -> (
        mpsc::Sender<Vec<f32>>,
        mpsc::Sender<Cmd>,
        mpsc::Receiver<()>,
        std::thread::JoinHandle<()>,
    ) {
        let (sample_tx, sample_rx) = mpsc::channel::<Vec<f32>>();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            run_consumer(
                constants::WHISPER_SAMPLE_RATE,
                None,
                sample_rx,
                cmd_rx,
                None,
                None,
                None,
            );
            let _ = done_tx.send(());
        });
        (sample_tx, cmd_tx, done_rx, worker)
    }

    #[test]
    fn start_ack_arrives_after_first_recorded_frame() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();
        let (ready_tx, ready_rx) = mpsc::channel();

        cmd_tx
            .send(Cmd::Start {
                start_id: 1,
                ready_tx: Some(ready_tx),
            })
            .unwrap();

        sample_tx.send(vec![0.5; 480]).unwrap();
        let ready_ack = ready_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("capture-ready acknowledgement should arrive");
        assert_eq!(ready_ack.start_id, 1);
        assert_eq!(ready_ack.status, StartReadyStatus::CaptureReady);

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();

        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return captured samples");
        assert!(
            !samples.is_empty(),
            "first frame should be captured once start is acknowledged"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn pre_roll_frames_are_included_after_start() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();

        // Feed audio before Start so it can be buffered in pre-roll.
        sample_tx.send(vec![0.25; 480]).unwrap();

        let (ready_tx, ready_rx) = mpsc::channel();
        cmd_tx
            .send(Cmd::Start {
                start_id: 1,
                ready_tx: Some(ready_tx),
            })
            .unwrap();
        sample_tx.send(vec![0.5; 480]).unwrap();
        let ready_ack = ready_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("capture-ready acknowledgement should arrive");
        assert_eq!(ready_ack.start_id, 1);
        assert_eq!(ready_ack.status, StartReadyStatus::CaptureReady);

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();

        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return captured samples");
        let has_preroll = samples.iter().any(|s| (*s - 0.25).abs() < 1e-3);
        let has_speech = samples.iter().any(|s| (*s - 0.5).abs() < 1e-3);
        assert!(has_preroll, "pre-roll frame must be present in final samples");
        assert!(has_speech, "speech frame must be present in final samples");

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn pre_roll_wraparound_keeps_latest_frames_in_order() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();

        // Feed more than PREROLL_FRAMES so the oldest buffered frames are dropped.
        for value in 10..=20 {
            sample_tx.send(vec![value as f32; 480]).unwrap();
        }

        let (ready_tx, ready_rx) = mpsc::channel();
        cmd_tx
            .send(Cmd::Start {
                start_id: 1,
                ready_tx: Some(ready_tx),
            })
            .unwrap();
        sample_tx.send(vec![99.0; 480]).unwrap();
        let ready_ack = ready_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("capture-ready acknowledgement should arrive");
        assert_eq!(ready_ack.start_id, 1);
        assert_eq!(ready_ack.status, StartReadyStatus::CaptureReady);

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();
        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return captured samples");

        let frame_values: Vec<f32> = samples
            .chunks(480)
            .filter_map(|chunk| chunk.first().copied())
            .collect();
        let expected_preroll: Vec<f32> = (12..=20).map(|value| value as f32).collect();
        let has_expected_preroll_order = frame_values.windows(expected_preroll.len()).any(|window| {
            window
                .iter()
                .zip(expected_preroll.iter())
                .all(|(actual, expected)| (*actual - *expected).abs() < 1e-3)
        });
        assert!(
            has_expected_preroll_order,
            "pre-roll should retain only the newest frames in chronological order"
        );
        assert!(
            samples.iter().any(|s| (*s - 99.0).abs() < 1e-3),
            "speech frame after start must still be present"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn dropped_ready_receiver_cancels_pending_start() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();

        // Buffer pre-roll so Start seeds processed samples, then verify cancellation clears them.
        sample_tx.send(vec![0.25; 480]).unwrap();

        let (ready_tx, ready_rx) = mpsc::channel();
        cmd_tx
            .send(Cmd::Start {
                start_id: 1,
                ready_tx: Some(ready_tx),
            })
            .unwrap();

        // Simulate caller timeout/cancel by dropping the receiver before first recorded frame.
        drop(ready_rx);
        sample_tx.send(vec![0.5; 480]).unwrap();

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();

        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return captured samples");
        assert!(
            samples.is_empty(),
            "abandoned start must not keep captured audio"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn newer_start_supersedes_older_pending_start() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();
        let (ready_a_tx, ready_a_rx) = mpsc::channel();
        let (ready_b_tx, ready_b_rx) = mpsc::channel();

        cmd_tx
            .send(Cmd::Start {
                start_id: 10,
                ready_tx: Some(ready_a_tx),
            })
            .unwrap();
        cmd_tx
            .send(Cmd::Start {
                start_id: 11,
                ready_tx: Some(ready_b_tx),
            })
            .unwrap();

        let superseded_ack = ready_a_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("older pending start should be superseded explicitly");
        assert_eq!(superseded_ack.start_id, 10);
        assert_eq!(
            superseded_ack.status,
            StartReadyStatus::SupersededByNewStart
        );

        sample_tx.send(vec![0.75; 480]).unwrap();
        let capture_ready_ack = ready_b_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("latest start should receive capture-ready acknowledgement");
        assert_eq!(capture_ready_ack.start_id, 11);
        assert_eq!(capture_ready_ack.status, StartReadyStatus::CaptureReady);

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();
        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return captured samples");
        assert!(
            !samples.is_empty(),
            "latest start should continue recording after superseding an older start"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn stop_then_start_same_batch_preserves_fifo() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();

        let (ready_a_tx, ready_a_rx) = mpsc::channel();
        cmd_tx
            .send(Cmd::Start {
                start_id: 100,
                ready_tx: Some(ready_a_tx),
            })
            .unwrap();
        sample_tx.send(vec![0.11; 480]).unwrap();
        let ready_a_ack = ready_a_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("first start should become capture-ready");
        assert_eq!(ready_a_ack.start_id, 100);
        assert_eq!(ready_a_ack.status, StartReadyStatus::CaptureReady);

        sample_tx.send(vec![0.22; 480]).unwrap();

        let (stop_a_tx, stop_a_rx) = mpsc::channel();
        let (ready_b_tx, ready_b_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_a_tx)).unwrap();
        cmd_tx
            .send(Cmd::Start {
                start_id: 101,
                ready_tx: Some(ready_b_tx),
            })
            .unwrap();

        sample_tx.send(vec![0.33; 480]).unwrap();
        let stop_a_samples = stop_a_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("older stop should complete before newer start is armed");
        assert!(
            stop_a_samples.iter().any(|s| (*s - 0.22).abs() < 1e-3),
            "older stop window should preserve already-recorded audio"
        );

        sample_tx.send(vec![0.44; 480]).unwrap();
        let ready_b_ack = ready_b_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("second start should become capture-ready");
        assert_eq!(ready_b_ack.start_id, 101);
        assert_eq!(ready_b_ack.status, StartReadyStatus::CaptureReady);

        let (stop_b_tx, stop_b_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_b_tx)).unwrap();
        sample_tx.send(Vec::new()).unwrap();
        let stop_b_samples = stop_b_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("second stop should return samples from the newer recording");
        assert!(
            stop_b_samples.iter().any(|s| (*s - 0.44).abs() < 1e-3),
            "newer recording should capture post-restart audio"
        );
        assert!(
            !stop_b_samples.iter().any(|s| (*s - 0.22).abs() < 1e-3),
            "newer recording must not inherit old-window samples"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn stop_then_start_does_not_clear_old_stop_window_incorrectly() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();

        let (ready_a_tx, ready_a_rx) = mpsc::channel();
        cmd_tx
            .send(Cmd::Start {
                start_id: 200,
                ready_tx: Some(ready_a_tx),
            })
            .unwrap();
        sample_tx.send(vec![0.51; 480]).unwrap();
        let ready_a_ack = ready_a_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("first start should become capture-ready");
        assert_eq!(ready_a_ack.start_id, 200);
        assert_eq!(ready_a_ack.status, StartReadyStatus::CaptureReady);

        sample_tx.send(vec![0.61; 480]).unwrap();

        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_b_tx, _ready_b_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        cmd_tx
            .send(Cmd::Start {
                start_id: 201,
                ready_tx: Some(ready_b_tx),
            })
            .unwrap();

        sample_tx.send(vec![0.71; 480]).unwrap();
        let stopped_samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should return the old recording window");
        assert!(
            stopped_samples.iter().any(|s| (*s - 0.61).abs() < 1e-3),
            "old recording marker must survive stop->start same-batch sequencing"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn start_then_stop_same_batch_still_captures_expected_current_frame_behavior() {
        let (sample_tx, cmd_tx, worker) = spawn_consumer();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();

        cmd_tx
            .send(Cmd::Start {
                start_id: 300,
                ready_tx: Some(ready_tx),
            })
            .unwrap();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();

        sample_tx.send(vec![0.81; 480]).unwrap();

        let ready_ack = ready_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("start should be armed before packet processing");
        assert_eq!(ready_ack.start_id, 300);
        assert_eq!(ready_ack.status, StartReadyStatus::CaptureReady);

        let samples = stop_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("stop should include current-frame samples");
        assert!(
            samples.iter().any(|s| (*s - 0.81).abs() < 1e-3),
            "start->stop same batch should preserve start-before-frame semantics"
        );

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn shutdown_exits_without_samples() {
        let (sample_tx, cmd_tx, done_rx, worker) = spawn_consumer_with_done();

        cmd_tx.send(Cmd::Shutdown).unwrap();
        done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("shutdown must exit promptly even without audio samples");

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn stop_while_not_recording_returns_without_samples() {
        let (sample_tx, cmd_tx, done_rx, worker) = spawn_consumer_with_done();

        let (stop_tx, stop_rx) = mpsc::channel();
        cmd_tx.send(Cmd::Stop(stop_tx)).unwrap();
        let samples = stop_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("stop should return quickly when not recording");
        assert!(samples.is_empty(), "stop without recording should return no audio");

        cmd_tx.send(Cmd::Shutdown).unwrap();
        done_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("shutdown must still exit promptly after stop");

        drop(sample_tx);
        worker.join().unwrap();
    }

    #[test]
    fn stop_returns_error_when_command_channel_missing() {
        let recorder = AudioRecorder::new().expect("recorder should construct");
        let err = recorder
            .stop()
            .expect_err("stop should fail-fast when command channel is not available");
        let io_err = err
            .downcast_ref::<std::io::Error>()
            .expect("stop error should be an io::Error");
        assert_eq!(io_err.kind(), std::io::ErrorKind::NotConnected);
    }

    #[test]
    fn begin_start_blocking_allows_wait_outside_external_mutex() {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let recorder = AudioRecorder {
            device: None,
            cmd_tx: Some(cmd_tx),
            worker_handle: None,
            vad: None,
            level_cb: None,
            cached_config: None,
            next_start_id: AtomicU64::new(1),
        };
        let recorder_mutex = Arc::new(Mutex::new(Some(recorder)));

        // Simulate manager behavior: hold mutex only long enough to dispatch Start.
        let start_wait = {
            let guard = recorder_mutex.lock().unwrap();
            guard
                .as_ref()
                .expect("recorder must exist")
                .begin_start_blocking()
                .expect("start dispatch should succeed")
        };

        // While wait is pending, external code should still acquire the same mutex.
        let recorder_mutex_for_thread = Arc::clone(&recorder_mutex);
        let wait_thread = std::thread::spawn(move || start_wait.wait(Duration::from_millis(50)));
        let guard = recorder_mutex_for_thread
            .lock()
            .expect("external mutex acquisition should not block on capture-ready wait");
        assert!(guard.is_some());
        drop(guard);

        let wait_result = wait_thread.join().unwrap();
        assert!(matches!(
            wait_result,
            Err(RecorderStartError::CaptureReadyTimeout(timeout))
            if timeout == Duration::from_millis(50)
        ));

        // Keep the receiver alive through the wait timeout.
        drop(cmd_rx);
    }

    #[test]
    fn begin_stop_allows_wait_outside_external_mutex() {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
        let recorder = AudioRecorder {
            device: None,
            cmd_tx: Some(cmd_tx),
            worker_handle: None,
            vad: None,
            level_cb: None,
            cached_config: None,
            next_start_id: AtomicU64::new(1),
        };
        let recorder_mutex = Arc::new(Mutex::new(Some(recorder)));

        let stop_wait: RecorderStopWait = {
            let guard = recorder_mutex.lock().unwrap();
            guard
                .as_ref()
                .expect("recorder must exist")
                .begin_stop()
                .expect("stop dispatch should succeed")
        };

        let recorder_mutex_for_thread = Arc::clone(&recorder_mutex);
        let wait_thread = std::thread::spawn(move || stop_wait.wait().is_ok());
        let guard = recorder_mutex_for_thread
            .lock()
            .expect("external mutex acquisition should not block on stop wait");
        assert!(guard.is_some());
        drop(guard);

        let Cmd::Stop(reply_tx) = cmd_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("stop command should be dispatched")
        else {
            panic!("expected stop command");
        };
        reply_tx
            .send(vec![0.42; 8])
            .expect("stop reply should be delivered");

        let stop_completed = wait_thread.join().expect("stop wait thread should join");
        assert!(stop_completed, "stop wait should complete successfully");
    }
}
