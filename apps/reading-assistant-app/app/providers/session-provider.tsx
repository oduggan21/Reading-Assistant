import {
  createContext,
  useContext,
  useRef,
  type ReactNode,
  useCallback,
  useState,
} from "react";
import { createStore, useStore } from "zustand";
import { useMutation } from "@tanstack/react-query";
import { createSessionHandler } from "@reading-assistant/query/handlers/handlers";
import type { CreateSessionResponse } from "@reading-assistant/query/schemas";  // ✅ Updated import path
import { WsClient } from "~/lib/ws-client";
import { AudioPlayer } from "~/lib/audio-player";
import { useVoiceRecorder } from "~/hooks/use-voice-recorder";

// Define the possible states of the session for the UI
export type SessionStatus =
  | "idle"
  | "uploading"
  | "connecting"
  | "reading"
  | "listening"
  | "processing"
  | "answering"
  | "paused"
  | "ended";

// Define the shape of our state
interface SessionState {
  status: SessionStatus;
  isInterruptible: boolean;
  sessionId: string | null;
  // ❌ Removed userId - no longer needed
}

// Define the actions that can modify the state
interface SessionActions {
  setStatus: (status: SessionStatus) => void;
  setInterruptible: (isInterruptible: boolean) => void;
  setSessionId: (sessionId: string | null) => void;
  // ❌ Removed setUserId - no longer needed
  reset: () => void;
}

// Create a Zustand store to hold our state and actions
const createSessionStore = () =>
  createStore<SessionState & SessionActions>((set) => ({
    status: "idle",
    isInterruptible: false,
    sessionId: null,
    // ❌ Removed userId - no longer needed
    setStatus: (status) => set({ status }),
    setInterruptible: (isInterruptible) => set({ isInterruptible }),
    setSessionId: (sessionId) => set({ sessionId }),
    // ❌ Removed setUserId - no longer needed
    reset: () =>
      set({
        status: "idle",
        isInterruptible: false,
        sessionId: null,
      }),
  }));

// This is the type for the value that our context will provide.
type SessionContextType = {
  store: ReturnType<typeof createSessionStore>;
  uploadDocument: (file: File) => void;
  isUploading: boolean;
  connect: (sessionId: string) => void;
  disconnect: () => void;
  pauseReading: () => void;
  resumeReading: () => void;
  isRecording: boolean;
  startRecording: () => void;
  stopRecordingAndSend: () => void;
};

const SessionContext = createContext<SessionContextType | null>(null);

type SessionProviderProps = { children: ReactNode };

