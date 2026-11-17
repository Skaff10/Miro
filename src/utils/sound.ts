import type { NotificationEventType } from "../types/monitor";

export function playNotificationSound(eventType: NotificationEventType): void {
  if (typeof window === "undefined") {
    return;
  }
  const AudioCtx =
    window.AudioContext ||
    (window as unknown as { webkitAudioContext?: typeof AudioContext })
      .webkitAudioContext;
  if (!AudioCtx) {
    return;
  }

  try {
    const ctx = new AudioCtx();
    const now = ctx.currentTime;

    const playTone = (freq: number, start: number, duration: number, gainVal: number) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, start);
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(gainVal, start + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(start);
      osc.stop(start + duration + 0.02);
    };

    if (eventType === "completed") {
      playTone(587.33, now, 0.14, 0.045); // D5
      playTone(880.0, now + 0.09, 0.18, 0.045); // A5
    } else if (eventType === "attention") {
      playTone(659.25, now, 0.14, 0.055); // E5
      playTone(659.25, now + 0.16, 0.18, 0.055);
    } else if (eventType === "failed") {
      playTone(392.0, now, 0.16, 0.055); // G4
      playTone(311.13, now + 0.13, 0.22, 0.055); // Eb4
    } else {
      playTone(523.25, now, 0.12, 0.035); // C5
    }

    window.setTimeout(() => {
      void ctx.close();
    }, 600);
  } catch {
    // Ignore audio context errors when browser blocks autoplay.
  }
}
