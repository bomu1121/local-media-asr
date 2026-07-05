#!/usr/bin/env python3
"""ASR worker: VAD segmentation + Paraformer ASR via sherpa-onnx.
Called by the Tauri frontend via shell spawn.
Outputs JSON lines to stdout:
  {"type":"progress","chunk":1,"total":20,"text":"..."}
  {"type":"result","text":"...","segments":[{"start":0,"end":2.5,"text":"..."}]}
  {"type":"error","message":"..."}
"""

import sys, os
# Force UTF-8 output for Tauri (Windows console may use GBK)
os.environ["PYTHONIOENCODING"] = "utf-8"
# Safe reconfigure ? stdout/stderr may be None in GUI subsystem
if sys.stdout is not None:
    sys.stdout.reconfigure(encoding="utf-8")
if sys.stderr is not None:
    sys.stderr.reconfigure(encoding="utf-8")

import json, wave, time, argparse
import numpy as np

def detect_speech_segments(samples, sample_rate=16000, threshold=0.05,
                           min_dur=1.0, max_dur=60.0, min_silence=0.5):
    """Simple energy-based VAD. Returns [(start_s, end_s), ...]."""
    sr = sample_rate
    frame_size = int(sr * 0.025)  # 25ms
    total_frames = len(samples) // frame_size
    if total_frames == 0:
        return []

    # Compute energy per frame
    energies = []
    for i in range(total_frames):
        start = i * frame_size
        end = start + frame_size
        chunk = samples[start:end]
        e = np.mean(chunk.astype(np.float64) ** 2)
        energies.append(e)

    max_e = max(energies) if energies else 1.0
    if max_e == 0:
        return []

    segments = []
    in_speech = False
    speech_start = 0
    silence_frames = 0

    for i, e in enumerate(energies):
        is_speech = (e / max_e) > threshold
        if is_speech and not in_speech:
            in_speech = True
            speech_start = i
            silence_frames = 0
        elif not is_speech and in_speech:
            silence_frames += 1
            if silence_frames * frame_size >= min_silence * sr:
                seg_start = speech_start * frame_size / sr
                seg_end = (i - silence_frames) * frame_size / sr
                dur = seg_end - seg_start
                if dur >= min_dur:
                    if dur > max_dur:
                        # Split long segments
                        pos = seg_start
                        while pos < seg_end:
                            end = min(pos + max_dur, seg_end)
                            segments.append((pos, end))
                            pos = end
                    else:
                        segments.append((seg_start, seg_end))
                in_speech = False
        elif is_speech and in_speech:
            silence_frames = 0

    if in_speech:
        seg_start = speech_start * frame_size / sr
        seg_end = len(samples) / sr
        dur = seg_end - seg_start
        if dur >= min_dur:
            segments.append((seg_start, seg_end))

    return segments


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--wav", required=True, help="16kHz mono WAV file")
    parser.add_argument("--model", default="paraformer", choices=["paraformer", "sensevoice"])
    parser.add_argument("--models-dir", default=None, help="Models root directory")
    parser.add_argument("--threshold", type=float, default=0.02, help="VAD energy threshold")
    parser.add_argument("--no-vad", action="store_true", help="Disable VAD, use whole audio")
    args = parser.parse_args()

    # Resolve models dir
    if args.models_dir:
        models_root = args.models_dir
    else:
        # Default: relative to this script or cwd
        script_dir = os.path.dirname(os.path.abspath(__file__))
        candidates = [
            os.path.join(script_dir, "models"),
            os.path.join(script_dir, "src-tauri", "models"),
            os.path.join(os.getcwd(), "models"),
            os.path.join(os.getcwd(), "src-tauri", "models"),
        ]
        models_root = next((d for d in candidates if os.path.isdir(d)), None)
        if not models_root:
            emit_error("Models directory not found")
            return

    # Determine model dir
    model_name = "paraformer-large" if args.model == "paraformer" else "sense-voice-small"
    model_dir = os.path.join(models_root, model_name)
    if not os.path.isdir(model_dir):
        emit_error(f"Model directory not found: {model_dir}")
        return

    # Find inner directory (contains model.int8.onnx)
    inner = model_dir
    for f in sorted(os.listdir(model_dir)):
        sub = os.path.join(model_dir, f)
        if os.path.isdir(sub) and os.path.exists(os.path.join(sub, "model.int8.onnx")):
            inner = sub
            break

    model_path = os.path.join(inner, "model.int8.onnx")
    tokens_path = os.path.join(inner, "tokens.txt")

    if not os.path.exists(model_path):
        emit_error(f"Model file not found: {model_path}")
        return

    # Load ASR model
    try:
        from sherpa_onnx import offline_recognizer as o
    except ImportError:
        emit_error("sherpa-onnx not installed. Run: pip install sherpa-onnx")
        return

    try:
        if args.model == "paraformer":
            rec = o.OfflineRecognizer.from_paraformer(
                paraformer=model_path, tokens=tokens_path,
                num_threads=4, sample_rate=16000,
            )
        else:
            emit_error("SenseVoice not supported in Python sherpa-onnx 1.8.11. Use --model paraformer")
            return
    except Exception as e:
        emit_error(f"Failed to load ASR model: {e}")
        return

    # Load audio
    try:
        with wave.open(args.wav, "r") as w:
            nchannels = w.getnchannels()
            sr = w.getframerate()
            nframes = w.getnframes()
            data = w.readframes(nframes)
        samples = np.frombuffer(data, dtype=np.int16).astype(np.float32) / 32768.0
        if nchannels > 1:
            samples = samples.reshape(-1, nchannels).mean(axis=1)
    except Exception as e:
        emit_error(f"Failed to load WAV: {e}")
        return

    # VAD segmentation
    if args.no_vad:
        segments = [(0.0, len(samples) / 16000)]
    else:
        segments = detect_speech_segments(samples, threshold=args.threshold)

    if not segments:
        # Fallback: chunk every 30s
        total_s = len(samples) / 16000
        segments = [(i, min(i + 60, total_s)) for i in range(0, int(total_s) + 1, 55)]

    total_chunks = len(segments)

    # Process each segment
    all_segments = []
    full_text_parts = []

    for i, (start_s, end_s) in enumerate(segments):
        start_idx = int(start_s * 16000)
        end_idx = int(end_s * 16000)
        chunk = samples[start_idx:end_idx]

        if len(chunk) < 1600:  # skip < 0.1s
            continue

        stream = rec.create_stream()
        stream.accept_waveform(16000, chunk)
        rec.decode_stream(stream)
        text = stream.result.text.strip()

        if text:
            # Simple dedup: skip exact duplicates of previous
            if full_text_parts and text == full_text_parts[-1]:
                continue
            full_text_parts.append(text)
            all_segments.append({
                "start": round(start_s, 2),
                "end": round(end_s, 2),
                "text": text,
            })

        # Emit progress
        emit_progress(i + 1, total_chunks, text)

    full_text = "\n".join(full_text_parts)

    # Emit final result
    emit_result(full_text, all_segments)


def emit_progress(chunk, total, text):
    sys.stdout.write(json.dumps({
        "type": "progress",
        "chunk": chunk,
        "total": total,
        "text": text,
    }, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def emit_result(text, segments):
    sys.stdout.write(json.dumps({
        "type": "result",
        "text": text,
        "segments": segments,
    }, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def emit_error(message):
    sys.stdout.write(json.dumps({
        "type": "error",
        "message": message,
    }, ensure_ascii=False) + "\n")
    sys.stdout.flush()


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        import traceback
        sys.stderr.write(json.dumps({
            "type": "error",
            "message": str(e),
            "traceback": traceback.format_exc(),
        }, ensure_ascii=False) + "\n")
        sys.stderr.flush()
        sys.exit(2)