export function SessionProvider({ children }: SessionProviderProps) {
  const store = useRef(createSessionStore()).current;

  const wsClientRef = useRef<WsClient | null>(null);
  const audioPlayerRef = useRef<AudioPlayer | null>(null);

  // ✅ Updated mutation - no userId needed
  const { mutate: createSessionMutation, isPending: isUploading } = useMutation({
    mutationFn: ({ formData }: { formData: FormData }) => {
      return createSessionHandler({
        headers: { "Content-Type": "multipart/form-data" },
        data: formData,
      });
    },
  });

  const { isRecording, start: startRecorder, stop: stopRecorder } =
    useVoiceRecorder((audioChunk: ArrayBuffer) => {
      wsClientRef.current?.sendAudio(audioChunk);
    });

  // ✅ Updated uploadDocument - removed localStorage userId logic
  const uploadDocument = useCallback((file: File) => {
    store.getState().setStatus("uploading");

    const formData = new FormData();
    formData.append("file", file);

    // ✅ Cookie handles authentication automatically
    createSessionMutation(
      { formData },
      {
        onSuccess: (data) => {
          store.getState().setSessionId(data.session_id); 
          connect(data.session_id);
        },
        onError: (error) => {
          console.error("Failed to create session:", error);
          store.getState().setStatus("idle");
        },
      }
    );
  }, [createSessionMutation, store]);

  const disconnect = useCallback(() => {
    wsClientRef.current?.close();
    wsClientRef.current = null;
    audioPlayerRef.current?.stopAndClear();
    audioPlayerRef.current = null;
    if (isRecording) {
      stopRecorder();
    }
    store.getState().reset();
  }, [isRecording, stopRecorder, store]);

  const connect = useCallback(
    (sessionId: string) => {
      if (wsClientRef.current) return;

      store.getState().setSessionId(sessionId);
      store.getState().setStatus("connecting");

      const wsUrl = import.meta.env.VITE_WS_URL || "ws://localhost:8000/ws";
      console.log("🔌 Connecting to WebSocket:", wsUrl, "Session ID:", sessionId);
      wsClientRef.current = new WsClient(wsUrl);
      audioPlayerRef.current = new AudioPlayer();

      wsClientRef.current.on("open", () => {
        wsClientRef.current?.sendInit(sessionId);
      });

      wsClientRef.current.on("initialized", () => {
        // Backend will send ReadingStarted right after this.
      });

      wsClientRef.current.on("readingStarted", () => {
        audioPlayerRef.current?.setAllowReadingPlayback(true);
        store.getState().setStatus("reading");
        store.getState().setInterruptible(true);
      });

      wsClientRef.current.on("readingPaused", () => {
        audioPlayerRef.current?.pause();
        audioPlayerRef.current?.setAllowReadingPlayback(false);
        store.getState().setStatus("paused");
        store.getState().setInterruptible(false);
      });

      wsClientRef.current.on("readingEnded", () => {
        console.log("📢 Backend finished sending reading audio");
        
        audioPlayerRef.current?.onQueueEmpty(() => {
          console.log("🎬 All reading audio has finished playing");
          store.getState().setStatus("ended");
          store.getState().setInterruptible(false);
        }, 'reading');
      });

      wsClientRef.current.on("serverError", (message) => {
        console.error("Server Error:", message);
      });

      wsClientRef.current.on("answeringStarted", () => {
        console.log("🎯 Answering started!");
        store.getState().setStatus("answering");
        store.getState().setInterruptible(false);
      });

      wsClientRef.current.on("answeringEnded", () => {
        console.log("📢 Answering ended"); 
        audioPlayerRef.current?.onQueueEmpty(() => {
          console.log("🔇 Audio queue empty"); 
          store.getState().setStatus("listening");
          store.getState().setInterruptible(true);
        });
      });

      wsClientRef.current.on("audio", (data) => {
        const { status } = store.getState();
        if (status === "reading") {
          console.log("Reading chunk added")
          audioPlayerRef.current?.addReadingChunk(data);
        } else if (status === "answering") {
          audioPlayerRef.current?.addAnsweringChunk(data);
        }
      });

      wsClientRef.current.on("close", () => disconnect());
      wsClientRef.current.on("error", (error) => {
        console.error("WebSocket error:", error);
        disconnect();
      });
      wsClientRef.current.connect();
    },
    [store, disconnect]
  );

  const startRecording = useCallback(() => {
    const { status, isInterruptible } = store.getState();
    if (
      (status === "reading" || status === "listening") &&
      isInterruptible
    ) {
      audioPlayerRef.current?.pause();
      audioPlayerRef.current?.setAllowReadingPlayback(false);
      wsClientRef.current?.sendInterruptStarted();
      store.getState().setStatus("listening");
      store.getState().setInterruptible(false);
      startRecorder();
    }
  }, [store, startRecorder]);

  const stopRecordingAndSend = useCallback(() => {
    stopRecorder();
    wsClientRef.current?.sendInterruptEnded();
    store.getState().setStatus("processing");
  }, [store, stopRecorder]);

  const pauseReading = useCallback(() => {
    wsClientRef.current?.sendPauseReading();
    audioPlayerRef.current?.pause();
    audioPlayerRef.current?.setAllowReadingPlayback(false);
    store.getState().setStatus("paused");
  }, []);

  const resumeReading = useCallback(() => {
    audioPlayerRef.current?.setAllowReadingPlayback(true); 
    store.getState().setStatus("reading");
    wsClientRef.current?.sendResumeReading();
  }, []);

  return (
    <SessionContext.Provider
      value={{
        store,
        uploadDocument,
        isUploading,
        connect,
        disconnect,
        pauseReading,
        resumeReading,
        isRecording,
        startRecording,
        stopRecordingAndSend,
      }}
    >
      {children}
    </SessionContext.Provider>
  );
}

export function useSession() {
  const context = useContext(SessionContext);
  if (!context) {
    throw new Error("useSession must be used within a SessionProvider");
  }

  const state = useStore(context.store);

  return {
    ...state,
    uploadDocument: context.uploadDocument,
    isUploading: context.isUploading,
    connect: context.connect,
    disconnect: context.disconnect,
    pauseReading: context.pauseReading,
    resumeReading: context.resumeReading,
    isRecording: context.isRecording,
    startRecording: context.startRecording,
    stopRecordingAndSend: context.stopRecordingAndSend,
  };
}