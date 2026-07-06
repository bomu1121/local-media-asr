#!/usr/bin/env python3
"""E2E smoke test for asr_worker.py - runs a quick transcription without Tauri.
Usage: python test_e2e.py
"""
import subprocess, sys, json, os, wave, struct, tempfile

def test_asr_direct():
    """Generate a short test WAV and run asr_worker.py directly."""
    print("=" * 50)
    print("E2E Smoke Test: ASR Worker")
    print("=" * 50)

    # 1. Check models
    models_dir = os.path.join(os.path.dirname(__file__), "src-tauri", "models")
    para_model = os.path.join(models_dir, "paraformer-large", "model.int8.onnx")
    print(f"\n[1] Models dir: {models_dir}")
    print(f"    Paraformer model: {'OK' if os.path.exists(para_model) else 'MISSING!'}")
    if not os.path.exists(para_model):
        print("    SKIP: model not found")
        return False

    # 2. Check asr_worker.py
    worker = os.path.join(os.path.dirname(__file__), "asr_worker.py")
    print(f"\n[2] ASR worker: {worker}")
    print(f"    {'OK' if os.path.exists(worker) else 'MISSING!'}")

    # 3. Generate a 2-second test WAV (16kHz mono, sine tone + silence)
    wav_path = os.path.join(tempfile.gettempdir(), "asr_test.wav")
    sample_rate = 16000
    duration = 3.0
    n_samples = int(sample_rate * duration)
    # Generate a simple tone
    import math
    samples = []
    for i in range(n_samples):
        t = i / sample_rate
        samples.append(int(16000 * math.sin(2 * math.pi * 440 * t)))
    wav_bytes = struct.pack(f"<{n_samples}h", *samples)

    with wave.open(wav_path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sample_rate)
        w.writeframes(wav_bytes)
    print(f"\n[3] Test WAV: {wav_path} ({duration}s @ {sample_rate}Hz)")

    # 4. Run asr_worker
    print(f"\n[4] Running ASR worker...")
    cmd = [
        sys.executable, worker,
        "--wav", wav_path,
        "--model", "paraformer",
        "--models-dir", models_dir,
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    except subprocess.TimeoutExpired:
        print("    FAIL: Timeout after 60s")
        return False

    if result.returncode != 0:
        print(f"    FAIL: exit code {result.returncode}")
        print(f"    stderr: {result.stderr[:500]}")
        return False

    # 5. Parse output
    lines = [l.strip() for l in result.stdout.split("\n") if l.strip()]
    result_line = None
    for line in lines:
        try:
            msg = json.loads(line)
            if msg.get("type") == "result":
                result_line = msg
        except json.JSONDecodeError:
            pass

    if result_line:
        text = result_line.get("text", "")
        segs = result_line.get("segments", [])
        print(f"    OK: {len(segs)} segments, {len(text)} chars")
        print(f"    Sample: {text[:100]}...")
        return True
    else:
        print(f"    FAIL: No result JSON found in output")
        print(f"    Raw stdout (first 500 chars): {result.stdout[:500]}")
        return False

def test_pyinstaller_exe():
    """Test the bundled exe if it exists."""
    print("\n" + "=" * 50)
    print("E2E Smoke Test: PyInstaller EXE")
    print("=" * 50)

    exe = os.path.join(os.path.dirname(__file__), "src-tauri", "binaries", "asr_worker-x86_64-pc-windows-msvc.exe")
    if not os.path.exists(exe):
        exe = os.path.join(os.path.dirname(__file__), "asr_worker.exe")
    print(f"\n[1] EXE: {exe}")
    if not os.path.exists(exe):
        print("    SKIP: exe not built yet")
        return None

    print(f"    OK: {os.path.getsize(exe) / 1024 / 1024:.1f} MB")

if __name__ == "__main__":
    ok1 = test_asr_direct()
    ok2 = test_pyinstaller_exe()

    print("\n" + "=" * 50)
    if ok1:
        print("PASS: ASR worker functional")
    else:
        print("FAIL: ASR worker broken")
    print("=" * 50)
    sys.exit(0 if ok1 else 1)
