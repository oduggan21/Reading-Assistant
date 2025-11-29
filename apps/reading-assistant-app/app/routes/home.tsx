// app/routes/home.tsx
import { useState, useEffect, useCallback, ChangeEvent } from "react";
import { useNavigate } from "react-router-dom";
import { useSession } from "~/providers/session-provider";
import { UploadCloud, Loader2, FileText, BookHeadphones } from "lucide-react";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";

export default function HomePage() {
  const navigate = useNavigate();
  const { uploadDocument, isUploading, sessionId } = useSession();
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  useEffect(() => {
    if (sessionId) {
      navigate(`/sessions/${sessionId}`);
    }
  }, [sessionId, navigate]);

  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    if (event.target.files && event.target.files[0]) {
      setSelectedFile(event.target.files[0]);
    }
  };

  const handleUpload = useCallback(() => {
    if (selectedFile && !isUploading) {
      uploadDocument(selectedFile);
    }
  }, [selectedFile, isUploading, uploadDocument]);

  return (
    <main className="min-h-[100svh] flex items-center justify-center px-4 py-10 bg-background relative">
      {/* Theme toggles - top right (add later) */}
      <div className="absolute top-1.5 right-1.5 flex flex-row gap-x-6">
        {/* ThemeToggle and ThemeSelector will go here */}
      </div>

      <div className="w-full max-w-2xl">
        {/* Header section */}
        <div className="mb-8 text-center">
          {/* Badge */}
          <div className="inline-flex items-center gap-2 rounded-full border bg-card px-3 py-1.5 text-sm text-muted-foreground">
            <BookHeadphones className="h-4 w-4 text-primary" />
            <span>New</span>
            <span className="hidden sm:inline">voice-first AI reading assistant</span>
          </div>

          {/* Title */}
          <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-5xl">
            Sage AI
          </h1>

          {/* Subtitle */}
          <p className="mx-auto mt-4 max-w-xl text-muted-foreground">
            Interactive audio learning. Upload a document, listen on the go, interrupt with questions anytime.
          </p>
        </div>

        {/* Main card */}
        <Card className="shadow-lg">
          <CardHeader className="space-y-2">
            <div className="mx-auto inline-flex h-12 w-12 items-center justify-center rounded-full bg-secondary text-primary">
              <FileText className="h-6 w-6" />
            </div>
            <CardTitle className="text-center text-2xl">Start reading</CardTitle>
            <CardDescription className="text-center">
              Upload a document and begin your AI-powered reading session.
            </CardDescription>
          </CardHeader>

          <CardContent className="space-y-4">
            {/* File upload area */}
            <div className="flex h-48 w-full flex-col items-center justify-center rounded-lg border-2 border-dashed border-muted-foreground/25 bg-muted/50 text-center transition-colors hover:border-muted-foreground/50">
              {selectedFile ? (
                <div className="flex flex-col items-center">
                  <FileText className="h-10 w-10 text-primary" />
                  <span className="mt-2 font-medium">{selectedFile.name}</span>
                  <span className="mt-1 text-sm text-muted-foreground">
                    {(selectedFile.size / 1024).toFixed(2)} KB
                  </span>
                </div>
              ) : (
                <>
                  <UploadCloud className="h-10 w-10 text-muted-foreground" />
                  <label
                    htmlFor="file-upload"
                    className="mt-2 cursor-pointer font-medium text-primary hover:underline"
                  >
                    Choose a file
                    <input
                      id="file-upload"
                      name="file-upload"
                      type="file"
                      className="sr-only"
                      onChange={handleFileChange}
                      accept=".txt,.md,.pdf"
                    />
                  </label>
                  <p className="mt-1 text-sm text-muted-foreground">
                    TXT, MD, or PDF up to 10MB
                  </p>
                </>
              )}
            </div>

            {/* Buttons */}
            <div className="flex flex-col gap-3">
              <Button
                onClick={handleUpload}
                disabled={!selectedFile || isUploading}
                className="w-full"
              >
                {isUploading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Uploading & Processing...
                  </>
                ) : (
                  "Start Reading Session"
                )}
              </Button>

              <Button
                type="button"
                variant="outline"
                className="w-full"
                onClick={() => navigate("/session/recent")}
              >
                Explore Dashboard
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </main>
  );
}