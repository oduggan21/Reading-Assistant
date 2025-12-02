import { useEffect, useRef } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSession, SessionStatus } from "~/providers/session-provider";
import { useAuth } from "~/providers/auth-provider";  // ✅ Add this
import { Loader2, Mic, Pause, Play, X } from "lucide-react";

const statusMap: Record<SessionStatus, string> = {
  idle: "Idle",
  uploading: "Uploading...",
  connecting: "Connecting to session...",
  reading: "Reading document...",
  listening: "Listening...",
  processing: "Processing...",
  answering: "Answering your question...",
  paused: "Paused",
  ended: "Session ended.",
};

export default function SessionPage() {
  const { id: sessionId } = useParams();
  const navigate = useNavigate();
  const { user } = useAuth();  // ✅ Add this
  const {
    status,
    connect,
    disconnect,
    isRecording,
    startRecording,
    stopRecordingAndSend,
    pauseReading,
    resumeReading,
  } = useSession();
  
  const hasConnected = useRef(false);

  // ✅ Add auth check
  useEffect(() => {
    if (!user) {
      navigate("/login");
    }
  }, [user, navigate]);

  useEffect(() => {
    if (!sessionId) {
      navigate("/");
      return;
    }
    
    if (!hasConnected.current) {
      hasConnected.current = true;
      connect(sessionId);
    }
    
    return () => {};
  }, [sessionId, connect, navigate]);

  useEffect(() => {
    const requestMicPermission = async () => {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        stream.getTracks().forEach(track => track.stop());
        console.log("✅ Microphone permission granted");
      } catch (error) {
        console.error("❌ Microphone permission denied:", error);
      }
    };

    requestMicPermission();
  }, []);

  const handleMicPress = () => {
    if ((status === "reading" || status === "listening") && !isRecording) {
      startRecording();
    }
  };

  const handleMicRelease = () => {
    if (isRecording) {
      stopRecordingAndSend();
    }
  };

  // ✅ Don't render if not authenticated
  if (!user) {
    return null;
  }

  if (status === "connecting" || status === "idle") {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-gray-900 text-white">
        <div className="flex items-center gap-3 text-lg">
          <Loader2 className="h-6 w-6 animate-spin" />
          <span>{statusMap[status]}</span>
        </div>
      </div>
    );
  }

  const isPaused = status === "paused";
  const canPauseOrResume = status === "reading" || status === "paused";
  const canInterrupt = (status === "reading" || status === "listening") && !isPaused;

  return (
    <div className="flex h-screen w-full flex-col items-center justify-between bg-gray-900 p-8 text-white">
      <header className="flex w-full items-center justify-between">
        <div className="rounded-full bg-white/10 px-4 py-1 text-sm">
          Status: <span className="font-bold">{statusMap[status]}</span>
        </div>
        <button
          onClick={disconnect}
          className="rounded-full bg-red-600/80 p-2 hover:bg-red-500"
          title="End Session"
        >
          <X className="h-5 w-5" />
        </button>
      </header>

      <main className="flex flex-1 items-center justify-center">
        <div className={`flex h-64 w-64 items-center justify-center rounded-full bg-white/5 transition-all duration-300 ${isRecording ? 'scale-110' : ''}`}>
          <div className={`h-48 w-48 rounded-full bg-white/10 transition-all duration-300 ${isRecording ? 'bg-blue-500/30' : ''}`} />
        </div>
      </main>

      <footer className="flex items-center gap-6">
        <button
          onClick={() => {
            if (isPaused) {
              resumeReading();
            } else {
              pauseReading();
            }
          }}
          disabled={!canPauseOrResume}
          className="rounded-full bg-white/20 p-4 hover:bg-white/30 disabled:opacity-50 disabled:cursor-not-allowed"
          title={isPaused ? "Resume Reading" : "Pause Reading"}
        >
          {isPaused ? <Play className="h-8 w-8" /> : <Pause className="h-8 w-8" />}
        </button>

        <button
          onMouseDown={handleMicPress}
          onMouseUp={handleMicRelease}
          onMouseLeave={handleMicRelease}
          onTouchStart={handleMicPress}
          onTouchEnd={handleMicRelease}
          disabled={!canInterrupt && !isRecording}
          className={`rounded-full p-8 transition-colors select-none ${
            isRecording ? "bg-red-600 scale-110" : "bg-blue-600"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          title="Hold to Speak"
        >
          {isRecording ? <Mic className="h-10 w-10" /> : <Mic className="h-10 w-10" />}
        </button>
        
        <div className="w-20">{/* Placeholder */}</div>
      </footer>
    </div>
  );
}