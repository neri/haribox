import iconAppWindow from '../assets/icons/app-window.svg?raw';
import iconFileText from '../assets/icons/file-text.svg?raw';
import iconFile from '../assets/icons/file.svg?raw';
import iconPhoto from '../assets/icons/photo.svg?raw';
import iconFileMusic from '../assets/icons/file-music.svg?raw';
import { normalizeFileName, toCanonicalFileKey } from './fileName';
import { fileSystem } from './fileSystem';

// File icon SVG data (src/assets/icons/)
const FILE_ICONS = {
  file: iconFile,
  fileText: iconFileText,
  appWindow: iconAppWindow,
  photo: iconPhoto,
  music: iconFileMusic,
};

/** Lower-cased extension including the dot, or '' when the name has none */
export const getFileExtension = (filename: string): string => {
  return filename.includes('.') ? '.' + filename.split('.').pop()!.toLowerCase() : '';
};

/** Lower-cased executable extensions from a colon separated PATH_EXT value */
export const parsePathExt = (pathExt: string): string[] => {
  return pathExt.split(':').filter(Boolean).map(e => e.toLowerCase());
};

export const getFileIcon = (filename: string, pathExt: string): string => {
  const ext = getFileExtension(filename);
  const extensions = parsePathExt(pathExt);

  // Check if it's an executable file
  if (extensions.includes(ext)) {
    return FILE_ICONS.appWindow;
  }

  // Check if it's a text file
  if (ext === '.txt') {
    return FILE_ICONS.fileText;
  }

  // Check if it's an image file
  if (ext === '.bmp' || ext === '.jpg') {
    return FILE_ICONS.photo;
  }

  // Check if it's a music file
  if (ext === '.mml') {
    return FILE_ICONS.music;
  }

  // Default to generic file icon
  return FILE_ICONS.file;
};

/**
 * Resolves a command name to a file: the exact name first, then with each PATH_EXT extension.
 * @returns the stored file name, or null
 */
export const searchExecutableFile = (fileName: string, pathExt: string): string | null => {
  const normalizedFile = normalizeFileName(fileName);
  if (normalizedFile.ok) {
    const entry = fileSystem.get(toCanonicalFileKey(normalizedFile.name));
    if (entry) {
      return entry.name;
    }
  }

  // Try with PATH_EXT extensions
  const extensions = pathExt.split(':').filter(Boolean);
  for (const ext of extensions) {
    const fileNameWithExt = fileName + ext;
    const normalizedWithExt = normalizeFileName(fileNameWithExt);
    if (normalizedWithExt.ok) {
      const entry = fileSystem.get(toCanonicalFileKey(normalizedWithExt.name));
      if (entry) {
        return entry.name;
      }
    }
  }

  return null;
};

// ---- Selection in a file list

/** The selected row of a file list. `key` is the canonical file key, null before anything was selected. */
export type FileSelection = {
  index: number;
  key: string | null;
};

/**
 * Finds the row to select after the list changed.
 * Follows the selected file when rows were added or removed around it;
 * if the file is gone, keeps the same row position (clamped to the list).
 */
export const resolveSelectedIndex = (fileKeys: string[], selection: FileSelection): number => {
  const followed = selection.key === null ? -1 : fileKeys.indexOf(selection.key);
  if (followed >= 0) {
    return followed;
  }

  return Math.max(0, Math.min(selection.index, fileKeys.length - 1));
};
