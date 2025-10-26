import { useState, useRef, useCallback, useEffect } from "react";

// The path to our audio worklet in the `public` directory.
const AUDIO_WORKLET_URL = "/audio-processor.js";

type VoiceRecorderStatus = "idle" | "recording" | "stopping";

/**
 * A custom React hook to manage microphone recording using an AudioWorklet.
 * @param onAudioData A callback that receives chunks of raw PCM16 audio data.
 */
export function useVoiceRecorder(onAudioData: (data: ArrayBuffer) => void) {
  const [status, setStatus] = useState<VoiceRecorderStatus>("idle");
  // Use a ref to hold the current status for use in stale closures.
  const statusRef = useRef(status);

  const audioContextRef = useRef<AudioContext | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const workletNodeRef = useRef<AudioWorkletNode | null>(null);

  // Keep the ref synchronized with the state.
  useEffect(() => {
    statusRef.current = status;
  }, [status]);

  const start = useCallback(async () => {
    // Check ref to prevent race conditions.
    if (statusRef.current !== "idle") return;

    setStatus("recording"); // Optimistically set status

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      mediaStreamRef.current = stream;

      const context = new AudioContext();
      console.log("🎤 Audio context sample rate:", context.sampleRate);
      audioContextRef.current = context;

      await context.audioWorklet.addModule(AUDIO_WORKLET_URL);
      const source = context.createMediaStreamSource(stream);
      const processor = new AudioWorkletNode(context, "audio-processor");

      processor.port.onmessage = (event) => {
        // Check the ref here to avoid sending data if recording has stopped.
        if (statusRef.current === "recording") {
          onAudioData(event.data);
        }
      };

      source.connect(processor);
      workletNodeRef.current = processor;
      console.log("Microphone is on. Start speaking.");
    } catch (error) {
      console.error("Failed to start voice recorder:", error);
      // Here you would typically show a toast notification to the user.
      setStatus("idle"); // Rollback status on error
    }
  }, [onAudioData]);

  const stop = useCallback(() => {
    if (statusRef.current !== "recording") return;

    setStatus("stopping");
    // Clean up all the audio resources.
    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach((track) => track.stop());
      mediaStreamRef.current = null;
    }
    if (workletNodeRef.current) {
      workletNodeRef.current.disconnect();
      workletNodeRef.current = null;
    }
    if (audioContextRef.current) {
      if (audioContextRef.current.state !== "closed") {
        audioContextRef.current.close();
      }
      audioContextRef.current = null;
    }
    setStatus("idle");
    console.log("Microphone off.");
  }, []);

  const isRecording = status === "recording";

  return { start, stop, isRecording, status };
}

