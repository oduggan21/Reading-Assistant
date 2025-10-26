class AudioProcessor extends AudioWorkletProcessor {
  process(inputs) {
    // We only care about the first input, and the first channel of that input.
    const input = inputs[0];
    if (!input || input.length === 0) {
      return true; // Keep processor alive
    }

    const channelData = input[0];
    if (!channelData) {
      return true;
    }

    // Convert Float32Array (from -1.0 to 1.0) to Int16Array (-32768 to 32767).
    const pcm16 = new Int16Array(channelData.length);
    for (let i = 0; i < channelData.length; i++) {
      const s = Math.max(-1, Math.min(1, channelData[i]));
      pcm16[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
    }

    // Post the raw ArrayBuffer back to the main thread.
    // The second argument is a list of transferable objects, making this very efficient.
    this.port.postMessage(pcm16.buffer, [pcm16.buffer]);

    // Return true to indicate the processor should not be terminated.
    return true;
  }
}

registerProcessor("audio-processor", AudioProcessor);
