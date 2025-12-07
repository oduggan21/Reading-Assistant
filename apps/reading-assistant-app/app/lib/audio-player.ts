// The sample rate must match the output from the backend's TTS service.
// OpenAI's default is 24000.
const SAMPLE_RATE = 24000;

export class AudioPlayer {
  private audioContext: AudioContext;
  private readingAudioQueue: AudioBuffer[] = [];
  private answeringAudioQueue: AudioBuffer[] = [];
  private isPlaying = false;
  private currentSource: AudioBufferSourceNode | null = null;
  private onQueueEmptyCallback: (() => void) | null = null;
  private waitingForQueue: 'reading' | 'answering' | null = null;
  private currentReadingSentenceIndex: number = 0;

  private allowReadingPlayback = true;

  constructor() {
    this.audioContext = new AudioContext({ sampleRate: SAMPLE_RATE });
  }

  public addReadingChunk(data: ArrayBuffer): void {
    this.processAndQueueChunk(data, 'reading');
  }

  public addAnsweringChunk(data: ArrayBuffer): void {
    this.processAndQueueChunk(data, 'answering');
  }

  public getCurrentSentenceIndex(): number {
    return this.currentReadingSentenceIndex;
  }

  public onQueueEmpty(callback: () => void, queueType: 'reading' | 'answering' = 'answering'): void {
  console.log("🎯 onQueueEmpty registered, isPlaying:", this.isPlaying, "answeringQueue:", this.answeringAudioQueue.length); // ✅ Debug
  this.onQueueEmptyCallback = callback;
  this.waitingForQueue = queueType;

  // ❌ Remove the immediate check - let playFromQueues handle it
  }
  public setAllowReadingPlayback(allow: boolean): void {
  this.allowReadingPlayback = allow;
  if (!allow && this.isPlaying) {
    // If we're currently playing reading audio, pause it
    this.pause();
  }
}

  private async processAndQueueChunk(
  data: ArrayBuffer,
  queueType: 'reading' | 'answering'
): Promise<void> {
  if (data.byteLength === 0) {
    console.log("Empty audio trigger received, starting playback");
    if (!this.isPlaying) {
      this.playFromQueues();
    }
    return;
  }
  try {
    console.log("we entered the process and queue")
    // ✅ Decode the MP3/audio data properly using Web Audio API
    const audioBuffer = await this.audioContext.decodeAudioData(data.slice(0));
    
    if (queueType === 'reading') {
      this.readingAudioQueue.push(audioBuffer);
    } else {
      this.answeringAudioQueue.push(audioBuffer);
    }

    console.log("isPlaying:", this.isPlaying);
    if (!this.isPlaying) {
      console.log("starting play from queue")
      this.playFromQueues();
    }
  } catch (error) {
    console.error("Error processing audio chunk:", error);
  }
}

private playFromQueues = (): void => {
    this.currentSource = null;
    
    // ✅ Prioritize answering queue, but only use reading queue if allowed
    const activeQueue =
      this.answeringAudioQueue.length > 0
        ? this.answeringAudioQueue
        : (this.allowReadingPlayback ? this.readingAudioQueue : []);

    if (activeQueue.length === 0) {
      console.log("🔇 Queue empty! Firing callback. isPlaying:", this.isPlaying);
      this.isPlaying = false;
       if (this.onQueueEmptyCallback) {
        const shouldFire = 
          (this.waitingForQueue === 'answering' && this.answeringAudioQueue.length === 0) ||
          (this.waitingForQueue === 'reading' && this.readingAudioQueue.length === 0);
        
        if (shouldFire) {
          console.log("🔔 Firing callback for", this.waitingForQueue, "queue");
          this.onQueueEmptyCallback();
          this.onQueueEmptyCallback = null;
          this.waitingForQueue = null;
        }
      }
    }

    this.isPlaying = true;
    const bufferToPlay = activeQueue.shift()!;
    const source = this.audioContext.createBufferSource();
    source.buffer = bufferToPlay;
    source.connect(this.audioContext.destination);
    source.onended = () => {
       this.currentReadingSentenceIndex++;
       this.playFromQueues();
    };
    source.start();
    this.currentSource = source;
  };

  /**
   * Immediately stops the current playback but PRESERVES the audio queues.
   * This is used for interruptions and pausing.
   */
  public pause(): void {
  if (this.currentSource && this.currentSource.buffer) {
    // Save the current buffer to replay it
    const currentBuffer = this.currentSource.buffer;
    
    // Determine which queue it came from and put it back at the front
    if (this.allowReadingPlayback && this.answeringAudioQueue.length === 0) {
      // Was from reading queue
      this.readingAudioQueue.unshift(currentBuffer);
    } else if (this.answeringAudioQueue.length > 0 || !this.allowReadingPlayback) {
      // Was from answering queue
      this.answeringAudioQueue.unshift(currentBuffer);
    }
    
    // Disconnect the onended event to prevent the next item from playing automatically.
    this.currentSource.onended = null;
    this.currentSource.stop();
    this.currentSource = null;
  }
  this.isPlaying = false;
}

  /**
   * Immediately stops playback and CLEARS all audio queues.
   * This is used when the session is completely disconnected or ended.
   */
  public stopAndClear(): void {
    this.pause(); // Stop any current playback first.
    this.readingAudioQueue = [];
    this.answeringAudioQueue = [];
  }
}
