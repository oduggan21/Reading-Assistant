import { useParams } from "react-router-dom";
import { useListNotesHandler, getListNotesHandlerQueryKey } from "@reading-assistant/query/handlers/handlers";
import { useQueryClient } from "@tanstack/react-query"; 
import { useSession } from "~/providers/session-provider";
import { ScrollArea } from "~/components/ui/scroll-area";
import { Card, CardContent } from "~/components/ui/card";
import { FileText, Loader2 } from "lucide-react";
import { formatDistanceToNow } from "date-fns";
import { useState, useEffect } from "react";

export function NotesGrid() {
  const { id: sessionId } = useParams();
  const [selectedNote, setSelectedNote] = useState<string | null>(null);
  const { status } = useSession();  
  const queryClient = useQueryClient(); 

  const refetchInterval = status === "answering" || status === "processing" ? 2000 : false;
  
  // Fetch notes for the current session
  const { data: notesData, isLoading, error } = useListNotesHandler(
    sessionId || "",  // ✅ Direct string parameter
    {
      query: {
        enabled: !!sessionId, // Only fetch if we have a session ID
        refetchInterval,
      },
    }
  );

  useEffect(() => {
    if (status === "reading" || status === "listening") {
      if (sessionId) {
        queryClient.invalidateQueries({ 
          queryKey: getListNotesHandlerQueryKey(sessionId) 
        });
      }
    }
  }, [status, sessionId, queryClient]);

  if (isLoading) {
    return (
      <div className="h-full w-full flex items-center justify-center">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="text-sm">Loading notes...</span>
        </div>
      </div>
    );
  }
  
  if (error) {
    console.error("Notes error:", error);
    return (
      <div className="h-full w-full flex items-center justify-center">
        <p className="text-sm text-destructive">Failed to load notes</p>
      </div>
    );
  }
  
  const notes = notesData?.notes || [];
  
  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b">
        <h3 className="font-semibold text-lg">Notes</h3>
        <p className="text-xs text-muted-foreground mt-1">
          {notes.length} {notes.length === 1 ? 'note' : 'notes'}
        </p>
      </div>
      
      {/* Notes Grid */}
      <ScrollArea className="flex-1 p-4">
        {notes.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center p-8">
            <FileText className="h-12 w-12 text-muted-foreground/50 mb-4" />
            <p className="text-sm text-muted-foreground">
              No notes yet. Ask questions during your reading session to generate notes!
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {notes.map((note) => {
              const createdAt = new Date(note.created_at);
              
              return (
                <Card
                  key={note.note_id}
                  className="cursor-pointer hover:border-primary transition-colors"
                  onClick={() => setSelectedNote(note.note_id)}
                >
                  <CardContent className="p-4">
                    {/* Note text - truncated */}
                    <p className="text-sm line-clamp-6 mb-3">
                      {note.text}
                    </p>
                    
                    {/* Timestamp */}
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <FileText className="h-3 w-3" />
                      <span>
                        {formatDistanceToNow(createdAt, { addSuffix: true })}
                      </span>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}