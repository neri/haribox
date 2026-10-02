const state: {
  audioContext?: AudioContext;
  globalGain?: GainNode;
  oscillators: Map<string, {
    osc: OscillatorNode;
    gain: GainNode;
    startTime: number;
    workerTimestamp: number;
  }>;
  globalVolume: number;
  audioContextState: 'suspended' | 'running' | 'closed';
} = {
  oscillators: new Map(),
  globalVolume: 0.5,
  audioContextState: 'suspended',
};

let volumeChangeListener: (() => void) | null = null;

// Called when the volume or the AudioContext state changes, so the UI can refresh its icon
export const setVolumeChangeListener = (listener: () => void): void => {
  volumeChangeListener = listener;
};

export const getGlobalVolume = (): number => {
  return state.globalVolume;
};

// Audio system initialization and management
const VOLUME_STORAGE_KEY = 'haribote.audio.volume';

const initializeAudioContext = (): void => {
  if (state.audioContext) {
    return;
  }

  try {
    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
    state.audioContext = audioContext;
    const gainNode = audioContext.createGain();
    gainNode.connect(audioContext.destination);
    gainNode.gain.value = state.globalVolume;
    state.globalGain = gainNode;
    state.audioContextState = audioContext.state as any;
    console.log(`[Audio] AudioContext initialized with state: ${audioContext.state}`);
  } catch (error) {
    console.error('[Audio] Failed to initialize AudioContext:', error);
  }
};

export const resumeAudioContext = async (): Promise<void> => {
  if (!state.audioContext) {
    initializeAudioContext();
  }

  if (!state.audioContext || state.audioContext.state === 'running') {
    return;
  }

  try {
    await state.audioContext.resume();
    state.audioContextState = 'running';
    volumeChangeListener?.();
    console.log('[Audio] AudioContext resumed');
  } catch (error) {
    console.error('[Audio] Failed to resume AudioContext:', error);
  }
};

export const stopOscillator = (workerId: string): void => {
  const oscillatorData = state.oscillators.get(workerId);
  if (!oscillatorData) {
    return;
  }

  try {
    oscillatorData.osc.stop();
    oscillatorData.gain.disconnect();
  } catch (error) {
    console.warn(`[Audio] Error stopping oscillator for worker ${workerId}:`, error);
  }

  state.oscillators.delete(workerId);
};

export const playSound = (workerId: string, frequency: number, timestamp: number): void => {
  if (!state.audioContext) {
    initializeAudioContext();
  }

  if (!state.audioContext || !state.globalGain) {
    return;
  }

  // Resume AudioContext if suspended
  if (state.audioContext.state === 'suspended') {
    resumeAudioContext();
  }

  // Stop existing oscillator for this worker
  stopOscillator(workerId);

  // If frequency is 0, just stop (already done above)
  if (frequency <= 0) {
    return;
  }

  try {
    const osc = state.audioContext.createOscillator();
    const gain = state.audioContext.createGain();

    osc.frequency.value = frequency / 1000;
    osc.type = 'square';
    osc.connect(gain);
    gain.connect(state.globalGain);
    gain.gain.value = 0.15; // Reduced from 0.3 to lower square wave volume

    osc.start(state.audioContext.currentTime);
    state.oscillators.set(workerId, {
      osc,
      gain,
      startTime: state.audioContext.currentTime,
      workerTimestamp: timestamp,
    });

    // console.log(`[Audio] Started oscillator for worker ${workerId} at ${frequency}Hz (worker timestamp: ${timestamp}ms)`);
  } catch (error) {
    console.error(`[Audio] Error playing sound for worker ${workerId}:`, error);
  }
};

export const setGlobalVolume = (volume: number): void => {
  const clampedVolume = Math.max(0, Math.min(1, volume));
  state.globalVolume = clampedVolume;

  if (state.globalGain && state.audioContext) {
    state.globalGain.gain.value = clampedVolume;
  }

  // Save to localStorage
  localStorage.setItem(VOLUME_STORAGE_KEY, String(clampedVolume));
  volumeChangeListener?.();
};

// Load saved volume from localStorage
export const loadSavedVolume = (): void => {
  const savedVolume = localStorage.getItem(VOLUME_STORAGE_KEY);
  if (savedVolume) {
    try {
      const volume = parseFloat(savedVolume);
      if (!isNaN(volume) && volume >= 0 && volume <= 1) {
        state.globalVolume = volume;
      }
    } catch {
      // Ignore parsing errors, use default
    }
  }
};
