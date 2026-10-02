import { APP_NAME, APP_VERSION, GIT_HASH } from '../../constants';
import { searchExecutableFile } from '../../fs/fileAssoc';
import { normalizeFileName, toCanonicalFileKey } from '../../fs/fileName';
import { decodeFileAsText, fileSystem, listFilesSorted, removeFile, renameFile, upsertFile } from '../../fs/fileSystem';
import { env, getPathExt, launchNoDisplayTask, launchRustTaskWithCommand } from '../../task/taskRunner';
import type { WindowId } from '../../wm/state';
import { closeWindow } from '../../wm/windowManager';
import { pushHistory, splitCommandTokens } from './buffer';
import {
  appendTerminalLine,
  syncTerminalView,
  terminalHistoryByWindow,
  terminalHistoryIndexByWindow,
  terminalOutputByWindow,
  terminalPendingInputByWindow,
} from './state';

const formatBytes = (size: number): string => {
  return `${size.toLocaleString('ja-JP')}`;
};

export const runTerminalCommand = (windowId: WindowId, inputText: string, options?: { echoInput?: boolean; skipHistory?: boolean }): void => {
  const normalized = inputText.trim();
  if (!normalized) {
    return;
  }

  // 履歴に追加（skipHistory が false でない場合）
  if (!options?.skipHistory) {
    if (!terminalHistoryByWindow.has(windowId)) {
      terminalHistoryByWindow.set(windowId, []);
    }

    const history = terminalHistoryByWindow.get(windowId)!;

    pushHistory(history, normalized);
  }

  terminalHistoryIndexByWindow.set(windowId, -1);
  terminalPendingInputByWindow.delete(windowId);

  if (options?.echoInput !== false) {
    appendTerminalLine(windowId, `> ${inputText}`);
  }

  const tokens = splitCommandTokens(normalized);
  const command = tokens[0]?.toUpperCase() ?? '';
  const args = tokens.slice(1);
  const argsText = normalized.slice(tokens[0]?.length ?? 0).trimStart();

  switch (command) {
    case 'VER':
      appendTerminalLine(windowId, `${APP_NAME} v${APP_VERSION} (${GIT_HASH})`);
      return;
    case 'ECHO':
      appendTerminalLine(windowId, argsText);
      return;
    case 'CLS': {
      terminalOutputByWindow.set(windowId, []);
      syncTerminalView(windowId);
      return;
    }
    case 'DIR':
    case 'LS': {
      const entries = listFilesSorted();
      if (entries.length === 0) {
        appendTerminalLine(windowId, 'No files.');
        return;
      }

      let totalSize = 0;
      for (const entry of entries) {
        totalSize += entry.content.byteLength;
        appendTerminalLine(windowId, `${entry.name.padEnd(13, ' ')} ${formatBytes(entry.content.byteLength)}`);
      }
      appendTerminalLine(windowId, `  ${entries.length} file(s), total ${formatBytes(totalSize)} bytes`);
      return;
    }
    case 'TYPE': {
      const targetName = args[0] ?? '';
      if (!targetName) {
        appendTerminalLine(windowId, 'Usage: TYPE <filename>');
        return;
      }

      const normalizedResult = normalizeFileName(targetName);
      if (!normalizedResult.ok) {
        appendTerminalLine(windowId, normalizedResult.reason);
        return;
      }

      const normalizedName = normalizedResult.name;
      const entry = fileSystem.get(toCanonicalFileKey(normalizedName));
      if (!entry) {
        appendTerminalLine(windowId, `File not found: ${targetName}`);
        return;
      }

      const text = decodeFileAsText(entry);
      if (!text) {
        appendTerminalLine(windowId, '(empty file)');
        return;
      }

      for (const line of text.split(/\r?\n/)) {
        appendTerminalLine(windowId, line);
      }
      return;
    }
    case 'COPY': {
      const sourceRaw = args[0] ?? '';
      const destinationRaw = args[1] ?? '';
      if (!sourceRaw || !destinationRaw) {
        appendTerminalLine(windowId, 'Usage: COPY <source> <destination>');
        return;
      }

      const sourceNormalized = normalizeFileName(sourceRaw);
      if (!sourceNormalized.ok) {
        appendTerminalLine(windowId, sourceNormalized.reason);
        return;
      }

      const sourceName = sourceNormalized.name;
      const sourceEntry = fileSystem.get(toCanonicalFileKey(sourceName));
      if (!sourceEntry) {
        appendTerminalLine(windowId, `File not found: ${sourceRaw}`);
        return;
      }

      const result = upsertFile(destinationRaw, new Uint8Array(sourceEntry.content));
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `${sourceName} copied to ${result.name}`);
      return;
    }
    case 'DEL': {
      const targetRaw = args[0] ?? '';
      if (!targetRaw) {
        appendTerminalLine(windowId, `Usage: DEL <filename>`);
        return;
      }

      const result = removeFile(targetRaw);
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `Deleted ${result.name}`);
      return;
    }
    case 'REN': {
      const sourceRaw = args[0] ?? '';
      const destinationRaw = args[1] ?? '';
      if (!sourceRaw || !destinationRaw) {
        appendTerminalLine(windowId, `Usage: REN <source> <destination>`);
        return;
      }

      const result = renameFile(sourceRaw, destinationRaw);
      if (!result.ok) {
        appendTerminalLine(windowId, result.reason);
        return;
      }

      appendTerminalLine(windowId, `${result.source} renamed to ${result.destination}`);
      return;
    }
    case 'EXIT': {
      closeWindow(windowId);
      return;
    }
    case 'SET': {
      const setArg = argsText.trim();
      if (!setArg) {
        // Display all environment variables
        const entries = Object.entries(env).sort(([a], [b]) => a.localeCompare(b));
        if (entries.length === 0) {
          appendTerminalLine(windowId, '(no environment variables set)');
        } else {
          for (const [name, value] of entries) {
            appendTerminalLine(windowId, `${name}=${value}`);
          }
        }
        return;
      }

      const eqIndex = setArg.indexOf('=');
      if (eqIndex === -1) {
        // Display current environment variable value
        const varName = setArg.trim();
        const value = env[varName];
        if (value !== undefined) {
          appendTerminalLine(windowId, `${varName}=${value}`);
        } else {
          appendTerminalLine(windowId, `${varName} is not set`);
        }
        return;
      }

      // Set environment variable
      const varName = setArg.slice(0, eqIndex).trim();
      const value = setArg.slice(eqIndex + 1).trim();

      if (!varName) {
        appendTerminalLine(windowId, 'Variable name cannot be empty');
        return;
      }

      env[varName] = value;
      appendTerminalLine(windowId, `${varName}=${value}`);
      return;
    }
    case 'HELP': {
      // Display help for selected commands (cognitive load mitigation)
      appendTerminalLine(windowId, 'Available Commands:');
      appendTerminalLine(windowId, '');
      appendTerminalLine(windowId, 'VER              - Display app version and git hash');
      appendTerminalLine(windowId, 'DIR              - List files with size information');
      appendTerminalLine(windowId, 'TYPE <filename>  - Display file contents as text');
      appendTerminalLine(windowId, 'NCST <file>      - Execute file with no output display');
      return;
    }
    case 'START':
    case 'NCST':
    case 'OPEN': {
      // Launch Rust task with dummy terminal (no output display)
      // Errors are only logged to console.error
      launchNoDisplayTask(normalized, tokens);
      return;
    }
    default: {
      const fileName = searchExecutableFile(tokens[0] ?? '', getPathExt());
      if (fileName) {
        launchRustTaskWithCommand(windowId, fileName, normalized);
        return;
      }

      appendTerminalLine(windowId, 'Bad command or file name');
      return;
    }
  }
};
