import { useState, useEffect, useCallback, ChangeEvent } from "react";
import { useNavigate } from "react-router-dom";
import { useSession } from "~/providers/session-provider";
import { UploadCloud, Loader2, FileText } from "lucide-react";

export default function HomePage() {
  const navigate = useNavigate();
  const { uploadDocument, isUploading, sessionId } = useSession();
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  // This effect listens for when a session ID becomes available after a
  // successful upload, and then navigates the user to the session page.
  useEffect(() => {
    console.log("📊 Session ID changed:", sessionId);
    if (sessionId) {
      console.log("🚀 Navigating to session:", sessionId); // ✅ Debug log
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
         console.log("📤 Starting upload...");
      uploadDocument(selectedFile);
    }
  }, [selectedFile, isUploading, uploadDocument]);

  return (
    <main className="flex min-h-screen items-center justify-center bg-gray-900 p-4 text-white">
      <div className="w-full max-w-lg rounded-xl bg-gray-800 shadow-2xl">
        <div className="p-8 text-center">
          <h1 className="text-3xl font-bold tracking-tight text-white">
            Interactive Audio Learner
          </h1>
          <p className="mt-2 text-gray-400">
            Upload a document to begin an interactive reading session. Listen,
            interrupt, and ask questions to deepen your understanding.
          </p>
        </div>

        <div className="border-t border-gray-700 p-8">
          <div className="flex h-48 w-full flex-col items-center justify-center rounded-lg border-2 border-dashed border-gray-600 bg-gray-900/50 text-center">
            {selectedFile ? (
              <div className="flex flex-col items-center text-gray-300">
                <FileText className="h-10 w-10" />
                <span className="mt-2 font-medium">{selectedFile.name}</span>
                <span className="text-xs text-gray-500">
                  ({(selectedFile.size / 1024).toFixed(2)} KB)
                </span>
              </div>
            ) : (
              <>
                <UploadCloud className="h-10 w-10 text-gray-500" />
                <label
                  htmlFor="file-upload"
                  className="mt-2 cursor-pointer font-medium text-indigo-400 hover:text-indigo-300"
                >
                  Choose a file
                  <input
                    id="file-upload"
                    name="file-upload"
                    type="file"
                    className="sr-only"
                    onChange={handleFileChange}
                    accept=".txt,.md,.text" // Accept common text file types
                  />
                </label>
                <p className="text-xs text-gray-500">TXT or MD up to 10MB</p>
              </>
            )}
          </div>

          <button
            onClick={handleUpload}
            disabled={!selectedFile || isUploading}
            className="mt-6 w-full rounded-md bg-indigo-600 px-4 py-3 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-gray-700 disabled:opacity-50"
          >
            {isUploading ? (
              <span className="flex items-center justify-center">
                <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                Uploading & Processing...
              </span>
            ) : (
              "Start Interactive Session"
            )}
          </button>
        </div>
      </div>
    </main>
  );
}