import {
  createContext,
  useContext,
  useRef,
  type ReactNode,
  useCallback,
  useEffect,
  useState,
} from "react";
import { createStore, useStore } from "zustand";
import { v4 as uuidv4 } from "uuid";
import { useMutation } from "@tanstack/react-query";
import { createSessionHandler } from "@reading-assistant/query/handlers/handlers";
import type { CreateSessionResponse } from "@reading-assistant/query/api.schemas";
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
  userId: string | null;
}

// Define the actions that can modify the state
interface SessionActions {
  setStatus: (status: SessionStatus) => void;
  setInterruptible: (isInterruptible: boolean) => void;
  setSessionId: (sessionId: string | null) => void;
  setUserId: (userId: string | null) => void;
  reset: () => void;
}

// Create a Zustand store to hold our state and actions
const createSessionStore = () =>
  createStore<SessionState & SessionActions>((set) => ({
    status: "idle",
    isInterruptible: false,
    sessionId: null,
    userId: null,
    setStatus: (status) => set({ status }),
    setInterruptible: (isInterruptible) => set({ isInterruptible }),
    setSessionId: (sessionId) => set({ sessionId }),
    setUserId: (userId) => set({ userId }),
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
  
  // ✅ Track if we're on the client to safely use localStorage
  const [isClient, setIsClient] = useState(false);

  const wsClientRef = useRef<WsClient | null>(null);
  const audioPlayerRef = useRef<AudioPlayer | null>(null);

  // ✅ Set isClient to true after mount (client-side only)
  useEffect(() => {
    setIsClient(true);
  }, []);

  const { mutate: createSessionMutation, isPending: isUploading } = useMutation({
    mutationFn: ({
      formData,
      userId,
    }: {
      formData: FormData;
      userId: string;
    }) => {
      return createSessionHandler({
        headers: { "x-user-id": userId, "Content-Type": "multipart/form-data" },
        data: formData,
      });
    },
  });

  const { isRecording, start: startRecorder, stop: stopRecorder } =
    useVoiceRecorder((audioChunk: ArrayBuffer) => {
      wsClientRef.current?.sendAudio(audioChunk);
    });

  const uploadDocument = useCallback((file: File) => {
    // ✅ Only access localStorage on the client
    if (typeof window === 'undefined') return;
    
    let userId = localStorage.getItem("reading-assistant-user-id");
    if (!userId) {
      userId = uuidv4();
      localStorage.setItem("reading-assistant-user-id", userId);
    }
    store.getState().setUserId(userId);
    store.getState().setStatus("uploading");

    const formData = new FormData();
    formData.append("file", file);

    createSessionMutation(
      { formData, userId },
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
      console.log("🔌 Connecting to WebSocket:", wsUrl, "Session ID:", sessionId); // ✅ Add t
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
        store.getState().setStatus("ended");
        store.getState().setInterruptible(false);
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
    audioPlayerRef.current?.setAllowReadingPlayback(false); // ✅ Disable reading playback
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