import { useEffect, useRef } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { useSession, SessionStatus } from "~/providers/session-provider";
import { useAuth } from "~/providers/auth-provider";
import { SessionListSidebar } from "~/components/session/session-list-sidebar";
import { NotesGrid } from "~/components/session/notes-grid";
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
  const { user } = useAuth();
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

  // Auth check
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

  // Request microphone permission
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

  if (!user) {
    return null;
  }

  if (status === "connecting" || status === "idle") {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background text-foreground">
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
  <div className="flex h-screen w-full bg-background">
    {/* Left Sidebar - 20% */}
    <div className="w-[20%] h-full">
      <SessionListSidebar />
    </div>

    {/* Center Content - 60% */}
    <div className="flex-1 flex flex-col">
      {/* Header */}
      <header className="flex items-center justify-between p-4 border-b">
        <div className="rounded-full bg-muted px-4 py-1 text-sm">
          Status: <span className="font-bold">{statusMap[status]}</span>
        </div>
        <button
          onClick={disconnect}
          className="rounded-full bg-destructive/80 p-2 hover:bg-destructive"
          title="End Session"
        >
          <X className="h-5 w-5" />
        </button>
      </header>

      {/* Main Content Area - Notes Grid */}
      <div className="flex-1 overflow-hidden">
        <NotesGrid />
      </div>

      {/* Audio Controls */}
      <footer className="flex items-center justify-center gap-6 p-8 border-t">
        <button
          onClick={() => {
            if (isPaused) {
              resumeReading();
            } else {
              pauseReading();
            }
          }}
          disabled={!canPauseOrResume}
          className="rounded-full bg-muted p-4 hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed"
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
            isRecording ? "bg-destructive scale-110" : "bg-primary"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          title="Hold to Speak"
        >
          <Mic className="h-10 w-10" />
        </button>
        
        <div className="w-20">{/* Placeholder */}</div>
      </footer>
    </div>

    {/* Right Sidebar - 20% - Status panel will go here */}
    <div className="w-[20%] h-full border-l bg-muted/20">
      <div className="p-4">
        <h3 className="font-semibold">Session Status</h3>
        <p className="text-sm text-muted-foreground mt-2">
          Status panel coming soon...
        </p>
      </div>
    </div>
  </div>
  );
}