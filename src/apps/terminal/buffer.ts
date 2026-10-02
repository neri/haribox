// Pure helpers for the terminal's output lines and command history

/** Appends text to the line buffer in place. The first segment continues the last line. */
export const appendTextToLines = (current: string[], text: string, maxLines: number): void => {
  // Split text by newlines
  const parts = text.split('\n');

  for (let i = 0; i < parts.length; i++) {
    if (i === 0) {
      // First part: append to the last line (or create new if empty)
      if (current.length === 0) {
        current.push(parts[i]);
      } else {
        current[current.length - 1] += parts[i];
      }
    } else {
      // Subsequent parts: each becomes a new line
      current.push(parts[i]);
    }
  }

  // Remove old lines if we exceed the maximum
  if (current.length > maxLines) {
    current.splice(0, current.length - maxLines);
  }
};

/** Puts the command at the front of the history (newest first), removing an older duplicate. */
export const pushHistory = (history: string[], normalized: string): void => {
  // 重複排除：同じコマンドが履歴にあれば古い方を削除
  const existingIndex = history.indexOf(normalized);
  if (existingIndex !== -1) {
    history.splice(existingIndex, 1);
  }

  history.unshift(normalized);
};

export const splitCommandTokens = (inputText: string): string[] => {
  return inputText.trim().split(/\s+/).filter(Boolean);
};
