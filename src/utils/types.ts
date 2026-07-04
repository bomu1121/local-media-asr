// Types matching Rust backend structs for Tauri IPC

export interface AudioExtractArgs {
  input_path: string;
  output_path: string;
  denoise: boolean;
}

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  format: string;
  duration: number | null;
  audio_channels: number | null;
  sample_rate: number | null;
}

export interface AppConfig {
  models_dir: string;
  output_dir: string;
  ffmpeg_path: string;
  download_mirror: string;
}

export interface ModelStatus {
  name: string;
  installed: boolean;
  size_bytes: number | null;
  required: boolean;
}

export interface TranscriptionArgs {
  audio_path: string;
  engine_type: string;
  use_vad: boolean;
  use_punctuation: boolean;
  vad_threshold: number;
  min_speech_duration: number;
  min_silence_duration: number;
  max_segment_duration: number;
}

export interface TranscriptionSegment {
  start: number;
  end: number;
  text: string;
}

export interface TranscriptionResult {
  text: string;
  segments: TranscriptionSegment[];
  engine: string;
  duration: number;
}

export interface TaskProgress {
  task_id: string;
  stage: string;
  progress: number;
  message: string;
}

export interface TaskRecord {
  id: string;
  name: string;
  file_path: string;
  file_size: number;
  file_format: string;
  status: string;
  engine: string;
  created_at: string;
  updated_at: string;
  result: {
    task_id: string;
    engine: string;
    full_text: string;
    duration_secs: number;
    created_at: string;
    segments: Array<{ start_time: number; end_time: number; text: string }>;
  } | null;
}
