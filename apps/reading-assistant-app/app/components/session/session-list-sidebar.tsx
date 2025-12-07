import { useNavigate, useParams } from "react-router-dom";
import { useListSessionsHandler } from "@reading-assistant/query/handlers/handlers";
import { useSession } from "~/providers/session-provider";  
import { ScrollArea } from "~/components/ui/scroll-area";
import { Button } from "~/components/ui/button";
import { FileText, Clock } from "lucide-react";
import { formatDistanceToNow } from "date-fns";

export function SessionListSidebar() {
  const navigate = useNavigate();
  const { id: currentSessionId } = useParams();
  const { disconnect, connect } = useSession(); 
  
  // Fetch all sessions for the authenticated user
  const { data: sessionsData, isLoading, error } = useListSessionsHandler();
  
  const handleSessionClick = (sessionId: string) => {
    // ✅ Don't switch if clicking the current session
    if (sessionId === currentSessionId) {
      return;
    }
    disconnect();
  
    
    // ✅ Navigate to new session
    navigate(`/sessions/${sessionId}`);
    
    // ✅ Connect will happen in session.tsx via useEffect when sessionId changes
  };
  
  if (isLoading) {
    return (
      <div className="h-full w-full flex items-center justify-center border-r bg-muted/20">
        <p className="text-sm text-muted-foreground">Loading sessions...</p>
      </div>
    );
  }
  
  if (error) {
    return (
      <div className="h-full w-full flex items-center justify-center border-r bg-muted/20">
        <p className="text-sm text-destructive">Failed to load sessions</p>
      </div>
    );
  }
  
  const sessions = sessionsData?.sessions || [];
  
  return (
    <div className="h-full w-full border-r bg-muted/20 flex flex-col">
      {/* Header */}
      <div className="p-4 border-b">
        <h2 className="font-semibold text-lg">Sessions</h2>
        <p className="text-xs text-muted-foreground mt-1">
          {sessions.length} {sessions.length === 1 ? 'session' : 'sessions'}
        </p>
      </div>
      
      {/* Session List */}
      <ScrollArea className="flex-1">
        <div className="p-2 space-y-2">
          {sessions.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              No sessions yet. Upload a document to get started!
            </div>
          ) : (
            sessions.map((session) => {
              const isActive = session.session_id === currentSessionId;
              const createdAt = new Date(session.created_at);
              
              return (
                <Button
                  key={session.session_id}
                  variant={isActive ? "secondary" : "ghost"}
                  className={`w-full justify-start h-auto p-3 ${
                    isActive ? "bg-secondary" : ""
                  }`}
                  onClick={() => handleSessionClick(session.session_id)}
                >
                  <div className="flex items-start gap-3 w-full">
                    {/* Icon */}
                    <FileText className="h-5 w-5 mt-0.5 flex-shrink-0" />
                    
                    {/* Content */}
                    <div className="flex-1 min-w-0 text-left">
                      <div className="font-medium text-sm truncate">
                        {session.title || `Session ${session.session_id.slice(0, 8)}`}
                      </div>
                      <div className="flex items-center gap-1 mt-1 text-xs text-muted-foreground">
                        <Clock className="h-3 w-3" />
                        <span>
                          {formatDistanceToNow(createdAt, { addSuffix: true })}
                        </span>
                      </div>
                    </div>
                  </div>
                </Button>
              );
            })
          )}
        </div>
      </ScrollArea>
    </div>
  );
}