// This file defines the low-level client for interacting with the backend WebSocket.
// It uses an event-emitter pattern to decouple the raw WebSocket events from the application logic.

//=========================================================================================
// WebSocket Protocol Types
// Mirror the enums defined in the Rust `protocol.rs` file.
//=========================================================================================

// Messages sent FROM the Client (browser) TO the Server
type ClientToServerMessage =
  | { type: "init"; session_id: string }
  | { type: "interrupt_started" }
  | { type: "interrupt_ended" }
  | { type: "pause_reading" }
  | { type: "resume_reading" }
  | { type: "update_progress"; session_id: string, sentence_index: number };  // ✅ Add this

// Messages sent FROM the Server TO the Client (browser)
type ServerToClientMessage =
  | { type: "session_initialized"; session_id: string }
  | { type: "error"; message: string }
  | { type: "reading_started" }
  | { type: "reading_paused" }
  | { type: "reading_ended" }
  | { type: "answering_started" }
  | { type: "answering_ended" };

//=========================================================================================
// Client-Side Event Definitions
// These are the clean events our React components will listen for.
//=========================================================================================

interface WsClientEvents {
  open: () => void;
  close: () => void;
  error: (error: Event) => void;
  initialized: () => void;
  readingStarted: () => void;
  readingPaused: () => void;
  readingEnded: () => void;
  answeringStarted: () => void;
  answeringEnded: () => void;
  audio: (data: ArrayBuffer) => void;
  serverError: (message: string) => void;
}

//=========================================================================================
// The WsClient Class
//=========================================================================================

export class WsClient {
  private ws: WebSocket | null = null;
  private listeners: {
    [K in keyof WsClientEvents]?: Array<WsClientEvents[K]>;
  } = {};

  constructor(private url: string) {}

  // --- Event Emitter Implementation ---

  public on<K extends keyof WsClientEvents>(
    event: K,
    listener: WsClientEvents[K]
  ): void {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event]?.push(listener);
  }

  private emit<K extends keyof WsClientEvents>(
    event: K,
    ...args: Parameters<WsClientEvents[K]>
  ): void {
    this.listeners[event]?.forEach((listener) =>
      (listener as Function)(...args)
    );
  }

  // --- Connection Management ---

  public connect(): void {
    if (this.ws) {
      console.warn("WsClient is already connected or connecting.");
      return;
    }

    console.log(`WsClient: Attempting to connect to ${this.url}`);
    this.ws = new WebSocket(this.url);
    this.ws.binaryType = "arraybuffer"; // Important for receiving raw audio

    this.ws.onopen = () => {
      console.log("WsClient: WebSocket connection opened successfully.");
      this.emit("open");
    };

    this.ws.onmessage = (event: MessageEvent) => {
      if (typeof event.data === "string") {
        try {
          const message: ServerToClientMessage = JSON.parse(event.data);
          this.handleServerMessage(message);
        } catch (error) {
          console.error("WsClient: Failed to parse server message.", error);
        }
      } else if (event.data instanceof ArrayBuffer) {
        this.emit("audio", event.data);
      }
    };

    this.ws.onclose = () => {
      console.log("WsClient: WebSocket connection closed.");
      this.ws = null;
      this.emit("close");
    };

    this.ws.onerror = (error: Event) => {
      console.error("WsClient: WebSocket error.", error);
      this.emit("error", error);
    };
  }

  public close(): void {
    if (this.ws) {
      console.log("🛑 WsClient: Sending close frame to backend");
      this.ws.close();
      console.log("✅ WsClient: Close frame sent");
    }
  }

  // --- Message Handling ---

  private handleServerMessage(message: ServerToClientMessage): void {
    console.log("WsClient: Received message from server:", message.type);
    switch (message.type) {
      case "session_initialized":
        this.emit("initialized");
        break;
      case "reading_started":
        this.emit("readingStarted");
        break;
      case "reading_paused":
        this.emit("readingPaused");
        break;
      case "reading_ended":
        this.emit("readingEnded");
        break;
      case "answering_started":
        this.emit("answeringStarted");
        break;
      case "answering_ended":
        this.emit("answeringEnded");
        break;
      case "error":
        this.emit("serverError", message.message);
        break;
    }
  }

  private sendMessageToServer(message: ClientToServerMessage): void {
    if (this.ws?.readyState !== WebSocket.OPEN) {
      console.error("WsClient: Cannot send message, WebSocket is not open.");
      return;
    }
    console.log("WsClient: Sending message to server:", message);
    this.ws.send(JSON.stringify(message));
  }

  // --- Public Methods for Sending Data ---

  public sendInit(sessionId: string): void {
    this.sendMessageToServer({ type: "init", session_id: sessionId });
  }

  public sendInterruptStarted(): void {
    this.sendMessageToServer({ type: "interrupt_started" });
  }

  public sendInterruptEnded(): void {
    this.sendMessageToServer({ type: "interrupt_ended" });
  }

  public sendPauseReading(): void {
    this.sendMessageToServer({ type: "pause_reading" });
  }

  public sendResumeReading(): void {
    this.sendMessageToServer({ type: "resume_reading" });
  }

  // ✅ New method to send playback progress
  public sendUpdateProgress(sessionId: string, sentenceIndex: number): void {
    this.sendMessageToServer({ 
      type: "update_progress", 
      session_id: sessionId,
      sentence_index: sentenceIndex 
    });
  }

  public sendAudio(chunk: ArrayBuffer): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(chunk);
    }
  }
}

